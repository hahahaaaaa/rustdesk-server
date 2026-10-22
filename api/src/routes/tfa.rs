use axum::{
    extract::{rejection::JsonRejection, Path, State},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::auth::{
    create_tfa_challenge, decode_tfa_challenge, AuthUser,
};
use crate::error::AppError;
use crate::models::{now_secs, UserRow};
use crate::totp;
use crate::AppState;

const ISSUER: &str = "RustDesk Server";
const MAX_CHALLENGE_ATTEMPTS: i64 = 5;
const USER_COLUMNS: &str = "id, username, display_name, password_hash, email, note, avatar, status, is_admin, totp_secret, totp_enabled, email_verification, email_alarm_notification, email_verified, ldap_dn, oidc_subject, created_at, updated_at";

#[derive(Debug, Clone, Deserialize, Default)]
pub struct CodeRequest {
    #[serde(default)]
    pub code: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct DisableRequest {
    #[serde(default)]
    pub password: String,
    #[serde(default)]
    pub code: String,
}

pub async fn status(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, AppError> {
    let user = load_user_by_id(&state, auth.id).await?;
    Ok(Json(json!({ "enabled": user.totp_enabled != 0 })))
}

pub async fn setup(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, AppError> {
    let user = load_user_by_id(&state, auth.id).await?;
    if user.totp_enabled != 0 {
        return Err(AppError::Conflict("双重认证已启用。".to_owned()));
    }

    let secret = totp::generate_secret();
    sqlx::query(
        "UPDATE users SET totp_secret = ?, totp_enabled = 0, updated_at = ? WHERE id = ?",
    )
    .bind(&secret)
    .bind(now_secs())
    .bind(user.id)
    .execute(&state.db)
    .await?;

    Ok(Json(json!({
        "secret": secret,
        "issuer": ISSUER,
        "account": user.username,
        "otpauth_url": totp::otpauth_url(ISSUER, &user.username, &secret),
    })))
}

pub async fn enable(
    State(state): State<AppState>,
    auth: AuthUser,
    payload: Result<Json<CodeRequest>, JsonRejection>,
) -> Result<Json<Value>, AppError> {
    let Json(request) = payload
        .map_err(|err| AppError::BadRequest(format!("请求格式错误：{err}")))?;
    let user = load_user_by_id(&state, auth.id).await?;
    if user.totp_enabled != 0 {
        return Err(AppError::Conflict("双重认证已启用。".to_owned()));
    }
    if user.totp_secret.is_empty() {
        return Err(AppError::BadRequest("请先生成双重认证密钥。".to_owned()));
    }
    if !totp::verify_code(&user.totp_secret, &request.code, now_secs()) {
        return Err(AppError::BadRequest("验证码不正确。".to_owned()));
    }

    sqlx::query("UPDATE users SET totp_enabled = 1, updated_at = ? WHERE id = ?")
        .bind(now_secs())
        .bind(user.id)
        .execute(&state.db)
        .await?;
    super::audit(&state, &user.username, "user.tfa_enable", &user.username, "").await;
    Ok(Json(json!({ "success": true, "enabled": true })))
}

pub async fn disable(
    State(state): State<AppState>,
    auth: AuthUser,
    payload: Result<Json<DisableRequest>, JsonRejection>,
) -> Result<Json<Value>, AppError> {
    let Json(request) = payload
        .map_err(|err| AppError::BadRequest(format!("请求格式错误：{err}")))?;
    let user = load_user_by_id(&state, auth.id).await?;
    if user.totp_enabled == 0 {
        return Err(AppError::BadRequest("双重认证尚未启用。".to_owned()));
    }
    let password_ok = bcrypt::verify(request.password.trim(), &user.password_hash)
        .unwrap_or(false);
    if !password_ok {
        return Err(AppError::BadRequest("密码不正确。".to_owned()));
    }
    if !totp::verify_code(&user.totp_secret, &request.code, now_secs()) {
        return Err(AppError::BadRequest("验证码不正确。".to_owned()));
    }

    sqlx::query(
        "UPDATE users SET totp_secret = '', totp_enabled = 0, updated_at = ? WHERE id = ?",
    )
    .bind(now_secs())
    .bind(user.id)
    .execute(&state.db)
    .await?;
    super::audit(&state, &user.username, "user.tfa_disable", &user.username, "").await;
    Ok(Json(json!({ "success": true, "enabled": false })))
}

pub async fn reset_user(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    if !auth.is_admin {
        return Err(AppError::Forbidden("需要管理员权限。".to_owned()));
    }
    let user = load_user_by_id(&state, id).await?;
    sqlx::query(
        "UPDATE users SET totp_secret = '', totp_enabled = 0, updated_at = ? WHERE id = ?",
    )
    .bind(now_secs())
    .bind(user.id)
    .execute(&state.db)
    .await?;
    super::audit(
        &state,
        &auth.username,
        "user.tfa_reset",
        &user.username,
        "",
    )
    .await;
    Ok(Json(json!({ "success": true })))
}

pub async fn issue_login_challenge(
    state: &AppState,
    user: &UserRow,
) -> Result<String, AppError> {
    let now = now_secs();
    let (token, claims) = create_tfa_challenge(state, user)?;
    let mut tx = state.db.begin().await?;
    sqlx::query("DELETE FROM tfa_challenges WHERE username = ? OR expires_at < ?")
        .bind(&user.username)
        .bind(now)
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "INSERT INTO tfa_challenges (jti, username, attempts, expires_at) VALUES (?, ?, 0, ?)",
    )
    .bind(&claims.jti)
    .bind(&user.username)
    .bind(claims.exp)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(token)
}

pub async fn check_login_challenge(
    state: &AppState,
    username: &str,
    token: &str,
) -> Result<String, AppError> {
    if token.trim().is_empty() {
        return Err(AppError::Unauthorized(
            "请先使用账号密码登录以开始两步验证。".to_owned(),
        ));
    }
    let claims = decode_tfa_challenge(state, token)?;
    if claims.sub != username {
        return Err(AppError::Unauthorized("两步验证会话与账号不匹配。".to_owned()));
    }
    let row: Option<(i64, i64)> = sqlx::query_as(
        "SELECT attempts, expires_at FROM tfa_challenges WHERE jti = ? AND username = ?",
    )
    .bind(&claims.jti)
    .bind(username)
    .fetch_optional(&state.db)
    .await?;
    let (attempts, expires_at) = row.ok_or_else(|| {
        AppError::Unauthorized("两步验证会话无效或已过期，请重新登录。".to_owned())
    })?;
    if expires_at < now_secs() {
        sqlx::query("DELETE FROM tfa_challenges WHERE jti = ?")
            .bind(&claims.jti)
            .execute(&state.db)
            .await?;
        return Err(AppError::Unauthorized(
            "两步验证会话已过期，请重新登录。".to_owned(),
        ));
    }
    if attempts >= MAX_CHALLENGE_ATTEMPTS {
        sqlx::query("DELETE FROM tfa_challenges WHERE jti = ?")
            .bind(&claims.jti)
            .execute(&state.db)
            .await?;
        return Err(AppError::Unauthorized(
            "验证码尝试次数过多，请重新登录。".to_owned(),
        ));
    }
    Ok(claims.jti)
}

pub async fn fail_login_challenge(state: &AppState, jti: &str) -> Result<(), AppError> {
    sqlx::query("UPDATE tfa_challenges SET attempts = attempts + 1 WHERE jti = ?")
        .bind(jti)
        .execute(&state.db)
        .await?;
    sqlx::query("DELETE FROM tfa_challenges WHERE jti = ? AND attempts >= ?")
        .bind(jti)
        .bind(MAX_CHALLENGE_ATTEMPTS)
        .execute(&state.db)
        .await?;
    Ok(())
}

pub async fn complete_login_challenge(state: &AppState, jti: &str) -> Result<(), AppError> {
    sqlx::query("DELETE FROM tfa_challenges WHERE jti = ?")
        .bind(jti)
        .execute(&state.db)
        .await?;
    Ok(())
}

async fn load_user_by_id(state: &AppState, id: i64) -> Result<UserRow, AppError> {
    let sql = format!("SELECT {USER_COLUMNS} FROM users WHERE id = ?");
    sqlx::query_as::<_, UserRow>(&sql)
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("用户不存在。".to_owned()))
}
