use axum::{
    extract::{rejection::JsonRejection, Json, State},
    http::StatusCode,
};
use serde_json::{json, Value};

use crate::auth::{create_token, revoke_token, AuthUser};
use crate::error::AppError;
use crate::models::{user_json, CurrentUserRequest, LoginRequest, UserRow};
use crate::AppState;

const USER_COLUMNS: &str = "id, username, display_name, password_hash, email, note, avatar, status, is_admin, created_at, updated_at";

pub async fn login(
    State(state): State<AppState>,
    payload: Result<Json<LoginRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<Value>), AppError> {
    let Json(request) = payload
        .map_err(|err| AppError::BadRequest(format!("请求格式错误：{err}")))?;
    let username = request.username.trim();
    if username.is_empty() || request.password.is_empty() {
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
            return Err(AppError::Unauthorized("用户名或密码错误。".to_owned()));
        }
    };

    match bcrypt::verify(&request.password, &user.password_hash) {
        Ok(true) => {}
        Ok(false) => {
            return Err(AppError::Unauthorized("用户名或密码错误。".to_owned()));
        }
        Err(err) => {
            tracing::warn!(username = %user.username, error = %err, "stored password hash is invalid");
            return Err(AppError::Unauthorized("用户名或密码错误。".to_owned()));
        }
    }

    if user.status != 1 {
        return Err(AppError::Forbidden("账号已被禁用。".to_owned()));
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
