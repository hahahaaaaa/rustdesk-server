use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::time::Duration;

use crate::auth::{self, AuthUser};
use crate::db;
use crate::error::AppError;
use crate::models::{now_secs, UserRow};
use crate::rbac;
use crate::AppState;

const SESSION_TTL_SECS: i64 = 600;
const PENDING_ERROR: &str = "No authed oidc is found";

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct OidcAuthRequest {
    #[serde(default)]
    pub op: String,
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub uuid: String,
    #[serde(default)]
    pub device_info: Value,
    #[serde(default)]
    pub api_domain: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct QueryParams {
    #[serde(default)]
    pub code: String,
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub uuid: String,
    #[serde(default)]
    pub error: String,
    #[serde(default)]
    pub error_description: String,
}

#[derive(Debug, Deserialize, Default)]
pub struct WebExchangeRequest {
    #[serde(default)]
    pub code: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct Discovery {
    #[serde(default)]
    authorization_endpoint: String,
    #[serde(default)]
    token_endpoint: String,
    #[serde(default)]
    userinfo_endpoint: String,
    #[serde(default)]
    issuer: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct TokenResponse {
    #[serde(default)]
    access_token: String,
    #[serde(default)]
    id_token: String,
}

pub async fn status(State(state): State<AppState>) -> Json<Value> {
    let enabled = setting(&state, "oidc_enabled").await == "1";
    let configured = !setting(&state, "oidc_issuer").await.is_empty()
        && !setting(&state, "oidc_client_id").await.is_empty();
    let name = setting(&state, "oidc_name").await;
    Json(json!({
        "enabled": enabled && configured,
        "name": if name.is_empty() { "单点登录".to_owned() } else { name },
    }))
}

pub async fn auth_start(
    State(state): State<AppState>,
    Json(body): Json<OidcAuthRequest>,
) -> Result<Json<Value>, AppError> {
    if setting(&state, "oidc_enabled").await != "1" {
        return Ok(Json(json!({ "error": "OIDC 未启用。" })));
    }
    let issuer = setting(&state, "oidc_issuer").await;
    let client_id = setting(&state, "oidc_client_id").await;
    if issuer.is_empty() || client_id.is_empty() {
        return Ok(Json(json!({ "error": "OIDC 尚未配置。" })));
    }
    let discovery = match fetch_discovery(issuer).await {
        Ok(discovery) => discovery,
        Err(err) => return Ok(Json(json!({ "error": err }))),
    };
    if discovery.authorization_endpoint.is_empty() || discovery.token_endpoint.is_empty() {
        return Ok(Json(json!({ "error": "OIDC 提供方配置不完整。" })));
    }

    let api_domain = normalize_domain(&body.api_domain);
    if api_domain.is_empty() {
        return Ok(Json(json!({ "error": "缺少 API 服务地址。" })));
    }
    let redirect_uri = {
        let configured = setting(&state, "oidc_redirect_url").await;
        if configured.is_empty() {
            format!("{api_domain}/api/oidc/callback")
        } else {
            configured
        }
    };
    let scopes = {
        let value = setting(&state, "oidc_scopes").await;
        if value.is_empty() {
            "openid profile email".to_owned()
        } else {
            value
        }
    };
    let code = uuid::Uuid::new_v4().simple().to_string();
    let now = now_secs();
    sqlx::query(
        "INSERT INTO oidc_sessions
            (code, op, device_id, device_uuid, device_info, redirect_uri, username, error,
             authorized, consumed, created_at, expires_at)
         VALUES (?, ?, ?, ?, ?, ?, '', '', 0, 0, ?, ?)",
    )
    .bind(&code)
    .bind(body.op.trim())
    .bind(body.id.trim())
    .bind(body.uuid.trim())
    .bind(body.device_info.to_string())
    .bind(&redirect_uri)
    .bind(now)
    .bind(now + SESSION_TTL_SECS)
    .execute(&state.db)
    .await?;
    let _ = sqlx::query("DELETE FROM oidc_sessions WHERE expires_at < ?")
        .bind(now)
        .execute(&state.db)
        .await;

    let url = format!(
        "{}{}client_id={}&redirect_uri={}&response_type=code&scope={}&state={}",
        discovery.authorization_endpoint,
        if discovery.authorization_endpoint.contains('?') {
            "&"
        } else {
            "?"
        },
        urlencoding::encode(&client_id),
        urlencoding::encode(&redirect_uri),
        urlencoding::encode(&scopes),
        urlencoding::encode(&code),
    );
    Ok(Json(json!({ "code": code, "url": url })))
}

pub async fn callback(
    State(state): State<AppState>,
    Query(query): Query<QueryParams>,
) -> Response {
    let code = if query.state.trim().is_empty() {
        query.code.trim()
    } else {
        query.state.trim()
    };
    let session: Option<(String, String, String, i64)> = sqlx::query_as(
        "SELECT op, device_id, device_uuid, expires_at FROM oidc_sessions WHERE code = ?",
    )
    .bind(code)
    .fetch_optional(&state.db)
    .await
    .ok()
    .flatten();
    let Some((op, device_id, device_uuid, expires_at)) = session else {
        return callback_page(StatusCode::BAD_REQUEST, false, "登录会话不存在或已过期。");
    };
    if expires_at < now_secs() {
        return callback_page(StatusCode::BAD_REQUEST, false, "登录会话已过期，请重试。");
    }

    if !query.error.trim().is_empty() {
        let message = if query.error_description.trim().is_empty() {
            query.error.trim().to_owned()
        } else {
            query.error_description.trim().to_owned()
        };
        let _ = set_session_error(&state, code, &message).await;
        if op == "web" {
            return Redirect::to(&format!(
                "/#/login?oidc_error={}",
                urlencoding::encode(&message)
            ))
            .into_response();
        }
        return callback_page(StatusCode::BAD_REQUEST, false, &message);
    }
    if query.code.trim().is_empty() {
        let message = "OIDC 回调缺少授权码。";
        let _ = set_session_error(&state, code, message).await;
        return callback_page(StatusCode::BAD_REQUEST, false, message);
    }

    match complete_authorization(&state, code, query.code.trim()).await {
        Ok(username) => {
            if op == "web" {
                return Redirect::to(&format!(
                    "/#/login?oidc_code={}",
                    urlencoding::encode(code)
                ))
                .into_response();
            }
            if !device_id.is_empty() {
                if let Err(err) =
                    super::bind_device(&state, &device_id, &device_uuid, &username).await
                {
                    tracing::warn!(error = %err, "failed to bind device on oidc login");
                }
            }
            super::audit(&state, &username, "login.oidc", &device_id, "oidc").await;
            callback_page(StatusCode::OK, true, "登录成功，请返回 RustDesk 客户端。")
        }
        Err(err) => {
            let message = err.to_string();
            let _ = set_session_error(&state, code, &message).await;
            if op == "web" {
                return Redirect::to(&format!(
                    "/#/login?oidc_error={}",
                    urlencoding::encode(&message)
                ))
                .into_response();
            }
            callback_page(StatusCode::UNAUTHORIZED, false, &message)
        }
    }
}

async fn set_session_error(state: &AppState, code: &str, message: &str) -> Result<(), AppError> {
    sqlx::query("UPDATE oidc_sessions SET error = ? WHERE code = ?")
        .bind(message)
        .bind(code)
        .execute(&state.db)
        .await?;
    Ok(())
}

async fn complete_authorization(
    state: &AppState,
    code: &str,
    authorization_code: &str,
) -> Result<String, AppError> {
    let (redirect_uri, expires_at): (String, i64) =
        sqlx::query_as("SELECT redirect_uri, expires_at FROM oidc_sessions WHERE code = ?")
            .bind(code)
            .fetch_optional(&state.db)
            .await?
            .ok_or_else(|| AppError::BadRequest("登录会话不存在。".to_owned()))?;
    if expires_at < now_secs() {
        return Err(AppError::BadRequest("登录会话已过期。".to_owned()));
    }
    let issuer = setting(state, "oidc_issuer").await;
    let client_id = setting(state, "oidc_client_id").await;
    let client_secret = setting(state, "oidc_client_secret").await;
    let discovery = fetch_discovery(issuer).await.map_err(AppError::BadRequest)?;
    let tokens = exchange_code(
        &discovery,
        &client_id,
        &client_secret,
        authorization_code,
        &redirect_uri,
    )
    .await
    .map_err(AppError::BadRequest)?;

    let mut claims = if discovery.userinfo_endpoint.is_empty() {
        Value::Null
    } else {
        fetch_userinfo(&discovery.userinfo_endpoint, &tokens.access_token)
            .await
            .unwrap_or(Value::Null)
    };
    if !claims.is_object() && !tokens.id_token.is_empty() {
        claims = claims_from_id_token(&tokens.id_token).unwrap_or(Value::Null);
    }
    if !claims.is_object() {
        return Err(AppError::BadRequest("无法获取 OIDC 用户信息。".to_owned()));
    }

    let subject = claims
        .get("sub")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_owned();
    if subject.is_empty() {
        return Err(AppError::BadRequest("OIDC 用户信息缺少 sub。".to_owned()));
    }
    let claim_name = {
        let configured = setting(state, "oidc_username_claim").await;
        if configured.is_empty() {
            "preferred_username".to_owned()
        } else {
            configured
        }
    };
    let raw_username = claims
        .get(&claim_name)
        .and_then(Value::as_str)
        .or_else(|| claims.get("preferred_username").and_then(Value::as_str))
        .or_else(|| claims.get("name").and_then(Value::as_str))
        .or_else(|| claims.get("email").and_then(Value::as_str))
        .unwrap_or(&subject)
        .trim()
        .to_owned();
    let username = sanitize_username(&raw_username)
        .ok_or_else(|| AppError::BadRequest("OIDC 用户名无效。".to_owned()))?;
    let email = claims
        .get("email")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_owned();
    let display_name = claims
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or(&username)
        .trim()
        .to_owned();
    let avatar = claims
        .get("picture")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();
    let email_verified = claims
        .get("email_verified")
        .and_then(|value| match value {
            Value::Bool(flag) => Some(*flag),
            Value::String(text) => Some(text == "true"),
            _ => None,
        })
        .unwrap_or(false);

    let user = link_or_provision(
        state,
        &subject,
        &username,
        &display_name,
        &email,
        &avatar,
        email_verified,
    )
    .await?;
    if user.status != 1 {
        return Err(AppError::Unauthorized("账号已被禁用。".to_owned()));
    }
    sqlx::query("UPDATE oidc_sessions SET username = ?, authorized = 1, error = '' WHERE code = ?")
        .bind(&user.username)
        .bind(code)
        .execute(&state.db)
        .await?;
    Ok(user.username)
}

#[allow(clippy::too_many_arguments)]
async fn link_or_provision(
    state: &AppState,
    subject: &str,
    username: &str,
    display_name: &str,
    email: &str,
    avatar: &str,
    email_verified: bool,
) -> Result<UserRow, AppError> {
    if let Some(user) = sqlx::query_as::<_, UserRow>(&format!(
        "SELECT {} FROM users WHERE oidc_subject = ? LIMIT 1",
        user_columns()
    ))
    .bind(subject)
    .fetch_optional(&state.db)
    .await?
    {
        return Ok(user);
    }

    if let Some(existing) = super::auth::find_user(state, username).await? {
        if existing.oidc_subject.is_empty()
            && !email.is_empty()
            && !existing.email.is_empty()
            && existing.email.eq_ignore_ascii_case(email)
        {
            sqlx::query("UPDATE users SET oidc_subject = ?, updated_at = ? WHERE id = ?")
                .bind(subject)
                .bind(now_secs())
                .bind(existing.id)
                .execute(&state.db)
                .await?;
            return super::auth::find_user(state, username)
                .await?
                .ok_or_else(|| AppError::Internal("failed to link oidc user".to_owned()));
        }
        return Err(AppError::Conflict(
            "该用户名已被占用，无法通过单点登录创建账号。".to_owned(),
        ));
    }

    let auto_create = setting(state, "oidc_auto_create").await != "0";
    if !auto_create {
        return Err(AppError::Unauthorized(
            "账号不存在，请联系管理员。".to_owned(),
        ));
    }
    let random = uuid::Uuid::new_v4().simple().to_string();
    let password_hash = bcrypt::hash(&random, bcrypt::DEFAULT_COST)
        .map_err(|err| AppError::Internal(format!("failed to hash password: {err}")))?;
    let now = now_secs();
    sqlx::query(
        "INSERT INTO users
            (username, display_name, password_hash, email, note, avatar, status, is_admin,
             oidc_subject, email_verified, created_at, updated_at)
         VALUES (?, ?, ?, ?, 'OIDC 自动创建', ?, 1, 0, ?, ?, ?, ?)",
    )
    .bind(username)
    .bind(if display_name.is_empty() {
        username
    } else {
        display_name
    })
    .bind(password_hash)
    .bind(email)
    .bind(avatar)
    .bind(subject)
    .bind(if email_verified { 1 } else { 0 })
    .bind(now)
    .bind(now)
    .execute(&state.db)
    .await?;
    super::auth::find_user(state, username)
        .await?
        .ok_or_else(|| AppError::Internal("failed to provision oidc user".to_owned()))
}

pub async fn auth_query(
    State(state): State<AppState>,
    Query(query): Query<QueryParams>,
) -> Json<Value> {
    let session: Option<(String, String, String, String, i64, i64)> = sqlx::query_as(
        "SELECT username, error, device_id, device_uuid, authorized, expires_at
         FROM oidc_sessions WHERE code = ?",
    )
    .bind(query.code.trim())
    .fetch_optional(&state.db)
    .await
    .ok()
    .flatten();
    let Some((username, error, device_id, device_uuid, authorized, expires_at)) = session else {
        return Json(json!({ "body": json!({ "error": PENDING_ERROR }).to_string() }));
    };
    if expires_at < now_secs() {
        return Json(json!({ "body": json!({ "error": "OIDC 登录超时，请重试。" }).to_string() }));
    }
    if !query.id.trim().is_empty() && !device_id.is_empty() && query.id.trim() != device_id {
        return Json(json!({ "body": json!({ "error": "OIDC 登录设备不匹配。" }).to_string() }));
    }
    if !query.uuid.trim().is_empty() && !device_uuid.is_empty() && query.uuid.trim() != device_uuid {
        return Json(json!({ "body": json!({ "error": "OIDC 登录设备不匹配。" }).to_string() }));
    }
    if authorized == 0 || username.is_empty() {
        let message = if error.is_empty() { PENDING_ERROR } else { &error };
        return Json(json!({ "body": json!({ "error": message }).to_string() }));
    }

    let Some(user) = super::auth::find_user(&state, &username).await.ok().flatten() else {
        return Json(json!({ "body": json!({ "error": "账号不存在。" }).to_string() }));
    };
    if user.status != 1 {
        return Json(json!({ "body": json!({ "error": "账号已被禁用。" }).to_string() }));
    }
    let token = match auth::create_token(&state, &user) {
        Ok(token) => token,
        Err(err) => {
            return Json(json!({ "body": json!({ "error": err.to_string() }).to_string() }))
        }
    };
    let permissions = rbac::load_permissions(&state.db, user.id, user.is_admin != 0)
        .await
        .unwrap_or_default();
    let roles: Vec<String> = rbac::load_roles(&state.db, user.id)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|(_, name)| name)
        .collect();
    let body = json!({
        "access_token": token,
        "type": "access_token",
        "tfa_type": "",
        "secret": "",
        "user": crate::models::user_json_with(&user, &permissions, &roles),
    });
    Json(json!({ "body": body.to_string() }))
}

pub async fn web_exchange(
    State(state): State<AppState>,
    Json(body): Json<WebExchangeRequest>,
) -> Result<Json<Value>, AppError> {
    let code = body.code.trim();
    if code.is_empty() {
        return Err(AppError::BadRequest("缺少登录凭据。".to_owned()));
    }
    let session: Option<(String, String, i64, i64)> = sqlx::query_as(
        "SELECT op, username, authorized, expires_at FROM oidc_sessions WHERE code = ?",
    )
    .bind(code)
    .fetch_optional(&state.db)
    .await?;
    let Some((op, username, authorized, expires_at)) = session else {
        return Err(AppError::Unauthorized("登录会话不存在。".to_owned()));
    };
    if op != "web" || authorized == 0 || expires_at < now_secs() {
        return Err(AppError::Unauthorized("登录会话无效或已过期。".to_owned()));
    }
    let user = super::auth::find_user(&state, &username)
        .await?
        .ok_or_else(|| AppError::Unauthorized("账号不存在。".to_owned()))?;
    if user.status != 1 {
        return Err(AppError::Unauthorized("账号已被禁用。".to_owned()));
    }
    sqlx::query("UPDATE oidc_sessions SET consumed = 1 WHERE code = ?")
        .bind(code)
        .execute(&state.db)
        .await?;
    let payload = super::auth::complete_login(&state, &user, "", "", "oidc-web").await?;
    Ok(Json(payload))
}

fn callback_page(status: StatusCode, success: bool, message: &str) -> Response {
    let title = if success { "登录成功" } else { "登录失败" };
    let color = if success { "#16a34a" } else { "#dc2626" };
    let message = html_escape(message);
    let body = format!(
        "<!doctype html><html lang=\"zh-CN\"><head><meta charset=\"utf-8\">\
         <meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\
         <title>{title}</title><style>\
         body{{font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif;background:#0f172a;\
         color:#e2e8f0;display:flex;align-items:center;justify-content:center;height:100vh;margin:0}}\
         .card{{background:#1e293b;padding:40px 48px;border-radius:16px;text-align:center;\
         box-shadow:0 20px 60px rgba(0,0,0,.45);max-width:420px}}\
         h1{{font-size:22px;margin:0 0 12px;color:{color}}}p{{margin:0;color:#94a3b8;line-height:1.7}}\
         </style></head><body><div class=\"card\"><h1>{title}</h1><p>{message}</p></div>\
         <script>if({success})setTimeout(function(){{window.close()}},3000)</script>\
         </body></html>",
        success = if success { "true" } else { "false" }
    );
    (status, Html(body)).into_response()
}

fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn normalize_domain(value: &str) -> String {
    value.trim().trim_end_matches('/').to_owned()
}

fn sanitize_username(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    let value: String = value
        .chars()
        .filter(|ch| !ch.is_control() && !ch.is_whitespace())
        .take(64)
        .collect();
    if value.is_empty() {
        return None;
    }
    Some(value)
}

fn user_columns() -> &'static str {
    "id, username, display_name, password_hash, email, note, avatar, status, is_admin,
     totp_secret, totp_enabled, email_verification, email_alarm_notification, email_verified,
     ldap_dn, oidc_subject, created_at, updated_at"
}

async fn setting(state: &AppState, key: &str) -> String {
    db::get_setting(&state.db, key)
        .await
        .ok()
        .flatten()
        .unwrap_or_default()
}

async fn fetch_discovery(issuer: String) -> Result<Discovery, String> {
    let issuer = issuer.trim().trim_end_matches('/').to_owned();
    if issuer.is_empty() {
        return Err("OIDC 签发者地址为空。".to_owned());
    }
    let url = format!("{issuer}/.well-known/openid-configuration");
    run_blocking(move || {
        let agent = agent();
        let response = agent
            .get(&url)
            .call()
            .map_err(|err| format!("无法访问 OIDC 配置：{err}"))?;
        response
            .into_json::<Discovery>()
            .map_err(|err| format!("OIDC 配置格式无效：{err}"))
    })
    .await
}

async fn exchange_code(
    discovery: &Discovery,
    client_id: &str,
    client_secret: &str,
    code: &str,
    redirect_uri: &str,
) -> Result<TokenResponse, String> {
    let endpoint = discovery.token_endpoint.clone();
    let client_id = client_id.to_owned();
    let client_secret = client_secret.to_owned();
    let code = code.to_owned();
    let redirect_uri = redirect_uri.to_owned();
    run_blocking(move || {
        let agent = agent();
        let mut request = agent.post(&endpoint);
        if !client_secret.is_empty() {
            request = request.set("Authorization", &basic_auth(&client_id, &client_secret));
        }
        let response = request
            .send_form(&[
                ("grant_type", "authorization_code"),
                ("code", code.as_str()),
                ("redirect_uri", redirect_uri.as_str()),
                ("client_id", client_id.as_str()),
            ])
            .map_err(|err| format!("OIDC 令牌交换失败：{err}"))?;
        response
            .into_json::<TokenResponse>()
            .map_err(|err| format!("OIDC 令牌响应无效：{err}"))
    })
    .await
}

async fn fetch_userinfo(endpoint: &str, access_token: &str) -> Result<Value, String> {
    let endpoint = endpoint.to_owned();
    let access_token = access_token.to_owned();
    run_blocking(move || {
        let agent = agent();
        let response = agent
            .get(&endpoint)
            .set("Authorization", &format!("Bearer {access_token}"))
            .call()
            .map_err(|err| format!("OIDC 用户信息请求失败：{err}"))?;
        response
            .into_json::<Value>()
            .map_err(|err| format!("OIDC 用户信息格式无效：{err}"))
    })
    .await
}

fn claims_from_id_token(id_token: &str) -> Option<Value> {
    use base64::Engine;
    let payload = id_token.split('.').nth(1)?;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload)
        .or_else(|_| base64::engine::general_purpose::STANDARD_NO_PAD.decode(payload))
        .ok()?;
    serde_json::from_slice(&bytes).ok()
}

