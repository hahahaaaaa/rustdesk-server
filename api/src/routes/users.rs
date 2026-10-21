use axum::{
    extract::{rejection::JsonRejection, Path, State},
    Json,
};
use serde_json::{json, Value};

use crate::auth::AuthUser;
use crate::error::AppError;
use crate::models::{
    now_secs, user_json, UserCreateRequest, UserGroupsRequest, UserRow, UserUpdateRequest,
};
use crate::AppState;

const USER_COLUMNS: &str = "id, username, display_name, password_hash, email, note, avatar, status, is_admin, totp_secret, totp_enabled, created_at, updated_at";

pub async fn create_user(
    State(state): State<AppState>,
    auth: AuthUser,
    payload: Result<Json<UserCreateRequest>, JsonRejection>,
) -> Result<(axum::http::StatusCode, Json<Value>), AppError> {
    require_admin(&auth)?;
    let Json(request) = payload
        .map_err(|err| AppError::BadRequest(format!("请求格式错误：{err}")))?;
    let username = request.username.trim();
    validate_username(username)?;
    validate_password(&request.password)?;
    let display_name = request.display_name.trim();
    let email = request.email.trim();
    check_text_len(display_name, 64, "显示名称过长。")?;
    check_text_len(email, 128, "邮箱过长。")?;
    check_text_len(request.note.trim(), 240, "备注过长。")?;

    let exists: Option<i64> = sqlx::query_scalar("SELECT 1 FROM users WHERE username = ?")
        .bind(username)
        .fetch_optional(&state.db)
        .await?;
    if exists.is_some() {
        return Err(AppError::Conflict("用户名已存在。".to_owned()));
    }

    let password_hash = hash_password(&request.password).await?;
    let now = now_secs();
    let display_name = if display_name.is_empty() {
        username.to_owned()
    } else {
        display_name.to_owned()
    };
    let result = sqlx::query(
        "INSERT INTO users
            (username, display_name, password_hash, email, note, avatar, status, is_admin, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, '', 1, ?, ?, ?)",
    )
    .bind(username)
    .bind(&display_name)
    .bind(password_hash)
    .bind(email)
    .bind(request.note.trim())
    .bind(if request.is_admin { 1 } else { 0 })
    .bind(now)
    .bind(now)
    .execute(&state.db)
    .await;
    match result {
        Ok(_) => {}
        Err(sqlx::Error::Database(err)) if err.is_unique_violation() => {
            return Err(AppError::Conflict("用户名已存在。".to_owned()));
        }
        Err(err) => return Err(err.into()),
    }

    let user = load_user(&state, username).await?;
    super::audit(&state, &auth.username, "user.create", username, "").await;
    Ok((axum::http::StatusCode::CREATED, Json(user_json(&user))))
}

