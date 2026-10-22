use axum::{
    extract::{Path, State},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use crate::auth::AuthUser;
use crate::error::AppError;
use crate::models::{iso_from_secs, now_secs};
use crate::AppState;

#[derive(Debug, sqlx::FromRow)]
struct RelayRow {
    guid: String,
    name: String,
    host: String,
    port: i64,
    note: String,
    enabled: i64,
    created_at: i64,
    updated_at: i64,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct RelayRequest {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub host: String,
    #[serde(default)]
    pub port: Option<i64>,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub enabled: Option<bool>,
}

pub async fn list(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, AppError> {
    require(&auth, "relay.manage")?;
    let rows = sqlx::query_as::<_, RelayRow>(
        "SELECT guid, name, host, port, note, enabled, created_at, updated_at
         FROM relay_servers ORDER BY name COLLATE NOCASE, host",
    )
    .fetch_all(&state.db)
    .await?;
    let data: Vec<Value> = rows.iter().map(relay_json).collect();
    Ok(Json(json!({
        "data": data,
        "builtin": state.cfg.relay_addr,
        "console": state.cfg.hbbs_console_addr(),
    })))
}

pub async fn create(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(body): Json<RelayRequest>,
) -> Result<Json<Value>, AppError> {
    require(&auth, "relay.manage")?;
    let (host, port) = validate(&body.host, body.port)?;
    let guid = uuid::Uuid::new_v4().to_string();
    let name = body.name.trim();
    let name = if name.is_empty() { &host } else { name };
    let now = now_secs();
    sqlx::query(
        "INSERT INTO relay_servers (guid, name, host, port, note, enabled, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&guid)
    .bind(name)
    .bind(&host)
    .bind(port)
    .bind(body.note.as_deref().unwrap_or_default().trim())
    .bind(if body.enabled.unwrap_or(true) { 1 } else { 0 })
    .bind(now)
    .bind(now)
    .execute(&state.db)
    .await?;
    let synced = sync_now(&state).await;
    super::audit(&state, &auth.username, "relay.create", &guid, name).await;
    Ok(Json(json!({ "guid": guid, "synced": synced })))
}

pub async fn update(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(guid): Path<String>,
    Json(body): Json<RelayRequest>,
) -> Result<Json<Value>, AppError> {
    require(&auth, "relay.manage")?;
    let existing = sqlx::query_as::<_, RelayRow>(
        "SELECT guid, name, host, port, note, enabled, created_at, updated_at
         FROM relay_servers WHERE guid = ?",
    )
    .bind(&guid)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound("中继服务器不存在。".to_owned()))?;

    let host_raw = if body.host.trim().is_empty() {
        existing.host.clone()
    } else {
        body.host.clone()
    };
    let (host, port) = validate(&host_raw, body.port.or(Some(existing.port)))?;
    let name = if body.name.trim().is_empty() {
        existing.name.clone()
    } else {
        body.name.trim().to_owned()
    };
    let note = body
        .note
        .clone()
        .unwrap_or_else(|| existing.note.clone());
    let enabled = body
        .enabled
        .map(|value| if value { 1 } else { 0 })
        .unwrap_or(existing.enabled);

    sqlx::query(
        "UPDATE relay_servers SET name = ?, host = ?, port = ?, note = ?, enabled = ?, updated_at = ?
         WHERE guid = ?",
    )
    .bind(&name)
    .bind(&host)
    .bind(port)
    .bind(note.trim())
    .bind(enabled)
    .bind(now_secs())
    .bind(&guid)
    .execute(&state.db)
    .await?;
    let synced = sync_now(&state).await;
    super::audit(&state, &auth.username, "relay.update", &guid, &name).await;
    Ok(Json(json!({ "success": true, "synced": synced })))
}

pub async fn delete(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(guid): Path<String>,
) -> Result<Json<Value>, AppError> {
    require(&auth, "relay.manage")?;
    sqlx::query("DELETE FROM relay_servers WHERE guid = ?")
        .bind(&guid)
        .execute(&state.db)
        .await?;
    let synced = sync_now(&state).await;
    super::audit(&state, &auth.username, "relay.delete", &guid, "").await;
    Ok(Json(json!({ "success": true, "synced": synced })))
}

pub async fn sync(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, AppError> {
    require(&auth, "relay.manage")?;
    match push_relays(&state).await {
        Ok(response) => Ok(Json(json!({ "success": true, "response": response }))),
        Err(err) => Ok(Json(json!({ "success": false, "error": err }))),
    }
}

pub fn spawn_sync(state: AppState) {
    tokio::spawn(async move {
        let mut first = true;
        loop {
            if !first {
                tokio::time::sleep(Duration::from_secs(300)).await;
            }
            first = false;
            let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM relay_servers WHERE enabled = 1")
                .fetch_one(&state.db)
                .await
                .unwrap_or(0);
            if count == 0 {
                continue;
            }
            match push_relays(&state).await {
                Ok(_) => tracing::info!("relay servers synchronized with hbbs"),
                Err(err) => tracing::warn!(error = %err, "failed to sync relay servers with hbbs"),
            }
        }
    });
}

async fn sync_now(state: &AppState) -> bool {
    match push_relays(state).await {
        Ok(_) => true,
        Err(err) => {
            tracing::warn!(error = %err, "failed to sync relay servers with hbbs");
            false
        }
    }
}

async fn push_relays(state: &AppState) -> Result<String, String> {
    let rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT host, port FROM relay_servers WHERE enabled = 1 ORDER BY name COLLATE NOCASE",
    )
    .fetch_all(&state.db)
    .await
    .map_err(|err| err.to_string())?;
    let servers: Vec<String> = rows
        .into_iter()
        .map(|(host, port)| format!("{host}:{port}"))
        .collect();
    let command = if servers.is_empty() {
        "relay-servers ,".to_owned()
    } else {
        format!("relay-servers {}", servers.join(","))
    };

    let addr = state.cfg.hbbs_console_addr();
    let mut stream = tokio::time::timeout(Duration::from_secs(3), TcpStream::connect(&addr))
        .await
        .map_err(|_| format!("连接 hbbs 控制台 {addr} 超时"))?
        .map_err(|err| format!("无法连接 hbbs 控制台 {addr}：{err}"))?;
    stream
        .write_all(command.as_bytes())
        .await
        .map_err(|err| format!("写入 hbbs 控制台失败：{err}"))?;
    let _ = stream.shutdown().await;
    let mut response = String::new();
    let _ = tokio::time::timeout(Duration::from_secs(2), stream.read_to_string(&mut response)).await;
    Ok(response.trim().to_owned())
}

fn relay_json(row: &RelayRow) -> Value {
    json!({
        "guid": row.guid,
        "name": row.name,
        "host": row.host,
        "port": row.port,
        "note": row.note,
        "enabled": row.enabled != 0,
        "created_at": iso_from_secs(Some(row.created_at)),
        "updated_at": iso_from_secs(Some(row.updated_at)),
    })
}

fn validate(host: &str, port: Option<i64>) -> Result<(String, i64), AppError> {
    let host = host.trim();
    if host.is_empty() {
        return Err(AppError::BadRequest("中继地址不能为空。".to_owned()));
    }
    if !host
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_'))
    {
        return Err(AppError::BadRequest(
            "中继地址只能包含字母、数字、点、横线和下划线。".to_owned(),
        ));
    }
    let port = port.unwrap_or(21117);
    if !(1..=65535).contains(&port) {
        return Err(AppError::BadRequest("中继端口无效。".to_owned()));
    }
    Ok((host.to_owned(), port))
}

fn require(auth: &AuthUser, permission: &str) -> Result<(), AppError> {
    if auth.can(permission) {
        Ok(())
    } else {
        Err(AppError::Forbidden("没有权限执行此操作。".to_owned()))
    }
}
