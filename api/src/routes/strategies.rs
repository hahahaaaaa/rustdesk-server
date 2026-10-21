use axum::{
    extract::{rejection::JsonRejection, Path, State},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Map, Value};

use crate::auth::AuthUser;
use crate::error::AppError;
use crate::models::{iso_from_secs, now_secs};
use crate::AppState;

const STRATEGY_COLUMNS: &str = "s.guid, s.name, s.note, s.enabled, s.config, s.created_at, s.updated_at,
    (SELECT COUNT(*) FROM strategy_assignments a WHERE a.strategy_guid = s.guid AND a.kind = 'device') AS device_count,
    (SELECT COUNT(*) FROM strategy_assignments a WHERE a.strategy_guid = s.guid AND a.kind = 'user') AS user_count,
    (SELECT COUNT(*) FROM strategy_assignments a WHERE a.strategy_guid = s.guid AND a.kind = 'group') AS group_count";

#[derive(Debug, Clone, sqlx::FromRow)]
struct StrategyRow {
    guid: String,
    name: String,
    note: String,
    enabled: i64,
    config: String,
    created_at: i64,
    updated_at: i64,
    device_count: i64,
    user_count: i64,
    group_count: i64,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct StrategyCreatePayload {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default, alias = "config_options")]
    pub config_options: Option<Map<String, Value>>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct StrategyUpdatePayload {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default, alias = "config_options")]
    pub config_options: Option<Map<String, Value>>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct AssignPayload {
    #[serde(default)]
    pub strategy: Option<String>,
    #[serde(default)]
    pub peers: Vec<String>,
    #[serde(default)]
    pub users: Vec<String>,
    #[serde(default)]
    pub groups: Vec<String>,
}