fn basic_auth(username: &str, password: &str) -> String {
    use base64::Engine;
    format!(
        "Basic {}",
        base64::engine::general_purpose::STANDARD.encode(format!("{username}:{password}"))
    )
}

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(10))
        .timeout(Duration::from_secs(20))
        .build()
}

async fn run_blocking<T, F>(task: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, String> + Send + 'static,
{
    tokio::task::spawn_blocking(task)
        .await
        .map_err(|err| format!("OIDC 请求线程失败：{err}"))?
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct OidcSettingsRequest {
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub issuer: Option<String>,
    #[serde(default)]
    pub client_id: Option<String>,
    #[serde(default)]
    pub client_secret: Option<String>,
    #[serde(default)]
    pub scopes: Option<String>,
    #[serde(default)]
    pub redirect_url: Option<String>,
    #[serde(default)]
    pub auto_create: Option<bool>,
    #[serde(default)]
    pub username_claim: Option<String>,
    #[serde(default)]
    pub clear_secret: Option<bool>,
}

pub async fn get_settings(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, AppError> {
    require(&auth, "settings.read")?;
    Ok(Json(json!({
        "enabled": setting(&state, "oidc_enabled").await == "1",
        "name": setting(&state, "oidc_name").await,
        "issuer": setting(&state, "oidc_issuer").await,
        "client_id": setting(&state, "oidc_client_id").await,
        "client_secret_set": !setting(&state, "oidc_client_secret").await.is_empty(),
        "scopes": setting(&state, "oidc_scopes").await,
        "redirect_url": setting(&state, "oidc_redirect_url").await,
        "auto_create": setting(&state, "oidc_auto_create").await != "0",
        "username_claim": setting(&state, "oidc_username_claim").await,
    })))
}

pub async fn save_settings(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(body): Json<OidcSettingsRequest>,
) -> Result<Json<Value>, AppError> {
    require(&auth, "settings.write")?;
    let mut updates: Vec<(&str, String)> = Vec::new();
    if let Some(enabled) = body.enabled {
        updates.push(("oidc_enabled", if enabled { "1" } else { "0" }.to_owned()));
    }
    if let Some(value) = body.name {
        updates.push(("oidc_name", value.trim().to_owned()));
    }
    if let Some(value) = body.issuer {
        updates.push((
            "oidc_issuer",
            value.trim().trim_end_matches('/').to_owned(),
        ));
    }
    if let Some(value) = body.client_id {
        updates.push(("oidc_client_id", value.trim().to_owned()));
    }
    if body.clear_secret.unwrap_or(false) {
        updates.push(("oidc_client_secret", String::new()));
    } else if let Some(value) = body.client_secret {
        if !value.trim().is_empty() {
            updates.push(("oidc_client_secret", value.trim().to_owned()));
        }
    }
    if let Some(value) = body.scopes {
        updates.push(("oidc_scopes", value.trim().to_owned()));
    }
    if let Some(value) = body.redirect_url {
        updates.push((
            "oidc_redirect_url",
            value.trim().trim_end_matches('/').to_owned(),
        ));
    }
    if let Some(value) = body.auto_create {
        updates.push(("oidc_auto_create", if value { "1" } else { "0" }.to_owned()));
    }
    if let Some(value) = body.username_claim {
        updates.push(("oidc_username_claim", value.trim().to_owned()));
    }
    for (key, value) in updates {
        db::set_setting(&state.db, key, &value).await?;
    }
    super::audit(&state, &auth.username, "settings.oidc", "", "").await;
    Ok(Json(json!({ "success": true })))
}

pub async fn test_settings(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, AppError> {
    require(&auth, "settings.write")?;
    let issuer = setting(&state, "oidc_issuer").await;
    if issuer.is_empty() {
        return Err(AppError::BadRequest("请先填写 OIDC 签发者地址。".to_owned()));
    }
    match fetch_discovery(issuer).await {
        Ok(discovery) => Ok(Json(json!({
            "success": true,
            "issuer": discovery.issuer,
            "authorization_endpoint": discovery.authorization_endpoint,
            "token_endpoint": discovery.token_endpoint,
            "userinfo_endpoint": discovery.userinfo_endpoint,
        }))),
        Err(err) => Ok(Json(json!({ "success": false, "error": err }))),
    }
}

fn require(auth: &AuthUser, permission: &str) -> Result<(), AppError> {
    if auth.can(permission) {
        Ok(())
    } else {
        Err(AppError::Forbidden("没有权限执行此操作。".to_owned()))
    }
}
