use axum::{
    extract::{rejection::JsonRejection, Json, State},
    http::StatusCode,
};
use serde_json::{json, Value};

use crate::auth::{create_token, decode_challenge, revoke_token, AuthUser};
use crate::db;
use crate::directory;
use crate::error::AppError;
use crate::mailer;
use crate::models::{now_secs, user_json, user_json_with, CurrentUserRequest, LoginRequest, UserRow};
use crate::rbac;
use crate::AppState;

const USER_COLUMNS: &str = "id, username, display_name, password_hash, email, note, avatar, status, is_admin, totp_secret, totp_enabled, email_verification, email_alarm_notification, email_verified, ldap_dn, oidc_subject, created_at, updated_at";

pub async fn login(
    State(state): State<AppState>,
    payload: Result<Json<LoginRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<Value>), AppError> {
    let Json(request) = payload
        .map_err(|err| AppError::BadRequest(format!("请求格式错误：{err}")))?;
    let username = request.username.trim();
    let code = if !request.tfa_code.trim().is_empty() {
        request.tfa_code.trim()
    } else {
        request.verification_code.trim()
    };
    if username.is_empty() {
        return Err(AppError::BadRequest("请输入用户名。".to_owned()));
    }
    if request.password.is_empty() && (code.is_empty() || request.secret.trim().is_empty()) {
        return Err(AppError::BadRequest("请输入用户名和密码。".to_owned()));
    }

    let mut user = match find_user(&state, username).await? {
        Some(user) => user,
        None => {
            let ldap = directory::LdapConfig::load(&state).await?;
            if let (Some(config), false) = (ldap.as_ref(), request.password.is_empty()) {
                match directory::authenticate(config, username, &request.password).await {
                    Ok(Some(dn)) => provision_ldap_user(&state, username, &dn).await?,
                    Ok(None) => {
                        super::audit(&state, username, "login_failed", "", "unknown_user").await;
                        return Err(AppError::Unauthorized("用户名或密码错误。".to_owned()));
                    }
                    Err(err) => {
                        tracing::warn!(username = %username, error = %err, "LDAP authentication failed");
                        super::audit(&state, username, "login_failed", "", "ldap_error").await;
                        return Err(AppError::Unauthorized("用户名或密码错误。".to_owned()));
                    }
                }
            } else {
                super::audit(&state, username, "login_failed", "", "unknown_user").await;
                return Err(AppError::Unauthorized("用户名或密码错误。".to_owned()));
            }
        }
    };

    let mut password_ok = false;
    if !request.password.is_empty() {
        password_ok = bcrypt::verify(&request.password, &user.password_hash).unwrap_or(false);
        if !password_ok {
            if let Ok(Some(config)) = directory::LdapConfig::load(&state).await {
                match directory::authenticate(&config, username, &request.password).await {
                    Ok(Some(dn)) => {
                        password_ok = true;
                        if dn != user.ldap_dn {
                            sqlx::query(
                                "UPDATE users SET ldap_dn = ?, updated_at = ? WHERE id = ?",
                            )
                            .bind(&dn)
                            .bind(now_secs())
                            .bind(user.id)
                            .execute(&state.db)
                            .await?;
                            user.ldap_dn = dn;
                        }
                    }
                    Ok(None) => {}
                    Err(err) => {
                        tracing::warn!(username = %user.username, error = %err, "LDAP authentication failed");
                    }
                }
            }
        }
        if !password_ok {
            super::audit(&state, username, "login_failed", "", "bad_password").await;
            return Err(AppError::Unauthorized("用户名或密码错误。".to_owned()));
        }
    }

    if user.status != 1 {
        return Err(AppError::Forbidden("账号已被禁用。".to_owned()));
    }

    let mut totp_verified = false;
    let mut email_verified = false;
    if !code.is_empty() {
        if !request.secret.trim().is_empty()
            && decode_challenge(&state, request.secret.trim(), "email").is_ok()
        {
            mailer::verify_email_challenge(
                &state,
                &user.username,
                request.secret.trim(),
                code,
            )
            .await?;
            email_verified = true;
        } else if user.totp_enabled != 0
            && !request.secret.trim().is_empty()
            && decode_challenge(&state, request.secret.trim(), "tfa").is_ok()
        {
            let jti = super::tfa::check_login_challenge(
                &state,
                &user.username,
                request.secret.trim(),
            )
            .await?;
            if !crate::totp::verify_code(&user.totp_secret, code, crate::models::now_secs()) {
                super::tfa::fail_login_challenge(&state, &jti).await?;
                super::audit(&state, &user.username, "login_failed", "", "bad_tfa_code").await;
                return Err(AppError::Unauthorized("验证码不正确。".to_owned()));
            }
            super::tfa::complete_login_challenge(&state, &jti).await?;
            totp_verified = true;
        } else {
            super::audit(&state, &user.username, "login_failed", "", "bad_tfa_code").await;
            return Err(AppError::Unauthorized("验证码不正确。".to_owned()));
        }
    }

    if !email_verified {
        let allow_totp = !totp_verified;
        if let Some(payload) = challenge_response(&state, &user, allow_totp).await? {
            return Ok((StatusCode::OK, Json(payload)));
        }
    }

    let payload = complete_login(
        &state,
        &user,
        request.id.trim(),
        request.uuid.trim(),
        request.device_info.os.as_str(),
    )
    .await?;
    Ok((StatusCode::OK, Json(payload)))
}