pub async fn list_strategies(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, AppError> {
    require_admin(&auth)?;
    let rows = sqlx::query_as::<_, StrategyRow>(&format!(
        "SELECT {STRATEGY_COLUMNS} FROM strategies s ORDER BY s.name COLLATE NOCASE"
    ))
    .fetch_all(&state.db)
    .await?;
    let data: Vec<Value> = rows.iter().map(strategy_json).collect();
    Ok(Json(Value::Array(data)))
}

pub async fn get_strategy(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(guid): Path<String>,
) -> Result<Json<Value>, AppError> {
    require_admin(&auth)?;
    let row = load_strategy(&state, &guid).await?;
    Ok(Json(strategy_json(&row)))
}

pub async fn create_strategy(
    State(state): State<AppState>,
    auth: AuthUser,
    payload: Result<Json<StrategyCreatePayload>, JsonRejection>,
) -> Result<Json<Value>, AppError> {
    require_admin(&auth)?;
    let Json(request) =
        payload.map_err(|err| AppError::BadRequest(format!("请求格式错误：{err}")))?;
    let name = request.name.trim().to_owned();
    validate_name(&name)?;
    let note = request.note.unwrap_or_default();
    validate_note(&note)?;
    let config = build_config(request.config_options.unwrap_or_default())?;
    let enabled = request.enabled.unwrap_or(true) as i64;
    let guid = uuid::Uuid::new_v4().to_string();
    let now = now_secs();

    match sqlx::query(
        "INSERT INTO strategies (guid, name, note, enabled, config, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&guid)
    .bind(&name)
    .bind(&note)
    .bind(enabled)
    .bind(&config)
    .bind(now)
    .bind(now)
    .execute(&state.db)
    .await
    {
        Ok(_) => {}
        Err(sqlx::Error::Database(err)) if err.is_unique_violation() => {
            return Err(AppError::Conflict("同名策略已存在。".to_owned()));
        }
        Err(err) => return Err(err.into()),
    }

    super::audit(
        &state,
        &auth.username,
        "strategy.create",
        &guid,
        &name,
    )
    .await;
    let row = load_strategy(&state, &guid).await?;
    Ok(Json(strategy_json(&row)))
}

pub async fn update_strategy(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(guid): Path<String>,
    payload: Result<Json<StrategyUpdatePayload>, JsonRejection>,
) -> Result<Json<Value>, AppError> {
    require_admin(&auth)?;
    let existing = load_strategy(&state, &guid).await?;
    let Json(request) =
        payload.map_err(|err| AppError::BadRequest(format!("请求格式错误：{err}")))?;

    let name = match request.name {
        Some(name) => {
            let name = name.trim().to_owned();
            validate_name(&name)?;
            name
        }
        None => existing.name.clone(),
    };
    let note = request.note.unwrap_or_else(|| existing.note.clone());
    validate_note(&note)?;
    let enabled = request
        .enabled
        .map(|value| value as i64)
        .unwrap_or(existing.enabled);
    let config = match request.config_options {
        Some(options) => build_config(options)?,
        None => existing.config.clone(),
    };
    let updated_at = next_revision(existing.updated_at);

    match sqlx::query(
        "UPDATE strategies
         SET name = ?, note = ?, enabled = ?, config = ?, updated_at = ?
         WHERE guid = ?",
    )
    .bind(&name)
    .bind(&note)
    .bind(enabled)
    .bind(&config)
    .bind(updated_at)
    .bind(&guid)
    .execute(&state.db)
    .await
    {
        Ok(_) => {}
        Err(sqlx::Error::Database(err)) if err.is_unique_violation() => {
            return Err(AppError::Conflict("同名策略已存在。".to_owned()));
        }
        Err(err) => return Err(err.into()),
    }

    super::audit(
        &state,
        &auth.username,
        "strategy.update",
        &guid,
        &name,
    )
    .await;
    let row = load_strategy(&state, &guid).await?;
    Ok(Json(strategy_json(&row)))
}

pub async fn set_status(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(guid): Path<String>,
    payload: Result<Json<Value>, JsonRejection>,
) -> Result<Json<Value>, AppError> {
    require_admin(&auth)?;
    let existing = load_strategy(&state, &guid).await?;
    let Json(body) =
        payload.map_err(|err| AppError::BadRequest(format!("请求格式错误：{err}")))?;
    let enabled = match body {
        Value::Bool(value) => value,
        Value::Object(map) => map
            .get("enabled")
            .and_then(Value::as_bool)
            .ok_or_else(|| AppError::BadRequest("请求需包含 enabled 布尔值。".to_owned()))?,
        _ => return Err(AppError::BadRequest("请求需包含 enabled 布尔值。".to_owned())),
    };
    let updated_at = next_revision(existing.updated_at);
    sqlx::query("UPDATE strategies SET enabled = ?, updated_at = ? WHERE guid = ?")
        .bind(enabled as i64)
        .bind(updated_at)
        .bind(&guid)
        .execute(&state.db)
        .await?;
    super::audit(
        &state,
        &auth.username,
        "strategy.status",
        &guid,
        if enabled { "enabled" } else { "disabled" },
    )
    .await;
    Ok(Json(json!({ "success": true, "enabled": enabled })))
}

pub async fn delete_strategy(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(guid): Path<String>,
) -> Result<Json<Value>, AppError> {
    require_admin(&auth)?;
    let existing = load_strategy(&state, &guid).await?;
    sqlx::query("DELETE FROM strategies WHERE guid = ?")
        .bind(&guid)
        .execute(&state.db)
        .await?;
    super::audit(
        &state,
        &auth.username,
        "strategy.delete",
        &guid,
        &existing.name,
    )
    .await;
    Ok(Json(json!({ "success": true })))
}

pub async fn get_assignments(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(guid): Path<String>,
) -> Result<Json<Value>, AppError> {
    require_admin(&auth)?;
    load_strategy(&state, &guid).await?;

    let devices: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT d.guid, d.id, d.device_name
         FROM strategy_assignments a
         JOIN devices d ON d.guid = a.target_id
         WHERE a.strategy_guid = ? AND a.kind = 'device'
         ORDER BY d.id",
    )
    .bind(&guid)
    .fetch_all(&state.db)
    .await?;
    let users: Vec<(String, String)> = sqlx::query_as(
        "SELECT u.username, u.display_name
         FROM strategy_assignments a
         JOIN users u ON u.username = a.target_id
         WHERE a.strategy_guid = ? AND a.kind = 'user'
         ORDER BY u.username COLLATE NOCASE",
    )
    .bind(&guid)
    .fetch_all(&state.db)
    .await?;
    let groups: Vec<(String, String)> = sqlx::query_as(
        "SELECT g.guid, g.name
         FROM strategy_assignments a
         JOIN device_groups g ON g.guid = a.target_id
         WHERE a.strategy_guid = ? AND a.kind = 'group'
         ORDER BY g.name COLLATE NOCASE",
    )
    .bind(&guid)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(json!({
        "devices": devices.iter().map(|(guid, id, name)| json!({
            "guid": guid,
            "id": id,
            "device_name": name,
        })).collect::<Vec<_>>(),
        "users": users.iter().map(|(name, display_name)| json!({
            "name": name,
            "display_name": display_name,
        })).collect::<Vec<_>>(),
        "groups": groups.iter().map(|(group_guid, name)| json!({
            "guid": group_guid,
            "name": name,
        })).collect::<Vec<_>>(),
    })))
}

