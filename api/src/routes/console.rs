use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde_json::{json, Value};

use crate::auth::AuthUser;
use crate::error::AppError;
use crate::models::{
    device_json, group_json, now_secs, parse_info, AssignRequest, GroupCreateRequest,
    GroupRow, GroupUpdateRequest, PageQuery,
};
use crate::AppState;

pub async fn list_devices(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(page): Query<PageQuery>,
) -> Result<Json<Value>, AppError> {
    let enabled_only = page.status == Some(1);
    let (total, rows) = super::client::query_devices(&state, &auth, &page, enabled_only).await?;
    let now = now_secs();
    let data: Vec<Value> = rows.iter().map(|row| device_json(row, now)).collect();
    Ok(Json(json!({ "total": total, "data": data })))
}

pub async fn list_groups(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(page): Query<PageQuery>,
) -> Result<Json<Value>, AppError> {
    let (total, rows) = super::client::query_groups(&state, &auth, &page).await?;
    let data: Vec<Value> = rows.iter().map(group_json).collect();
    Ok(Json(json!({ "total": total, "data": data })))
}

pub async fn assign_device(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(guid): Path<String>,
    payload: Result<Json<AssignRequest>, axum::extract::rejection::JsonRejection>,
) -> Result<Json<Value>, AppError> {
    let Json(request) =
        payload.map_err(|err| AppError::BadRequest(format!("请求格式错误：{err}")))?;
    let kind = request.kind.trim();
    let value = request.value.unwrap_or_default();

    if matches!(kind, "user_name" | "device_group_name") {
        require_admin(&auth)?;
    }

    let device: Option<(String, Option<String>)> =
        sqlx::query_as("SELECT id, user FROM devices WHERE guid = ?")
            .bind(&guid)
            .fetch_optional(&state.db)
            .await?;
    let (device_id, device_user) = device.ok_or_else(|| {
        AppError::NotFound("未找到设备。".to_owned())
    })?;
    if !auth.is_admin && device_user.as_deref() != Some(auth.username.as_str()) {
        return Err(AppError::Forbidden("无权修改此设备。".to_owned()));
    }

    let now = now_secs();
    match kind {
        "device_name" => {
            let name = value.trim();
            if name.is_empty() {
                return Err(AppError::BadRequest("设备名称不能为空。".to_owned()));
            }
            if name.chars().count() > 64 {
                return Err(AppError::BadRequest("设备名称过长。".to_owned()));
            }
            sqlx::query("UPDATE devices SET device_name = ?, updated_at = ? WHERE guid = ?")
                .bind(name)
                .bind(now)
                .bind(&guid)
                .execute(&state.db)
                .await?;
        }
        "note" => {
            if value.chars().count() > 240 {
                return Err(AppError::BadRequest("备注过长。".to_owned()));
            }
            sqlx::query("UPDATE devices SET note = ?, updated_at = ? WHERE guid = ?")
                .bind(&value)
                .bind(now)
                .bind(&guid)
                .execute(&state.db)
                .await?;
        }
        "user_name" => {
            let owner = value.trim();
            if owner.is_empty() {
                sqlx::query("UPDATE devices SET user = NULL, updated_at = ? WHERE guid = ?")
                    .bind(now)
                    .bind(&guid)
                    .execute(&state.db)
                    .await?;
                sqlx::query("DELETE FROM device_bindings WHERE device_id = ?")
                    .bind(&device_id)
                    .execute(&state.db)
                    .await?;
            } else {
                let exists: Option<i64> =
                    sqlx::query_scalar("SELECT 1 FROM users WHERE username = ?")
                        .bind(owner)
                        .fetch_optional(&state.db)
                        .await?;
                if exists.is_none() {
                    return Err(AppError::BadRequest("用户不存在。".to_owned()));
                }
                sqlx::query("UPDATE devices SET user = ?, updated_at = ? WHERE guid = ?")
                    .bind(owner)
                    .bind(now)
                    .bind(&guid)
                    .execute(&state.db)
                    .await?;
                sqlx::query(
                    "INSERT INTO device_bindings (device_id, uuid, username, updated_at)
                     VALUES (?, '', ?, ?)
                     ON CONFLICT(device_id) DO UPDATE SET
                        username = excluded.username,
                        updated_at = excluded.updated_at",
                )
                .bind(&device_id)
                .bind(owner)
                .bind(now)
                .execute(&state.db)
                .await?;
            }
        }
        "device_group_name" => {
            let name = value.trim();
            if name.is_empty() {
                sqlx::query(
                    "UPDATE devices SET device_group_guid = NULL, updated_at = ? WHERE guid = ?",
                )
                .bind(now)
                .bind(&guid)
                .execute(&state.db)
                .await?;
            } else {
                let group: Option<String> =
                    sqlx::query_scalar("SELECT guid FROM device_groups WHERE name = ?")
                        .bind(name)
                        .fetch_optional(&state.db)
                        .await?;
                let group = group
                    .ok_or_else(|| AppError::BadRequest("分组不存在。".to_owned()))?;
                sqlx::query(
                    "UPDATE devices SET device_group_guid = ?, updated_at = ? WHERE guid = ?",
                )
                .bind(group)
                .bind(now)
                .bind(&guid)
                .execute(&state.db)
                .await?;
            }
        }
        "device_username" => {
            let raw: String = sqlx::query_scalar("SELECT info FROM devices WHERE guid = ?")
                .bind(&guid)
                .fetch_one(&state.db)
                .await?;
            let mut info = parse_info(&raw);
            if let Value::Object(map) = &mut info {
                if value.is_empty() {
                    map.remove("username");
                } else {
                    map.insert("username".to_owned(), Value::String(value.clone()));
                }
            }
            sqlx::query("UPDATE devices SET info = ?, updated_at = ? WHERE guid = ?")
                .bind(info.to_string())
                .bind(now)
                .bind(&guid)
                .execute(&state.db)
                .await?;
        }
        _ => {
            return Err(AppError::BadRequest("不支持的分配类型。".to_owned()));
        }
    }

    super::audit(&state, &auth.username, "device.assign", &guid, kind).await;
    Ok(Json(json!({ "success": true })))
}