async fn challenge_response(
    state: &AppState,
    user: &UserRow,
    allow_totp: bool,
) -> Result<Option<Value>, AppError> {
    if allow_totp && user.totp_enabled != 0 {
        let challenge = super::tfa::issue_login_challenge(state, user).await?;
        return Ok(Some(json!({
            "access_token": "",
            "type": "email_check",
            "tfa_type": "tfa_check",
            "secret": challenge,
            "user": user_json(user),
        })));
    }
    if user.email_verification != 0 && !user.email.is_empty() {
        if mailer::is_configured(state).await {
            let challenge = mailer::issue_email_challenge(state, user).await?;
            return Ok(Some(json!({
                "access_token": "",
                "type": "email_check",
                "tfa_type": "email_check",
                "secret": challenge,
                "user": user_json(user),
            })));
        }
        tracing::warn!(
            username = %user.username,
            "email verification is enabled but SMTP is not configured; skipping the email step"
        );
    }
    Ok(None)
}

pub(crate) async fn complete_or_challenge(
    state: &AppState,
    user: &UserRow,
    device_id: &str,
    device_uuid: &str,
    os: &str,
) -> Result<Value, AppError> {
    if let Some(payload) = challenge_response(state, user, true).await? {
        return Ok(payload);
    }
    complete_login(state, user, device_id, device_uuid, os).await
}

pub(crate) async fn complete_login(
    state: &AppState,
    user: &UserRow,
    device_id: &str,
    device_uuid: &str,
    os: &str,
) -> Result<Value, AppError> {
    if !device_id.trim().is_empty() {
        if let Err(err) =
            super::bind_device(state, device_id.trim(), device_uuid.trim(), &user.username).await
        {
            tracing::warn!(error = %err, "failed to bind device on login");
        }
    }

    let token = create_token(state, user)?;
    let permissions = rbac::load_permissions(&state.db, user.id, user.is_admin != 0).await?;
    let roles: Vec<String> = rbac::load_roles(&state.db, user.id)
        .await?
        .into_iter()
        .map(|(_, name)| name)
        .collect();
    super::audit(state, &user.username, "login", device_id.trim(), os).await;

    Ok(json!({
        "access_token": token,
        "type": "access_token",
        "tfa_type": "",
        "secret": "",
        "user": user_json_with(user, &permissions, &roles),
    }))
}