pub async fn assign(
    State(state): State<AppState>,
    auth: AuthUser,
    payload: Result<Json<AssignPayload>, JsonRejection>,
) -> Result<Json<Value>, AppError> {
    require_admin(&auth)?;
    let Json(request) =
        payload.map_err(|err| AppError::BadRequest(format!("请求格式错误：{err}")))?;

    let strategy = match request
        .strategy
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        Some(guid) => {
            let name: Option<String> =
                sqlx::query_scalar("SELECT name FROM strategies WHERE guid = ?")
                    .bind(guid)
                    .fetch_optional(&state.db)
                    .await?;
            name.ok_or_else(|| AppError::NotFound("未找到策略。".to_owned()))?;
            Some(guid.to_owned())
        }
        None => None,
    };

    let mut peers = Vec::new();
    for target in &request.peers {
        let guid = resolve_device(&state, target).await?;
        if !peers.contains(&guid) {
            peers.push(guid);
        }
    }
    let mut users = Vec::new();
    for target in &request.users {
        let username = resolve_user(&state, target).await?;
        if !users.contains(&username) {
            users.push(username);
        }
    }
    let mut groups = Vec::new();
    for target in &request.groups {
        let guid = resolve_group(&state, target).await?;
        if !groups.contains(&guid) {
            groups.push(guid);
        }
    }

    let counts = (peers.len(), users.len(), groups.len());
    let now = now_secs();
    let mut tx = state.db.begin().await?;
    for (kind, targets) in [
        ("device", peers.as_slice()),
        ("user", users.as_slice()),
        ("group", groups.as_slice()),
    ] {
        for target in targets {
            match strategy.as_deref() {
                Some(strategy_guid) => {
                    sqlx::query(
                        "INSERT INTO strategy_assignments (strategy_guid, kind, target_id, created_at)
                         VALUES (?, ?, ?, ?)
                         ON CONFLICT(kind, target_id) DO UPDATE SET
                            strategy_guid = excluded.strategy_guid,
                            created_at = excluded.created_at",
                    )
                    .bind(strategy_guid)
                    .bind(kind)
                    .bind(target)
                    .bind(now)
                    .execute(&mut *tx)
                    .await?;
                }
                None => {
                    sqlx::query(
                        "DELETE FROM strategy_assignments WHERE kind = ? AND target_id = ?",
                    )
                    .bind(kind)
                    .bind(target)
                    .execute(&mut *tx)
                    .await?;
                }
            }
        }
    }
    tx.commit().await?;

    let detail = format!(
        "peers={},users={},groups={}",
        counts.0, counts.1, counts.2
    );
    super::audit(
        &state,
        &auth.username,
        "strategy.assign",
        strategy.as_deref().unwrap_or(""),
        &detail,
    )
    .await;

    Ok(Json(json!({
        "success": true,
        "assigned": {
            "peers": counts.0,
            "users": counts.1,
            "groups": counts.2,
        }
    })))
}

async fn resolve_device(state: &AppState, target: &str) -> Result<String, AppError> {
    let target = target.trim();
    if target.is_empty() {
        return Err(AppError::BadRequest("设备不能为空。".to_owned()));
    }
    let guid: Option<String> = sqlx::query_scalar(
        "SELECT guid FROM devices
         WHERE guid = ? OR id = ?
         ORDER BY CASE WHEN guid = ? THEN 0 ELSE 1 END
         LIMIT 1",
    )
    .bind(target)
    .bind(target)
    .bind(target)
    .fetch_optional(&state.db)
    .await?;
    guid.ok_or_else(|| AppError::BadRequest(format!("设备“{target}”不存在。")))
}