pub async fn disable_device(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(guid): Path<String>,
) -> Result<Json<Value>, AppError> {
    set_disabled(&state, &auth, &guid, true).await
}

pub async fn enable_device(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(guid): Path<String>,
) -> Result<Json<Value>, AppError> {
    set_disabled(&state, &auth, &guid, false).await
}

async fn set_disabled(
    state: &AppState,
    auth: &AuthUser,
    guid: &str,
    disabled: bool,
) -> Result<Json<Value>, AppError> {
    require_admin(auth)?;
    let result = sqlx::query("UPDATE devices SET disabled = ?, updated_at = ? WHERE guid = ?")
        .bind(if disabled { 1 } else { 0 })
        .bind(now_secs())
        .bind(guid)
        .execute(&state.db)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("未找到设备。".to_owned()));
    }
    let action = if disabled {
        "device.disable"
    } else {
        "device.enable"
    };
    super::audit(state, &auth.username, action, guid, "").await;
    Ok(Json(json!({ "success": true })))
}

pub async fn delete_device(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(guid): Path<String>,
) -> Result<Json<Value>, AppError> {
    require_admin(&auth)?;
    let device_id: Option<String> = sqlx::query_scalar("SELECT id FROM devices WHERE guid = ?")
        .bind(&guid)
        .fetch_optional(&state.db)
        .await?;
    let device_id =
        device_id.ok_or_else(|| AppError::NotFound("未找到设备。".to_owned()))?;

    let mut tx = state.db.begin().await?;
    sqlx::query("DELETE FROM device_bindings WHERE device_id = ?")
        .bind(&device_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM devices WHERE guid = ?")
        .bind(&guid)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    super::audit(&state, &auth.username, "device.delete", &guid, &device_id).await;
    Ok(Json(json!({ "success": true })))
}

pub async fn create_group(
    State(state): State<AppState>,
    auth: AuthUser,
    payload: Result<Json<GroupCreateRequest>, axum::extract::rejection::JsonRejection>,
) -> Result<Json<Value>, AppError> {
    require_admin(&auth)?;
    let Json(request) =
        payload.map_err(|err| AppError::BadRequest(format!("请求格式错误：{err}")))?;
    let name = request.name.trim();
    if name.is_empty() {
        return Err(AppError::BadRequest("分组名称不能为空。".to_owned()));
    }
    if name.chars().count() > 64 {
        return Err(AppError::BadRequest("分组名称过长。".to_owned()));
    }
    let note = request.note.unwrap_or_default();
    if note.chars().count() > 240 {
        return Err(AppError::BadRequest("备注过长。".to_owned()));
    }

    let guid = uuid::Uuid::new_v4().to_string();
    let now = now_secs();
    match sqlx::query(
        "INSERT INTO device_groups (guid, name, note, created_at, updated_at) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&guid)
    .bind(name)
    .bind(&note)
    .bind(now)
    .bind(now)
    .execute(&state.db)
    .await
    {
        Ok(_) => {}
        Err(sqlx::Error::Database(err)) if err.is_unique_violation() => {
            return Err(AppError::Conflict("同名分组已存在。".to_owned()));
        }
        Err(err) => return Err(err.into()),
    }

    super::audit(&state, &auth.username, "group.create", &guid, name).await;
    let row = GroupRow {
        guid,
        name: name.to_owned(),
        note,
        created_at: now,
        updated_at: now,
        device_count: 0,
    };
    Ok(Json(group_json(&row)))
}

pub async fn update_group(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(guid): Path<String>,
    payload: Result<Json<GroupUpdateRequest>, axum::extract::rejection::JsonRejection>,
) -> Result<Json<Value>, AppError> {
    require_admin(&auth)?;
    let Json(request) =
        payload.map_err(|err| AppError::BadRequest(format!("请求格式错误：{err}")))?;

    let current: Option<(String, String)> =
        sqlx::query_as("SELECT name, note FROM device_groups WHERE guid = ?")
            .bind(&guid)
            .fetch_optional(&state.db)
            .await?;
    let (current_name, current_note) =
        current.ok_or_else(|| AppError::NotFound("未找到分组。".to_owned()))?;

    let name = match request.name {
        Some(name) => {
            let name = name.trim();
            if name.is_empty() {
                return Err(AppError::BadRequest("分组名称不能为空。".to_owned()));
            }
            if name.chars().count() > 64 {
                return Err(AppError::BadRequest("分组名称过长。".to_owned()));
            }
            name.to_owned()
        }
        None => current_name,
    };
    let note = request.note.unwrap_or(current_note);
    if note.chars().count() > 240 {
        return Err(AppError::BadRequest("备注过长。".to_owned()));
    }

    let conflict: Option<String> =
        sqlx::query_scalar("SELECT guid FROM device_groups WHERE name = ? AND guid <> ?")
            .bind(&name)
            .bind(&guid)
            .fetch_optional(&state.db)
            .await?;
    if conflict.is_some() {
        return Err(AppError::Conflict("同名分组已存在。".to_owned()));
    }

    match sqlx::query(
        "UPDATE device_groups SET name = ?, note = ?, updated_at = ? WHERE guid = ?",
    )
    .bind(&name)
    .bind(&note)
    .bind(now_secs())
    .bind(&guid)
    .execute(&state.db)
    .await
    {
        Ok(_) => {}
        Err(sqlx::Error::Database(err)) if err.is_unique_violation() => {
            return Err(AppError::Conflict("同名分组已存在。".to_owned()));
        }
        Err(err) => return Err(err.into()),
    }

    super::audit(&state, &auth.username, "group.update", &guid, &name).await;
    Ok(Json(json!({ "success": true })))
}

pub async fn delete_group(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(guid): Path<String>,
) -> Result<Json<Value>, AppError> {
    require_admin(&auth)?;
    let now = now_secs();

    let mut tx = state.db.begin().await?;
    sqlx::query(
        "UPDATE devices SET device_group_guid = NULL, updated_at = ? WHERE device_group_guid = ?",
    )
    .bind(now)
    .bind(&guid)
    .execute(&mut *tx)
    .await?;
    sqlx::query("DELETE FROM user_device_groups WHERE group_guid = ?")
        .bind(&guid)
        .execute(&mut *tx)
        .await?;
    let deleted = sqlx::query("DELETE FROM device_groups WHERE guid = ?")
        .bind(&guid)
        .execute(&mut *tx)
        .await?;
    if deleted.rows_affected() == 0 {
        return Err(AppError::NotFound("未找到分组。".to_owned()));
    }
    tx.commit().await?;

    super::audit(&state, &auth.username, "group.delete", &guid, "").await;
    Ok(Json(json!({ "success": true })))
}

fn require_admin(auth: &AuthUser) -> Result<(), AppError> {
    if auth.is_admin {
        Ok(())
    } else {
        Err(AppError::Forbidden("需要管理员权限。".to_owned()))
    }
}