pub async fn update_user(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    payload: Result<Json<UserUpdateRequest>, JsonRejection>,
) -> Result<Json<Value>, AppError> {
    require_admin(&auth)?;
    let Json(request) = payload
        .map_err(|err| AppError::BadRequest(format!("请求格式错误：{err}")))?;
    let current = load_user_by_id(&state, id).await?;

    if let Some(password) = request.password.as_deref() {
        if !password.is_empty() {
            validate_password(password)?;
            let password_hash = hash_password(password).await?;
            sqlx::query(
                "UPDATE users SET password_hash = ?, updated_at = ? WHERE id = ?",
            )
            .bind(password_hash)
            .bind(now_secs())
            .bind(id)
            .execute(&state.db)
            .await?;
        }
    }

    if let Some(display_name) = request.display_name.as_deref() {
        let display_name = display_name.trim();
        check_text_len(display_name, 64, "显示名称过长。")?;
        let value = if display_name.is_empty() {
            current.username.clone()
        } else {
            display_name.to_owned()
        };
        sqlx::query("UPDATE users SET display_name = ?, updated_at = ? WHERE id = ?")
            .bind(value)
            .bind(now_secs())
            .bind(id)
            .execute(&state.db)
            .await?;
    }

    if let Some(email) = request.email.as_deref() {
        let email = email.trim();
        check_text_len(email, 128, "邮箱过长。")?;
        sqlx::query("UPDATE users SET email = ?, updated_at = ? WHERE id = ?")
            .bind(email)
            .bind(now_secs())
            .bind(id)
            .execute(&state.db)
            .await?;
    }

    if let Some(note) = request.note.as_deref() {
        let note = note.trim();
        check_text_len(note, 240, "备注过长。")?;
        sqlx::query("UPDATE users SET note = ?, updated_at = ? WHERE id = ?")
            .bind(note)
            .bind(now_secs())
            .bind(id)
            .execute(&state.db)
            .await?;
    }

    if let Some(status) = request.status {
        if !matches!(status, 0 | 1) {
            return Err(AppError::BadRequest("账号状态无效。".to_owned()));
        }
        if status != 1 {
            if id == auth.id {
                return Err(AppError::Forbidden("不能禁用当前登录的账号。".to_owned()));
            }
            ensure_not_last_admin(&state, &current).await?;
        }
        sqlx::query("UPDATE users SET status = ?, updated_at = ? WHERE id = ?")
            .bind(status)
            .bind(now_secs())
            .bind(id)
            .execute(&state.db)
            .await?;
    }

    if let Some(is_admin) = request.is_admin {
        if !is_admin && current.is_admin != 0 {
            if id == auth.id {
                return Err(AppError::Forbidden("不能取消当前登录账号的管理员权限。".to_owned()));
            }
            ensure_not_last_admin(&state, &current).await?;
        }
        sqlx::query("UPDATE users SET is_admin = ?, updated_at = ? WHERE id = ?")
            .bind(if is_admin { 1 } else { 0 })
            .bind(now_secs())
            .bind(id)
            .execute(&state.db)
            .await?;
    }

    let user = load_user_by_id(&state, id).await?;
    super::audit(&state, &auth.username, "user.update", &user.username, "").await;
    Ok(Json(user_json(&user)))
}

pub async fn delete_user(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    require_admin(&auth)?;
    if id == auth.id {
        return Err(AppError::Forbidden("不能删除当前登录的账号。".to_owned()));
    }
    let current = load_user_by_id(&state, id).await?;
    if current.is_admin != 0 {
        ensure_not_last_admin(&state, &current).await?;
    }

    let now = now_secs();
    let mut tx = state.db.begin().await?;
    sqlx::query("UPDATE devices SET user = NULL, updated_at = ? WHERE user = ?")
        .bind(now)
        .bind(&current.username)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM device_bindings WHERE username = ?")
        .bind(&current.username)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM user_device_groups WHERE user_id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM address_books WHERE owner = ? AND is_personal = 1")
        .bind(&current.username)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM strategy_assignments WHERE kind = 'user' AND target_id = ?")
        .bind(&current.username)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM tfa_challenges WHERE username = ?")
        .bind(&current.username)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM users WHERE id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    super::audit(&state, &auth.username, "user.delete", &current.username, "").await;
    Ok(Json(json!({ "success": true })))
}

pub async fn get_user_groups(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    require_admin(&auth)?;
    load_user_by_id(&state, id).await?;
    let groups: Vec<String> = sqlx::query_scalar(
        "SELECT group_guid FROM user_device_groups WHERE user_id = ? ORDER BY group_guid",
    )
    .bind(id)
    .fetch_all(&state.db)
    .await?;
    Ok(Json(json!({ "groups": groups })))
}

