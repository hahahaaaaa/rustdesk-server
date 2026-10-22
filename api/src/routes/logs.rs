use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::{QueryBuilder, Sqlite};

use crate::auth::AuthUser;
use crate::error::AppError;
use crate::mailer;
use crate::models::{iso_from_secs, now_secs};
use crate::AppState;

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ConnPayload {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub uuid: String,
    #[serde(default)]
    pub ip: String,
    #[serde(default)]
    pub action: String,
    #[serde(default)]
    pub conn_audit_ref: String,
    #[serde(default)]
    pub peer: Vec<String>,
    #[serde(default, rename = "type")]
    pub conn_type: Option<i64>,
    #[serde(default)]
    pub primary_auth: Option<i64>,
    #[serde(default)]
    pub two_factor: Option<i64>,
    #[serde(default)]
    pub conn_id: Option<i64>,
    #[serde(default)]
    pub session_id: Option<i64>,
    #[serde(default)]
    pub nonce: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct FilePayload {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub uuid: String,
    #[serde(default)]
    pub peer_id: String,
    #[serde(default)]
    pub conn_id: Option<i64>,
    #[serde(default, rename = "type")]
    pub file_type: Option<i64>,
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub is_file: bool,
    #[serde(default)]
    pub info: String,
    #[serde(default)]
    pub nonce: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct AlarmPayload {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub uuid: String,
    #[serde(default, rename = "typ")]
    pub alarm_type: Option<i64>,
    #[serde(default)]
    pub info: String,
    #[serde(default)]
    pub conn_id: Option<i64>,
    #[serde(default)]
    pub conn_audit_ref: String,
    #[serde(default)]
    pub nonce: String,
}

pub async fn ingest_conn(
    State(state): State<AppState>,
    Json(payload): Json<ConnPayload>,
) -> Result<StatusCode, AppError> {
    if !claim_nonce(&state, &payload.nonce, "conn").await? {
        return Ok(StatusCode::OK);
    }
    let now = now_secs();
    let device_id = payload.id.trim();
    let action = if payload.action.trim().is_empty() {
        "new"
    } else {
        payload.action.trim()
    };

    if action == "close" {
        let updated = sqlx::query(
            "UPDATE conn_logs SET closed_at = ?
             WHERE id = (
                SELECT id FROM conn_logs
                WHERE device_id = ? AND closed_at IS NULL
                  AND (? IS NULL OR conn_id = ?)
                ORDER BY id DESC LIMIT 1
             )",
        )
        .bind(now)
        .bind(device_id)
        .bind(payload.conn_id)
        .bind(payload.conn_id)
        .execute(&state.db)
        .await?;
        if updated.rows_affected() == 0 {
            sqlx::query(
                "INSERT INTO conn_logs
                    (nonce, device_id, device_uuid, conn_id, session_id, ip, action,
                     conn_audit_ref, closed_at, created_at)
                 VALUES (?, ?, ?, ?, ?, ?, 'close', ?, ?, ?)",
            )
            .bind(nonce_or_new(&payload.nonce))
            .bind(device_id)
            .bind(payload.uuid.trim())
            .bind(payload.conn_id)
            .bind(payload.session_id)
            .bind(payload.ip.trim())
            .bind(payload.conn_audit_ref.trim())
            .bind(now)
            .bind(now)
            .execute(&state.db)
            .await?;
        }
        return Ok(StatusCode::OK);
    }

    let peer_id = payload.peer.first().cloned().unwrap_or_default();
    let peer_name = payload.peer.get(1).cloned().unwrap_or_default();

    if payload.conn_id.is_some() && (!peer_id.is_empty() || payload.conn_type.is_some()) {
        let updated = sqlx::query(
            "UPDATE conn_logs SET
                peer_id = CASE WHEN ? <> '' THEN ? ELSE peer_id END,
                peer_name = CASE WHEN ? <> '' THEN ? ELSE peer_name END,
                conn_type = COALESCE(?, conn_type),
                primary_auth = COALESCE(?, primary_auth),
                two_factor = COALESCE(?, two_factor),
                device_uuid = CASE WHEN device_uuid = '' THEN ? ELSE device_uuid END,
                ip = CASE WHEN ip = '' THEN ? ELSE ip END,
                conn_audit_ref = CASE WHEN conn_audit_ref = '' THEN ? ELSE conn_audit_ref END
             WHERE id = (
                SELECT id FROM conn_logs
                WHERE device_id = ? AND conn_id = ? AND closed_at IS NULL
                ORDER BY id DESC LIMIT 1
             )",
        )
        .bind(&peer_id)
        .bind(&peer_id)
        .bind(&peer_name)
        .bind(&peer_name)
        .bind(payload.conn_type)
        .bind(payload.primary_auth)
        .bind(payload.two_factor)
        .bind(payload.uuid.trim())
        .bind(payload.ip.trim())
        .bind(payload.conn_audit_ref.trim())
        .bind(device_id)
        .bind(payload.conn_id)
        .execute(&state.db)
        .await?;
        if updated.rows_affected() > 0 {
            return Ok(StatusCode::OK);
        }
    }

    sqlx::query(
        "INSERT INTO conn_logs
            (nonce, device_id, device_uuid, peer_id, peer_name, conn_id, session_id,
             conn_type, primary_auth, two_factor, ip, action, conn_audit_ref, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, COALESCE(?, 0), COALESCE(?, 0), COALESCE(?, 0),
                 ?, 'new', ?, ?)",
    )
    .bind(nonce_or_new(&payload.nonce))
    .bind(device_id)
    .bind(payload.uuid.trim())
    .bind(&peer_id)
    .bind(&peer_name)
    .bind(payload.conn_id)
    .bind(payload.session_id)
    .bind(payload.conn_type)
    .bind(payload.primary_auth)
    .bind(payload.two_factor)
    .bind(payload.ip.trim())
    .bind(payload.conn_audit_ref.trim())
    .bind(now)
    .execute(&state.db)
    .await?;
    Ok(StatusCode::OK)
}

pub async fn ingest_file(
    State(state): State<AppState>,
    Json(payload): Json<FilePayload>,
) -> Result<StatusCode, AppError> {
    if !claim_nonce(&state, &payload.nonce, "file").await? {
        return Ok(StatusCode::OK);
    }
    let info = if payload.info.trim().is_empty() {
        "{}"
    } else {
        payload.info.trim()
    };
    sqlx::query(
        "INSERT INTO file_logs
            (nonce, device_id, device_uuid, peer_id, conn_id, file_type, path, is_file, info, created_at)
         VALUES (?, ?, ?, ?, ?, COALESCE(?, 0), ?, ?, ?, ?)",
    )
    .bind(nonce_or_new(&payload.nonce))
    .bind(payload.id.trim())
    .bind(payload.uuid.trim())
    .bind(payload.peer_id.trim())
    .bind(payload.conn_id)
    .bind(payload.file_type)
    .bind(payload.path.trim())
    .bind(if payload.is_file { 1 } else { 0 })
    .bind(info)
    .bind(now_secs())
    .execute(&state.db)
    .await?;
    Ok(StatusCode::OK)
}

pub async fn ingest_alarm(
    State(state): State<AppState>,
    Json(payload): Json<AlarmPayload>,
) -> Result<StatusCode, AppError> {
    if !claim_nonce(&state, &payload.nonce, "alarm").await? {
        return Ok(StatusCode::OK);
    }
    let info = if payload.info.trim().is_empty() {
        "{}"
    } else {
        payload.info.trim()
    };
    let alarm_type = payload.alarm_type.unwrap_or(-1).clamp(-1, 32);
    sqlx::query(
        "INSERT INTO alarm_logs
            (nonce, device_id, device_uuid, alarm_type, info, conn_id, conn_audit_ref, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(nonce_or_new(&payload.nonce))
    .bind(payload.id.trim())
    .bind(payload.uuid.trim())
    .bind(alarm_type)
    .bind(info)
    .bind(payload.conn_id)
    .bind(payload.conn_audit_ref.trim())
    .bind(now_secs())
    .execute(&state.db)
    .await?;

    let state_clone = state.clone();
    let device_id = payload.id.trim().to_owned();
    let info_owned = info.to_owned();
    tokio::spawn(async move {
        mailer::notify_alarm(&state_clone, &device_id, alarm_type, &info_owned).await;
    });
    Ok(StatusCode::OK)
}

async fn claim_nonce(state: &AppState, nonce: &str, kind: &str) -> Result<bool, AppError> {
    let nonce = nonce_or_new(nonce);
    let result = sqlx::query(
        "INSERT OR IGNORE INTO audit_nonces (nonce, kind, created_at) VALUES (?, ?, ?)",
    )
    .bind(&nonce)
    .bind(kind)
    .bind(now_secs())
    .execute(&state.db)
    .await?;
    if result.rows_affected() == 0 {
        return Ok(false);
    }
    if rand::random::<u8>() == 0 {
        let _ = sqlx::query("DELETE FROM audit_nonces WHERE created_at < ?")
            .bind(now_secs() - 7 * 24 * 3600)
            .execute(&state.db)
            .await;
    }
    Ok(true)
}

fn nonce_or_new(nonce: &str) -> String {
    let nonce = nonce.trim();
    if nonce.is_empty() {
        uuid::Uuid::new_v4().to_string()
    } else {
        nonce.to_owned()
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ConnQuery {
    #[serde(default)]
    pub current: Option<i64>,
    #[serde(default)]
    pub page_size: Option<i64>,
    #[serde(default)]
    pub device: Option<String>,
    #[serde(default)]
    pub remote: Option<String>,
    #[serde(default)]
    pub conn_type: Option<i64>,
    #[serde(default)]
    pub q: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FileQuery {
    #[serde(default)]
    pub current: Option<i64>,
    #[serde(default)]
    pub page_size: Option<i64>,
    #[serde(default)]
    pub device: Option<String>,
    #[serde(default)]
    pub remote: Option<String>,
    #[serde(default)]
    pub q: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AlarmQuery {
    #[serde(default)]
    pub current: Option<i64>,
    #[serde(default)]
    pub page_size: Option<i64>,
    #[serde(default)]
    pub device: Option<String>,
    #[serde(default)]
    pub alarm_type: Option<i64>,
    #[serde(default)]
    pub q: Option<String>,
}

#[derive(Debug, sqlx::FromRow)]
struct ConnRow {
    id: i64,
    device_id: String,
    device_uuid: String,
    peer_id: String,
    peer_name: String,
    conn_id: Option<i64>,
    session_id: Option<i64>,
    conn_type: i64,
    primary_auth: i64,
    two_factor: i64,
    ip: String,
    action: String,
    conn_audit_ref: String,
    note: String,
    created_at: i64,
    closed_at: Option<i64>,
}

#[derive(Debug, sqlx::FromRow)]
struct FileRow {
    id: i64,
    device_id: String,
    device_uuid: String,
    peer_id: String,
    conn_id: Option<i64>,
    file_type: i64,
    path: String,
    is_file: i64,
    info: String,
    note: String,
    created_at: i64,
}

#[derive(Debug, sqlx::FromRow)]
struct AlarmRow {
    id: i64,
    device_id: String,
    device_uuid: String,
    alarm_type: i64,
    info: String,
    conn_id: Option<i64>,
    conn_audit_ref: String,
    note: String,
    created_at: i64,
}

pub async fn list_conn(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<ConnQuery>,
) -> Result<Json<Value>, AppError> {
    require(&auth, "audit.read")?;
    let (current, page_size) = page(&query.current, &query.page_size);
    let offset = (current - 1) * page_size;

    let mut count = QueryBuilder::new("SELECT COUNT(*) FROM conn_logs c WHERE 1=1");
    push_conn_filters(&mut count, &query);
    let total: i64 = count.build_query_scalar().fetch_one(&state.db).await?;

    let mut list = QueryBuilder::new(
        "SELECT c.id, c.device_id, c.device_uuid, c.peer_id, c.peer_name, c.conn_id,
                c.session_id, c.conn_type, c.primary_auth, c.two_factor, c.ip, c.action,
                c.conn_audit_ref, c.note, c.created_at, c.closed_at
         FROM conn_logs c WHERE 1=1",
    );
    push_conn_filters(&mut list, &query);
    list.push(" ORDER BY c.id DESC LIMIT ");
    list.push_bind(page_size);
    list.push(" OFFSET ");
    list.push_bind(offset);
    let rows = list.build_query_as::<ConnRow>().fetch_all(&state.db).await?;

    let data: Vec<Value> = rows
        .iter()
        .map(|row| {
            json!({
                "id": row.id,
                "device_id": row.device_id,
                "device_uuid": row.device_uuid,
                "peer_id": row.peer_id,
                "peer_name": row.peer_name,
                "conn_id": row.conn_id,
                "session_id": row.session_id,
                "conn_type": row.conn_type,
                "primary_auth": row.primary_auth,
                "two_factor": row.two_factor,
                "ip": row.ip,
                "action": row.action,
                "conn_audit_ref": row.conn_audit_ref,
                "note": row.note,
                "created_at": iso_from_secs(Some(row.created_at)),
                "created_at_ts": row.created_at,
                "closed_at": iso_from_secs(row.closed_at),
                "end_time": row.closed_at,
            })
        })
        .collect();
    Ok(Json(
        json!({ "total": total, "data": data, "current": current, "pageSize": page_size }),
    ))
}

pub async fn list_file(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<FileQuery>,
) -> Result<Json<Value>, AppError> {
    require(&auth, "audit.read")?;
    let (current, page_size) = page(&query.current, &query.page_size);
    let offset = (current - 1) * page_size;

    let mut count = QueryBuilder::new("SELECT COUNT(*) FROM file_logs f WHERE 1=1");
    push_file_filters(&mut count, &query);
    let total: i64 = count.build_query_scalar().fetch_one(&state.db).await?;

    let mut list = QueryBuilder::new(
        "SELECT f.id, f.device_id, f.device_uuid, f.peer_id, f.conn_id, f.file_type,
                f.path, f.is_file, f.info, f.note, f.created_at
         FROM file_logs f WHERE 1=1",
    );
    push_file_filters(&mut list, &query);
    list.push(" ORDER BY f.id DESC LIMIT ");
    list.push_bind(page_size);
    list.push(" OFFSET ");
    list.push_bind(offset);
    let rows = list.build_query_as::<FileRow>().fetch_all(&state.db).await?;

    let data: Vec<Value> = rows
        .iter()
        .map(|row| {
            json!({
                "id": row.id,
                "device_id": row.device_id,
                "device_uuid": row.device_uuid,
                "peer_id": row.peer_id,
                "conn_id": row.conn_id,
                "file_type": row.file_type,
                "path": row.path,
                "is_file": row.is_file != 0,
                "info": serde_json::from_str::<Value>(&row.info).unwrap_or(Value::Null),
                "info_raw": row.info,
                "note": row.note,
                "created_at": iso_from_secs(Some(row.created_at)),
                "created_at_ts": row.created_at,
            })
        })
        .collect();
    Ok(Json(
        json!({ "total": total, "data": data, "current": current, "pageSize": page_size }),
    ))
}

pub async fn list_alarm(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<AlarmQuery>,
) -> Result<Json<Value>, AppError> {
    require(&auth, "audit.read")?;
    let (current, page_size) = page(&query.current, &query.page_size);
    let offset = (current - 1) * page_size;

    let mut count = QueryBuilder::new("SELECT COUNT(*) FROM alarm_logs a WHERE 1=1");
    push_alarm_filters(&mut count, &query);
    let total: i64 = count.build_query_scalar().fetch_one(&state.db).await?;

    let mut list = QueryBuilder::new(
        "SELECT a.id, a.device_id, a.device_uuid, a.alarm_type, a.info, a.conn_id,
                a.conn_audit_ref, a.note, a.created_at
         FROM alarm_logs a WHERE 1=1",
    );
    push_alarm_filters(&mut list, &query);
    list.push(" ORDER BY a.id DESC LIMIT ");
    list.push_bind(page_size);
    list.push(" OFFSET ");
    list.push_bind(offset);
    let rows = list.build_query_as::<AlarmRow>().fetch_all(&state.db).await?;

    let data: Vec<Value> = rows
        .iter()
        .map(|row| {
            json!({
                "id": row.id,
                "device_id": row.device_id,
                "device_uuid": row.device_uuid,
                "alarm_type": row.alarm_type,
                "type_label": mailer::alarm_label(row.alarm_type),
                "info": serde_json::from_str::<Value>(&row.info).unwrap_or(Value::Null),
                "info_raw": row.info,
                "conn_id": row.conn_id,
                "conn_audit_ref": row.conn_audit_ref,
                "note": row.note,
                "created_at": iso_from_secs(Some(row.created_at)),
                "created_at_ts": row.created_at,
            })
        })
        .collect();
    Ok(Json(
        json!({ "total": total, "data": data, "current": current, "pageSize": page_size }),
    ))
}

fn push_conn_filters(qb: &mut QueryBuilder<'_, Sqlite>, query: &ConnQuery) {
    if let Some(device) = term(&query.device) {
        qb.push(" AND c.device_id LIKE ");
        qb.push_bind(device);
    }
    if let Some(remote) = term(&query.remote) {
        qb.push(" AND (c.peer_id LIKE ");
        qb.push_bind(remote.clone());
        qb.push(" OR c.peer_name LIKE ");
        qb.push_bind(remote);
        qb.push(")");
    }
    if let Some(conn_type) = query.conn_type {
        qb.push(" AND c.conn_type = ");
        qb.push_bind(conn_type);
    }
    if let Some(q) = term(&query.q) {
        qb.push(" AND (c.ip LIKE ");
        qb.push_bind(q.clone());
        qb.push(" OR c.peer_id LIKE ");
        qb.push_bind(q.clone());
        qb.push(" OR c.peer_name LIKE ");
        qb.push_bind(q.clone());
        qb.push(" OR c.device_id LIKE ");
        qb.push_bind(q);
        qb.push(")");
    }
}

fn push_file_filters(qb: &mut QueryBuilder<'_, Sqlite>, query: &FileQuery) {
    if let Some(device) = term(&query.device) {
        qb.push(" AND f.device_id LIKE ");
        qb.push_bind(device);
    }
    if let Some(remote) = term(&query.remote) {
        qb.push(" AND f.peer_id LIKE ");
        qb.push_bind(remote);
    }
    if let Some(q) = term(&query.q) {
        qb.push(" AND (f.path LIKE ");
        qb.push_bind(q.clone());
        qb.push(" OR f.info LIKE ");
        qb.push_bind(q.clone());
        qb.push(" OR f.device_id LIKE ");
        qb.push_bind(q.clone());
        qb.push(" OR f.peer_id LIKE ");
        qb.push_bind(q);
        qb.push(")");
    }
}

fn push_alarm_filters(qb: &mut QueryBuilder<'_, Sqlite>, query: &AlarmQuery) {
    if let Some(device) = term(&query.device) {
        qb.push(" AND a.device_id LIKE ");
        qb.push_bind(device);
    }
    if let Some(alarm_type) = query.alarm_type {
        qb.push(" AND a.alarm_type = ");
        qb.push_bind(alarm_type);
    }
    if let Some(q) = term(&query.q) {
        qb.push(" AND (a.info LIKE ");
        qb.push_bind(q.clone());
        qb.push(" OR a.device_id LIKE ");
        qb.push_bind(q);
        qb.push(")");
    }
}

pub async fn update_note(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((kind, id)): Path<(String, i64)>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, AppError> {
    require(&auth, "audit.read")?;
    let note = body
        .get("note")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim();
    if note.chars().count() > 240 {
        return Err(AppError::BadRequest("备注过长。".to_owned()));
    }
    let table = table_for(&kind)?;
    sqlx::query(&format!("UPDATE {table} SET note = ? WHERE id = ?"))
        .bind(note)
        .bind(id)
        .execute(&state.db)
        .await?;
    super::audit(&state, &auth.username, "audit.note", &kind, &id.to_string()).await;
    Ok(Json(json!({ "success": true })))
}

pub async fn delete_log(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((kind, id)): Path<(String, i64)>,
) -> Result<Json<Value>, AppError> {
    require(&auth, "audit.read")?;
    let table = table_for(&kind)?;
    sqlx::query(&format!("DELETE FROM {table} WHERE id = ?"))
        .bind(id)
        .execute(&state.db)
        .await?;
    super::audit(&state, &auth.username, "audit.delete", &kind, &id.to_string()).await;
    Ok(Json(json!({ "success": true })))
}

fn table_for(kind: &str) -> Result<&'static str, AppError> {
    match kind {
        "conn" => Ok("conn_logs"),
        "file" => Ok("file_logs"),
        "alarm" => Ok("alarm_logs"),
        _ => Err(AppError::BadRequest("未知的日志类型。".to_owned())),
    }
}

fn require(auth: &AuthUser, permission: &str) -> Result<(), AppError> {
    if auth.can(permission) {
        Ok(())
    } else {
        Err(AppError::Forbidden("没有权限执行此操作。".to_owned()))
    }
}

fn page(current: &Option<i64>, page_size: &Option<i64>) -> (i64, i64) {
    (
        current.unwrap_or(1).max(1),
        page_size.unwrap_or(50).clamp(1, 200),
    )
}

fn term(value: &Option<String>) -> Option<String> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| format!("%{value}%"))
}