async fn resolve_user(state: &AppState, target: &str) -> Result<String, AppError> {
    let target = target.trim();
    if target.is_empty() {
        return Err(AppError::BadRequest("用户不能为空。".to_owned()));
    }
    let username: Option<String> = match target.parse::<i64>() {
        Ok(id) => {
            sqlx::query_scalar("SELECT username FROM users WHERE id = ?")
                .bind(id)
                .fetch_optional(&state.db)
                .await?
        }
        Err(_) => {
            sqlx::query_scalar("SELECT username FROM users WHERE username = ?")
                .bind(target)
                .fetch_optional(&state.db)
                .await?
        }
    };
    username.ok_or_else(|| AppError::BadRequest(format!("用户“{target}”不存在。")))
}

async fn resolve_group(state: &AppState, target: &str) -> Result<String, AppError> {
    let target = target.trim();
    if target.is_empty() {
        return Err(AppError::BadRequest("设备分组不能为空。".to_owned()));
    }
    let guid: Option<String> = sqlx::query_scalar(
        "SELECT guid FROM device_groups
         WHERE guid = ? OR name = ? COLLATE NOCASE
         LIMIT 1",
    )
    .bind(target)
    .bind(target)
    .fetch_optional(&state.db)
    .await?;
    guid.ok_or_else(|| AppError::BadRequest(format!("设备分组“{target}”不存在。")))
}

async fn load_strategy(state: &AppState, guid: &str) -> Result<StrategyRow, AppError> {
    sqlx::query_as::<_, StrategyRow>(&format!(
        "SELECT {STRATEGY_COLUMNS} FROM strategies s WHERE s.guid = ?"
    ))
    .bind(guid)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound("未找到策略。".to_owned()))
}

fn strategy_json(row: &StrategyRow) -> Value {
    json!({
        "guid": row.guid,
        "name": row.name,
        "note": row.note,
        "enabled": row.enabled != 0,
        "config_options": parse_config(&row.config),
        "created_at": iso_from_secs(Some(row.created_at)),
        "updated_at": iso_from_secs(Some(row.updated_at)),
        "modified_at": row.updated_at,
        "device_count": row.device_count,
        "user_count": row.user_count,
        "group_count": row.group_count,
    })
}

pub fn parse_config(raw: &str) -> Value {
    serde_json::from_str::<Value>(raw)
        .ok()
        .filter(Value::is_object)
        .unwrap_or_else(|| Value::Object(Map::new()))
}

fn build_config(options: Map<String, Value>) -> Result<String, AppError> {
    if options.len() > 200 {
        return Err(AppError::BadRequest("策略配置项过多。".to_owned()));
    }
    let mut normalized = Map::new();
    for (key, value) in options {
        let key = key.trim().to_owned();
        if key.is_empty() {
            return Err(AppError::BadRequest("策略配置的键不能为空。".to_owned()));
        }
        if key.chars().count() > 128 {
            return Err(AppError::BadRequest(format!(
                "策略配置键“{key}”过长。"
            )));
        }
        let value = value.as_str().ok_or_else(|| {
            AppError::BadRequest(format!("策略配置“{key}”的值必须是字符串。"))
        })?;
        if value.chars().count() > 1024 {
            return Err(AppError::BadRequest(format!(
                "策略配置“{key}”的值过长。"
            )));
        }
        normalized.insert(key, Value::String(value.to_owned()));
    }
    Ok(Value::Object(normalized).to_string())
}

fn validate_name(name: &str) -> Result<(), AppError> {
    if name.is_empty() {
        return Err(AppError::BadRequest("策略名称不能为空。".to_owned()));
    }
    if name.chars().count() > 48 {
        return Err(AppError::BadRequest("策略名称过长。".to_owned()));
    }
    Ok(())
}

fn validate_note(note: &str) -> Result<(), AppError> {
    if note.chars().count() > 240 {
        return Err(AppError::BadRequest("策略备注过长。".to_owned()));
    }
    Ok(())
}

fn next_revision(previous: i64) -> i64 {
    let now = now_secs();
    if now <= previous {
        previous + 1
    } else {
        now
    }
}

fn require_admin(auth: &AuthUser) -> Result<(), AppError> {
    if auth.is_admin {
        Ok(())
    } else {
        Err(AppError::Forbidden("需要管理员权限。".to_owned()))
    }
}