pub async fn set_user_groups(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    payload: Result<Json<UserGroupsRequest>, JsonRejection>,
) -> Result<Json<Value>, AppError> {
    require_admin(&auth)?;
    let Json(request) = payload
        .map_err(|err| AppError::BadRequest(format!("请求格式错误：{err}")))?;
    let current = load_user_by_id(&state, id).await?;

    let mut groups: Vec<String> = Vec::new();
    for group in request.groups {
        let group = group.trim().to_owned();
        if group.is_empty() || groups.contains(&group) {
            continue;
        }
        let exists: Option<i64> = sqlx::query_scalar("SELECT 1 FROM device_groups WHERE guid = ?")
            .bind(&group)
            .fetch_optional(&state.db)
            .await?;
        if exists.is_none() {
            return Err(AppError::BadRequest("分组不存在。".to_owned()));
        }
        groups.push(group);
    }

    let now = now_secs();
    let mut tx = state.db.begin().await?;
    sqlx::query("DELETE FROM user_device_groups WHERE user_id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    for group in &groups {
        sqlx::query(
            "INSERT INTO user_device_groups (user_id, group_guid, created_at) VALUES (?, ?, ?)",
        )
        .bind(id)
        .bind(group)
        .bind(now)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;

    super::audit(
        &state,
        &auth.username,
        "user.groups",
        &current.username,
        &groups.join(","),
    )
    .await;
    Ok(Json(json!({ "success": true, "groups": groups })))
}

async fn load_user(state: &AppState, username: &str) -> Result<UserRow, AppError> {
    let sql = format!("SELECT {USER_COLUMNS} FROM users WHERE username = ?");
    sqlx::query_as::<_, UserRow>(&sql)
        .bind(username)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("用户不存在。".to_owned()))
}

async fn load_user_by_id(state: &AppState, id: i64) -> Result<UserRow, AppError> {
    let sql = format!("SELECT {USER_COLUMNS} FROM users WHERE id = ?");
    sqlx::query_as::<_, UserRow>(&sql)
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("用户不存在。".to_owned()))
}

async fn ensure_not_last_admin(state: &AppState, target: &UserRow) -> Result<(), AppError> {
    if target.is_admin == 0 {
        return Ok(());
    }
    let remaining: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM users WHERE is_admin = 1 AND status = 1 AND id <> ?",
    )
    .bind(target.id)
    .fetch_one(&state.db)
    .await?;
    if remaining == 0 {
        return Err(AppError::Forbidden("系统至少需要保留一个管理员账号。".to_owned()));
    }
    Ok(())
}

fn require_admin(auth: &AuthUser) -> Result<(), AppError> {
    if auth.is_admin {
        Ok(())
    } else {
        Err(AppError::Forbidden("需要管理员权限。".to_owned()))
    }
}

fn validate_username(username: &str) -> Result<(), AppError> {
    let length = username.chars().count();
    if !(3..=64).contains(&length) {
        return Err(AppError::BadRequest("用户名长度需在 3 到 64 个字符之间。".to_owned()));
    }
    if !username
        .chars()
        .all(|ch| ch.is_alphanumeric() || matches!(ch, '.' | '_' | '-' | '@'))
    {
        return Err(AppError::BadRequest(
            "用户名只能包含字母、数字、点、下划线、短横线或 @。".to_owned(),
        ));
    }
    Ok(())
}

fn validate_password(password: &str) -> Result<(), AppError> {
    if password.chars().count() < 6 {
        return Err(AppError::BadRequest("密码至少需要 6 个字符。".to_owned()));
    }
    Ok(())
}

fn check_text_len(value: &str, limit: usize, message: &str) -> Result<(), AppError> {
    if value.chars().count() > limit {
        return Err(AppError::BadRequest(message.to_owned()));
    }
    Ok(())
}

async fn hash_password(password: &str) -> Result<String, AppError> {
    let password = password.to_owned();
    tokio::task::spawn_blocking(move || bcrypt::hash(password, bcrypt::DEFAULT_COST))
        .await
        .map_err(|err| AppError::Internal(format!("failed to hash password: {err}")))?
        .map_err(|err| AppError::Internal(format!("failed to hash password: {err}")))
}
