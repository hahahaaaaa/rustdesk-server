use axum::{
    extract::{rejection::JsonRejection, Path, State},
    http::{header, HeaderMap, StatusCode},
    Json,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::auth::AuthUser;
use crate::error::AppError;
use crate::models::now_secs;
use crate::AppState;

#[derive(Debug, Clone, Deserialize, Default)]
pub struct TokenCreateRequest {
    #[serde(default)]
    pub name: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct TokenUpdateRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct DeployRequest {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub uuid: String,
    #[serde(default)]
    pub pk: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct CliRequest {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub uuid: String,
    #[serde(default, alias = "userName")]
    pub user_name: Option<String>,
    #[serde(default, alias = "strategyName")]
    pub strategy_name: Option<String>,
    #[serde(default, alias = "addressBookName")]
    pub address_book_name: Option<String>,
    #[serde(default, alias = "tag")]
    pub tag: Option<String>,
    #[serde(default, alias = "alias")]
    pub alias: Option<String>,
    #[serde(default, alias = "password")]
    pub password: Option<String>,
    #[serde(default, alias = "note")]
    pub note: Option<String>,
    #[serde(default, alias = "deviceGroupName")]
    pub device_group_name: Option<String>,
    #[serde(default, alias = "deviceUsername")]
    pub device_username: Option<String>,
    #[serde(default, alias = "deviceName")]
    pub device_name: Option<String>,
}

pub async fn list_tokens(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, AppError> {
    require(&auth, "deployment.manage")?;
    let rows: Vec<(i64, String, String, i64, i64, Option<i64>)> = sqlx::query_as(
        "SELECT id, name, token, enabled, created_at, last_used_at
         FROM deployment_tokens ORDER BY id DESC",
    )
    .fetch_all(&state.db)
    .await?;
    let data: Vec<Value> = rows
        .into_iter()
        .map(|(id, name, token, enabled, created_at, last_used_at)| {
            json!({
                "id": id,
                "name": name,
                "token": token,
                "enabled": enabled != 0,
                "created_at": crate::models::iso_from_secs(Some(created_at)),
                "last_used_at": crate::models::iso_from_secs(last_used_at),
            })
        })
        .collect();
    Ok(Json(json!({ "data": data })))
}

pub async fn create_token(
    State(state): State<AppState>,
    auth: AuthUser,
    payload: Result<Json<TokenCreateRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<Value>), AppError> {
    require(&auth, "deployment.manage")?;
    let Json(request) = payload
        .map_err(|err| AppError::BadRequest(format!("请求格式错误：{err}")))?;
    let name = request.name.trim();
    if name.chars().count() > 64 {
        return Err(AppError::BadRequest("名称过长。".to_owned()));
    }
    let token = random_token();
    let now = now_secs();
    let id = sqlx::query(
        "INSERT INTO deployment_tokens (name, token, enabled, created_at) VALUES (?, ?, 1, ?)",
    )
    .bind(name)
    .bind(&token)
    .bind(now)
    .execute(&state.db)
    .await?
    .last_insert_rowid();
    super::audit(&state, &auth.username, "deployment.token_create", name, "").await;
    Ok((
        StatusCode::CREATED,
        Json(json!({
            "id": id,
            "name": name,
            "token": token,
            "enabled": true,
            "created_at": crate::models::iso_from_secs(Some(now)),
            "last_used_at": Value::Null,
        })),
    ))
}

pub async fn update_token(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    payload: Result<Json<TokenUpdateRequest>, JsonRejection>,
) -> Result<Json<Value>, AppError> {
    require(&auth, "deployment.manage")?;
    let Json(request) = payload
        .map_err(|err| AppError::BadRequest(format!("请求格式错误：{err}")))?;
    let exists: Option<i64> =
        sqlx::query_scalar("SELECT 1 FROM deployment_tokens WHERE id = ?")
            .bind(id)
            .fetch_optional(&state.db)
            .await?;
    if exists.is_none() {
        return Err(AppError::NotFound("部署令牌不存在。".to_owned()));
    }
    if let Some(name) = request.name.as_deref() {
        let name = name.trim();
        if name.chars().count() > 64 {
            return Err(AppError::BadRequest("名称过长。".to_owned()));
        }
        sqlx::query("UPDATE deployment_tokens SET name = ? WHERE id = ?")
            .bind(name)
            .bind(id)
            .execute(&state.db)
            .await?;
    }
    if let Some(enabled) = request.enabled {
        sqlx::query("UPDATE deployment_tokens SET enabled = ? WHERE id = ?")
            .bind(if enabled { 1 } else { 0 })
            .bind(id)
            .execute(&state.db)
            .await?;
    }
    super::audit(&state, &auth.username, "deployment.token_update", "", "").await;
    Ok(Json(json!({ "success": true })))
}

pub async fn delete_token(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    require(&auth, "deployment.manage")?;
    sqlx::query("DELETE FROM deployment_tokens WHERE id = ?")
        .bind(id)
        .execute(&state.db)
        .await?;
    super::audit(&state, &auth.username, "deployment.token_delete", "", "").await;
    Ok(Json(json!({ "success": true })))
}

pub async fn deploy(
    State(state): State<AppState>,
    headers: HeaderMap,
    payload: Result<Json<DeployRequest>, JsonRejection>,
) -> Result<Json<Value>, AppError> {
    let token = match bearer(&headers) {
        Some(token) => token,
        None => return Ok(Json(json!({ "result": "NOT_ENABLED" }))),
    };
    let Some(token_id) = enabled_token_id(&state, token).await? else {
        return Ok(Json(json!({ "result": "NOT_ENABLED" })));
    };
    let Json(request) = payload
        .map_err(|err| AppError::BadRequest(format!("请求格式错误：{err}")))?;
    let id = request.id.trim();
    let uuid = request.uuid.trim();
    if id.is_empty() || id.chars().count() > 100 || uuid.is_empty() {
        return Ok(Json(json!({ "result": "INVALID_INPUT" })));
    }
    let pk = match STANDARD.decode(request.pk.trim()) {
        Ok(pk) if pk.len() == 32 => pk,
        _ => return Ok(Json(json!({ "result": "INVALID_INPUT" }))),
    };
    let exists: Option<i64> = sqlx::query_scalar("SELECT 1 FROM devices WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db)
        .await?;
    if exists.is_some() {
        return Ok(Json(json!({ "result": "ID_TAKEN" })));
    }

    let now = now_secs();
    let guid = uuid::Uuid::new_v4().to_string();
    let info = json!({
        "id": id,
        "device_name": id,
        "pk": STANDARD.encode(pk),
    });
    sqlx::query(
        "INSERT INTO devices
            (guid, id, uuid, user, device_name, note, disabled, last_online, info, version, created_at, updated_at)
         VALUES (?, ?, ?, NULL, '', '', 0, NULL, ?, '', ?, ?)",
    )
    .bind(&guid)
    .bind(id)
    .bind(uuid)
    .bind(info.to_string())
    .bind(now)
    .bind(now)
    .execute(&state.db)
    .await?;
    sqlx::query("UPDATE deployment_tokens SET last_used_at = ? WHERE id = ?")
        .bind(now)
        .bind(token_id)
        .execute(&state.db)
        .await?;
    Ok(Json(json!({ "result": "OK" })))
}

pub async fn cli(
    State(state): State<AppState>,
    headers: HeaderMap,
    payload: Result<Json<CliRequest>, JsonRejection>,
) -> Result<StatusCode, AppError> {
    let token = bearer(&headers)
        .ok_or_else(|| AppError::Unauthorized("缺少部署令牌。".to_owned()))?;
    let Some(token_id) = enabled_token_id(&state, token).await? else {
        return Err(AppError::Unauthorized("部署令牌无效或已禁用。".to_owned()));
    };
    let Json(request) = payload
        .map_err(|err| AppError::BadRequest(format!("请求格式错误：{err}")))?;
    let id = request.id.trim();
    let uuid = request.uuid.trim();
    if id.is_empty() {
        return Err(AppError::BadRequest("设备 ID 不能为空。".to_owned()));
    }

    let now = now_secs();
    let mut guid: Option<String> = if uuid.is_empty() {
        sqlx::query_scalar("SELECT guid FROM devices WHERE id = ? LIMIT 1")
            .bind(id)
            .fetch_optional(&state.db)
            .await?
    } else {
        sqlx::query_scalar(
            "SELECT guid FROM devices WHERE uuid = ? OR id = ?
             ORDER BY CASE WHEN uuid = ? THEN 0 ELSE 1 END LIMIT 1",
        )
        .bind(uuid)
        .bind(id)
        .bind(uuid)
        .fetch_optional(&state.db)
        .await?
    };
    if guid.is_none() {
        let new_guid = uuid::Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO devices
                (guid, id, uuid, user, device_name, note, disabled, last_online, info, version, created_at, updated_at)
             VALUES (?, ?, ?, NULL, '', '', 0, NULL, '{}', '', ?, ?)",
        )
        .bind(&new_guid)
        .bind(id)
        .bind(if uuid.is_empty() { None } else { Some(uuid) })
        .bind(now)
        .bind(now)
        .execute(&state.db)
        .await?;
        guid = Some(new_guid);
    }
    let guid = guid.unwrap_or_default();

    if let Some(user_name) = request.user_name.as_deref() {
        let user_name = user_name.trim();
        if user_name.is_empty() {
            sqlx::query("UPDATE devices SET user = NULL, updated_at = ? WHERE guid = ?")
                .bind(now)
                .bind(&guid)
                .execute(&state.db)
                .await?;
        } else {
            let exists: Option<i64> =
                sqlx::query_scalar("SELECT 1 FROM users WHERE username = ?")
                    .bind(user_name)
                    .fetch_optional(&state.db)
                    .await?;
            if exists.is_none() {
                return Err(AppError::BadRequest("用户不存在。".to_owned()));
            }
            sqlx::query("UPDATE devices SET user = ?, updated_at = ? WHERE guid = ?")
                .bind(user_name)
                .bind(now)
                .bind(&guid)
                .execute(&state.db)
                .await?;
            sqlx::query(
                "INSERT INTO device_bindings (device_id, uuid, username, updated_at)
                 VALUES (?, ?, ?, ?)
                 ON CONFLICT(device_id) DO UPDATE SET
                    username = excluded.username,
                    uuid = excluded.uuid,
                    updated_at = excluded.updated_at",
            )
            .bind(id)
            .bind(if uuid.is_empty() { None } else { Some(uuid) })
            .bind(user_name)
            .bind(now)
            .execute(&state.db)
            .await?;
        }
    }

    if let Some(device_name) = request.device_name.as_deref() {
        let device_name = device_name.trim();
        if device_name.chars().count() > 64 {
            return Err(AppError::BadRequest("设备名称过长。".to_owned()));
        }
        sqlx::query("UPDATE devices SET device_name = ?, updated_at = ? WHERE guid = ?")
            .bind(device_name)
            .bind(now)
            .bind(&guid)
            .execute(&state.db)
            .await?;
    }

    if let Some(note) = request.note.as_deref() {
        let note = note.trim();
        if note.chars().count() > 240 {
            return Err(AppError::BadRequest("备注过长。".to_owned()));
        }
        sqlx::query("UPDATE devices SET note = ?, updated_at = ? WHERE guid = ?")
            .bind(note)
            .bind(now)
            .bind(&guid)
            .execute(&state.db)
            .await?;
    }

    if let Some(group_name) = request.device_group_name.as_deref() {
        let group_name = group_name.trim();
        if group_name.is_empty() {
            sqlx::query(
                "UPDATE devices SET device_group_guid = NULL, updated_at = ? WHERE guid = ?",
            )
            .bind(now)
            .bind(&guid)
            .execute(&state.db)
            .await?;
        } else {
            if group_name.chars().count() > 64 {
                return Err(AppError::BadRequest("分组名称过长。".to_owned()));
            }
            let group_guid: Option<String> =
                sqlx::query_scalar("SELECT guid FROM device_groups WHERE name = ? COLLATE NOCASE")
                    .bind(group_name)
                    .fetch_optional(&state.db)
                    .await?;
            let group_guid = match group_guid {
                Some(guid) => guid,
                None => {
                    let guid = uuid::Uuid::new_v4().to_string();
                    sqlx::query(
                        "INSERT INTO device_groups (guid, name, note, created_at, updated_at)
                         VALUES (?, ?, '', ?, ?)",
                    )
                    .bind(&guid)
                    .bind(group_name)
                    .bind(now)
                    .bind(now)
                    .execute(&state.db)
                    .await?;
                    guid
                }
            };
            sqlx::query(
                "UPDATE devices SET device_group_guid = ?, updated_at = ? WHERE guid = ?",
            )
            .bind(&group_guid)
            .bind(now)
            .bind(&guid)
            .execute(&state.db)
            .await?;
        }
    }

    if let Some(strategy_name) = request.strategy_name.as_deref() {
        let strategy_name = strategy_name.trim();
        if strategy_name.is_empty() {
            sqlx::query("DELETE FROM strategy_assignments WHERE kind = 'device' AND target_id = ?")
                .bind(&guid)
                .execute(&state.db)
                .await?;
        } else {
            let strategy_guid: Option<String> =
                sqlx::query_scalar("SELECT guid FROM strategies WHERE name = ? COLLATE NOCASE")
                    .bind(strategy_name)
                    .fetch_optional(&state.db)
                    .await?;
            let strategy_guid = strategy_guid
                .ok_or_else(|| AppError::BadRequest("策略不存在。".to_owned()))?;
            sqlx::query(
                "INSERT INTO strategy_assignments (strategy_guid, kind, target_id, created_at)
                 VALUES (?, 'device', ?, ?)
                 ON CONFLICT(kind, target_id) DO UPDATE SET
                    strategy_guid = excluded.strategy_guid,
                    created_at = excluded.created_at",
            )
            .bind(strategy_guid)
            .bind(&guid)
            .bind(now)
            .execute(&state.db)
            .await?;
        }
    }

    if let Some(book_name) = request.address_book_name.as_deref() {
        let book_name = book_name.trim();
        if book_name.is_empty() {
            sqlx::query(
                "DELETE FROM ab_peers WHERE peer_id = ? AND ab_guid IN
                    (SELECT guid FROM address_books WHERE is_personal = 0)",
            )
            .bind(id)
            .execute(&state.db)
            .await?;
        } else {
            let ab_guid: Option<String> = sqlx::query_scalar(
                "SELECT guid FROM address_books WHERE name = ? AND is_personal = 0",
            )
            .bind(book_name)
            .fetch_optional(&state.db)
            .await?;
            let ab_guid = ab_guid
                .ok_or_else(|| AppError::BadRequest("地址簿不存在。".to_owned()))?;
            let tags: Vec<String> = request
                .tag
                .as_deref()
                .unwrap_or_default()
                .split(',')
                .map(str::trim)
                .filter(|tag| !tag.is_empty())
                .map(ToOwned::to_owned)
                .collect();
            let tags = serde_json::to_string(&tags).unwrap_or_else(|_| "[]".to_owned());
            sqlx::query(
                "INSERT INTO ab_peers
                    (ab_guid, peer_id, hash, password, username, hostname, platform, alias, tags,
                     force_always_relay, rdp_port, rdp_username, login_name, device_group_name,
                     note, created_at, updated_at)
                 VALUES (?, ?, '', ?, ?, '', '', ?, ?, 0, '', '', ?, ?, ?, ?, ?)
                 ON CONFLICT(ab_guid, peer_id) DO UPDATE SET
                    password = excluded.password,
                    username = excluded.username,
                    alias = excluded.alias,
                    tags = excluded.tags,
                    login_name = excluded.login_name,
                    device_group_name = excluded.device_group_name,
                    note = excluded.note,
                    updated_at = excluded.updated_at",
            )
            .bind(&ab_guid)
            .bind(id)
            .bind(request.password.as_deref().unwrap_or_default())
            .bind(request.device_username.as_deref().unwrap_or_default())
            .bind(request.alias.as_deref().unwrap_or_default())
            .bind(tags)
            .bind(request.device_username.as_deref().unwrap_or_default())
            .bind(request.device_group_name.as_deref().unwrap_or_default())
            .bind(request.note.as_deref().unwrap_or_default())
            .bind(now)
            .bind(now)
            .execute(&state.db)
            .await?;
        }
    }

    sqlx::query("UPDATE deployment_tokens SET last_used_at = ? WHERE id = ?")
        .bind(now)
        .bind(token_id)
        .execute(&state.db)
        .await?;
    Ok(StatusCode::OK)
}

async fn enabled_token_id(state: &AppState, token: &str) -> Result<Option<i64>, AppError> {
    let id: Option<i64> = sqlx::query_scalar(
        "SELECT id FROM deployment_tokens WHERE token = ? AND enabled = 1",
    )
    .bind(token)
    .fetch_optional(&state.db)
    .await?;
    Ok(id)
}

fn bearer(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| {
            value
                .strip_prefix("Bearer ")
                .or_else(|| value.strip_prefix("bearer "))
        })
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn require(auth: &AuthUser, permission: &str) -> Result<(), AppError> {
    if auth.can(permission) {
        Ok(())
    } else {
        Err(AppError::Forbidden("没有权限执行此操作。".to_owned()))
    }
}

fn random_token() -> String {
    use rand::RngCore;
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}