pub(crate) async fn find_user(
    state: &AppState,
    username: &str,
) -> Result<Option<UserRow>, AppError> {
    let sql = format!("SELECT {USER_COLUMNS} FROM users WHERE username = ?");
    Ok(sqlx::query_as::<_, UserRow>(&sql)
        .bind(username)
        .fetch_optional(&state.db)
        .await?)
}

async fn provision_ldap_user(
    state: &AppState,
    username: &str,
    dn: &str,
) -> Result<UserRow, AppError> {
    let auto_create = db::get_setting(&state.db, "ldap_auto_create")
        .await?
        .as_deref()
        != Some("0");
    if !auto_create {
        return Err(AppError::Unauthorized("用户名或密码错误。".to_owned()));
    }
    if !username
        .chars()
        .all(|ch| ch.is_alphanumeric() || matches!(ch, '.' | '_' | '-' | '@'))
        || username.chars().count() < 3
        || username.chars().count() > 64
    {
        return Err(AppError::Unauthorized("用户名或密码错误。".to_owned()));
    }
    let random = uuid::Uuid::new_v4().simple().to_string();
    let password_hash = bcrypt::hash(&random, bcrypt::DEFAULT_COST)
        .map_err(|err| AppError::Internal(format!("failed to hash password: {err}")))?;
    let now = now_secs();
    sqlx::query(
        "INSERT INTO users
            (username, display_name, password_hash, email, note, avatar, status, is_admin, ldap_dn, created_at, updated_at)
         VALUES (?, ?, ?, '', 'LDAP 自动创建', '', 1, 0, ?, ?, ?)",
    )
    .bind(username)
    .bind(username)
    .bind(password_hash)
    .bind(dn)
    .bind(now)
    .bind(now)
    .execute(&state.db)
    .await?;
    find_user(state, username)
        .await?
        .ok_or_else(|| AppError::Internal("failed to create LDAP user".to_owned()))
}

pub async fn logout(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, AppError> {
    revoke_token(&state, &auth.jti, auth.exp).await?;
    super::audit(&state, &auth.username, "logout", "", "").await;
    Ok(Json(json!({ "success": true })))
}

pub async fn current_user(
    State(state): State<AppState>,
    auth: AuthUser,
    payload: Result<Json<CurrentUserRequest>, JsonRejection>,
) -> Result<Json<Value>, AppError> {
    if let Ok(Json(request)) = payload {
        if !request.id.trim().is_empty() {
            if let Err(err) = super::bind_device(
                &state,
                request.id.trim(),
                request.uuid.trim(),
                &auth.username,
            )
            .await
            {
                tracing::warn!(error = %err, "failed to bind device on currentUser");
            }
        }
    }

    let user = find_user(&state, &auth.username)
        .await?
        .ok_or_else(|| AppError::Unauthorized("账号不存在。".to_owned()))?;
    let permissions = rbac::load_permissions(&state.db, user.id, user.is_admin != 0).await?;
    let roles: Vec<String> = rbac::load_roles(&state.db, user.id)
        .await?
        .into_iter()
        .map(|(_, name)| name)
        .collect();
    Ok(Json(user_json_with(&user, &permissions, &roles)))
}

pub async fn login_options(State(state): State<AppState>) -> Json<Vec<String>> {
    let enabled = db::get_setting(&state.db, "oidc_enabled")
        .await
        .ok()
        .flatten()
        .as_deref()
        == Some("1");
    if !enabled {
        return Json(Vec::new());
    }
    let configured = db::get_setting(&state.db, "oidc_issuer")
        .await
        .ok()
        .flatten()
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false)
        && db::get_setting(&state.db, "oidc_client_id")
            .await
            .ok()
            .flatten()
            .map(|value| !value.trim().is_empty())
            .unwrap_or(false);
    if !configured {
        return Json(Vec::new());
    }
    let name = db::get_setting(&state.db, "oidc_name")
        .await
        .ok()
        .flatten()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "单点登录".to_owned());
    Json(vec![format!("oidc/{name}")])
}
