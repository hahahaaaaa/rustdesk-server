use axum::{
    extract::{Path, State},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::auth::AuthUser;
use crate::error::AppError;
use crate::models::{iso_from_secs, now_secs};
use crate::rbac;
use crate::AppState;

#[derive(Debug, Clone, Deserialize, Default)]
pub struct RoleRequest {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub note: String,
    #[serde(default)]
    pub permissions: Option<Vec<String>>,
}

#[derive(Debug, sqlx::FromRow)]
struct RoleRow {
    guid: String,
    name: String,
    note: String,
    permissions: String,
    created_at: i64,
    updated_at: i64,
    user_count: i64,
}

pub async fn list_permissions(auth: AuthUser) -> Json<Value> {
    let items: Vec<Value> = rbac::PERMISSIONS
        .iter()
        .map(|(key, label)| json!({ "key": key, "label": label }))
        .collect();
    let _ = auth;
    Json(json!({ "data": items }))
}

pub async fn list(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, AppError> {
    require(&auth, "role.manage")?;
    let rows = sqlx::query_as::<_, RoleRow>(
        "SELECT r.guid, r.name, r.note, r.permissions, r.created_at, r.updated_at,
                (SELECT COUNT(*) FROM user_roles ur WHERE ur.role_guid = r.guid) AS user_count
         FROM roles r ORDER BY r.name COLLATE NOCASE",
    )
    .fetch_all(&state.db)
    .await?;
    let data: Vec<Value> = rows.iter().map(role_json).collect();
    Ok(Json(json!({ "data": data })))
}

pub async fn create(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(body): Json<RoleRequest>,
) -> Result<Json<Value>, AppError> {
    require_admin(&auth)?;
    let name = body.name.trim();
    if name.is_empty() {
        return Err(AppError::BadRequest("角色名称不能为空。".to_owned()));
    }
    let exists: Option<i64> = sqlx::query_scalar("SELECT 1 FROM roles WHERE name = ?")
        .bind(name)
        .fetch_optional(&state.db)
        .await?;
    if exists.is_some() {
        return Err(AppError::Conflict("角色名称已存在。".to_owned()));
    }
    let permissions = normalize_permissions(body.permissions.unwrap_or_default());
    let guid = uuid::Uuid::new_v4().to_string();
    let now = now_secs();
    sqlx::query(
        "INSERT INTO roles (guid, name, note, permissions, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&guid)
    .bind(name)
    .bind(body.note.trim())
    .bind(serde_json::to_string(&permissions).unwrap_or_else(|_| "[]".to_owned()))
    .bind(now)
    .bind(now)
    .execute(&state.db)
    .await?;
    super::audit(&state, &auth.username, "role.create", &guid, name).await;
    Ok(Json(json!({ "guid": guid })))
}

pub async fn update(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(guid): Path<String>,
    Json(body): Json<RoleRequest>,
) -> Result<Json<Value>, AppError> {
    require_admin(&auth)?;
    let existing = sqlx::query_as::<_, RoleRow>(
        "SELECT r.guid, r.name, r.note, r.permissions, r.created_at, r.updated_at, 0 AS user_count
         FROM roles r WHERE r.guid = ?",
    )
    .bind(&guid)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound("角色不存在。".to_owned()))?;
    let name = if body.name.trim().is_empty() {
        existing.name
    } else {
        body.name.trim().to_owned()
    };
    let duplicate: Option<i64> =
        sqlx::query_scalar("SELECT 1 FROM roles WHERE name = ? AND guid <> ?")
            .bind(&name)
            .bind(&guid)
            .fetch_optional(&state.db)
            .await?;
    if duplicate.is_some() {
        return Err(AppError::Conflict("角色名称已存在。".to_owned()));
    }
    let permissions = body.permissions.map(normalize_permissions);
    let permissions_json = match permissions {
        Some(permissions) => serde_json::to_string(&permissions).unwrap_or_else(|_| "[]".to_owned()),
        None => existing.permissions,
    };
    sqlx::query(
        "UPDATE roles SET name = ?, note = ?, permissions = ?, updated_at = ? WHERE guid = ?",
    )
    .bind(&name)
    .bind(if body.note.trim().is_empty() {
        existing.note
    } else {
        body.note.trim().to_owned()
    })
    .bind(permissions_json)
    .bind(now_secs())
    .bind(&guid)
    .execute(&state.db)
    .await?;
    super::audit(&state, &auth.username, "role.update", &guid, &name).await;
    Ok(Json(json!({ "success": true })))
}

pub async fn delete(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(guid): Path<String>,
) -> Result<Json<Value>, AppError> {
    require_admin(&auth)?;
    sqlx::query("DELETE FROM roles WHERE guid = ?")
        .bind(&guid)
        .execute(&state.db)
        .await?;
    sqlx::query("DELETE FROM user_roles WHERE role_guid = ?")
        .bind(&guid)
        .execute(&state.db)
        .await?;
    super::audit(&state, &auth.username, "role.delete", &guid, "").await;
    Ok(Json(json!({ "success": true })))
}

fn role_json(row: &RoleRow) -> Value {
    let permissions: Vec<String> = serde_json::from_str(&row.permissions).unwrap_or_default();
    json!({
        "guid": row.guid,
        "name": row.name,
        "note": row.note,
        "permissions": permissions,
        "user_count": row.user_count,
        "created_at": iso_from_secs(Some(row.created_at)),
        "updated_at": iso_from_secs(Some(row.updated_at)),
    })
}

fn normalize_permissions(items: Vec<String>) -> Vec<String> {
    let mut out = Vec::new();
    for item in items {
        let item = item.trim();
        if rbac::PERMISSIONS.iter().any(|(key, _)| *key == item) && !out.iter().any(|v| v == item) {
            out.push(item.to_owned());
        }
    }
    out.sort();
    out
}

fn require(auth: &AuthUser, permission: &str) -> Result<(), AppError> {
    if auth.can(permission) {
        Ok(())
    } else {
        Err(AppError::Forbidden("没有权限执行此操作。".to_owned()))
    }
}

fn require_admin(auth: &AuthUser) -> Result<(), AppError> {
    if auth.is_admin {
        Ok(())
    } else {
        Err(AppError::Forbidden(
            "只有管理员可以创建、修改或删除角色。".to_owned(),
        ))
    }
}
