use axum::{
    extract::{rejection::JsonRejection, Json, State},
    http::StatusCode,
};
use serde_json::{json, Value};

use crate::auth::{create_token, revoke_token, AuthUser};
use crate::error::AppError;
use crate::models::{user_json, CurrentUserRequest, LoginRequest, UserRow};
use crate::AppState;

const USER_COLUMNS: &str = "id, username, display_name, password_hash, email, note, avatar, status, is_admin, totp_secret, totp_enabled, created_at, updated_at";

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

    let sql = format!("SELECT {USER_COLUMNS} FROM users WHERE username = ?");
    let user = sqlx::query_as::<_, UserRow>(&sql)
        .bind(username)
        .fetch_optional(&state.db)
        .await?;
    let user = match user {
        Some(user) => user,
        None => {
            super::audit(&state, username, "login_failed", "", "unknown_user").await;
            return Err(AppError::Unauthorized("用户名或密码错误。".to_owned()));
        }
    };

    if !request.password.is_empty() {
        match bcrypt::verify(&request.password, &user.password_hash) {
            Ok(true) => {}
            Ok(false) => {
                super::audit(&state, username, "login_failed", "", "bad_password").await;
                return Err(AppError::Unauthorized("用户名或密码错误。".to_owned()));
            }
            Err(err) => {
                tracing::warn!(username = %user.username, error = %err, "stored password hash is invalid");
                return Err(AppError::Unauthorized("用户名或密码错误。".to_owned()));
            }
        }
    } else if user.totp_enabled == 0 {
        super::audit(&state, username, "login_failed", "", "bad_password").await;
        return Err(AppError::Unauthorized("用户名或密码错误。".to_owned()));
    }

    if user.status != 1 {
        return Err(AppError::Forbidden("账号已被禁用。".to_owned()));
    }

    if user.totp_enabled != 0 {
        if code.is_empty() {
            let challenge = super::tfa::issue_login_challenge(&state, &user).await?;
            return Ok((
                StatusCode::OK,
                Json(json!({
                    "access_token": "",
                    "type": "email_check",
                    "tfa_type": "totp",
                    "secret": challenge,
                    "user": user_json(&user),
                })),
            ));
        }
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
    }

    if !request.id.trim().is_empty() {
        if let Err(err) = super::bind_device(
            &state,
            request.id.trim(),
            request.uuid.trim(),
            &user.username,
        )
        .await
        {
            tracing::warn!(error = %err, "failed to bind device on login");
        }
    }

    let token = create_token(&state, &user)?;
    super::audit(
        &state,
        &user.username,
        "login",
        request.id.trim(),
        request.device_info.os.as_str(),
    )
    .await;

    Ok((
        StatusCode::OK,
        Json(json!({
            "access_token": token,
            "type": "access_token",
            "tfa_type": "",
            "secret": "",
            "user": user_json(&user),
        })),
    ))
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

    let sql = format!("SELECT {USER_COLUMNS} FROM users WHERE id = ?");
    let user = sqlx::query_as::<_, UserRow>(&sql)
        .bind(auth.id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::Unauthorized("账号不存在。".to_owned()))?;
    Ok(Json(user_json(&user)))
}

pub async fn login_options() -> Json<Vec<String>> {
    Json(Vec::new())
}
