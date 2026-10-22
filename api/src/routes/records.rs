use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::Response,
    Json,
};
use bytes::Bytes;
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use tokio_util::io::ReaderStream;

use crate::auth::{self, AuthUser};
use crate::error::AppError;
use crate::models::{iso_from_secs, now_secs};
use crate::AppState;

const MAX_FILENAME_LEN: usize = 180;

#[derive(Debug, Clone, Deserialize, Default)]
pub struct UploadQuery {
    #[serde(default, rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub file: String,
    #[serde(default)]
    pub offset: Option<i64>,
    #[serde(default)]
    pub length: Option<i64>,
}

/// Unauthenticated endpoint used by the RustDesk client to upload session
/// recordings in chunks (`type=new|part|tail|remove`).
pub async fn upload(
    State(state): State<AppState>,
    Query(query): Query<UploadQuery>,
    body: Bytes,
) -> Result<Json<Value>, AppError> {
    let filename = sanitize_filename(&query.file)?;
    let dir = state.cfg.recordings_dir.clone();
    tokio::fs::create_dir_all(&dir)
        .await
        .map_err(|err| AppError::Internal(format!("cannot create recordings dir: {err}")))?;
    let path = dir.join(&filename);
    let now = now_secs();

    match query.kind.trim() {
        "new" => {
            tokio::fs::write(&path, b"")
                .await
                .map_err(|err| AppError::Internal(format!("cannot create recording: {err}")))?;
            sqlx::query(
                "INSERT INTO recordings (filename, size, complete, note, created_at, updated_at)
                 VALUES (?, 0, 0, '', ?, ?)
                 ON CONFLICT(filename) DO UPDATE SET
                    size = 0, complete = 0, updated_at = excluded.updated_at",
            )
            .bind(&filename)
            .bind(now)
            .bind(now)
            .execute(&state.db)
            .await?;
        }
        "part" | "tail" => {
            let offset = query.offset.unwrap_or(0).max(0);
            let mut file = tokio::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .open(&path)
                .await
                .map_err(|err| AppError::Internal(format!("cannot open recording: {err}")))?;
            if !body.is_empty() {
                file.seek(std::io::SeekFrom::Start(offset as u64))
                    .await
                    .map_err(|err| AppError::Internal(format!("cannot seek recording: {err}")))?;
                tokio::io::AsyncWriteExt::write_all(&mut file, &body)
                    .await
                    .map_err(|err| AppError::Internal(format!("cannot write recording: {err}")))?;
            }
            let size = file
                .metadata()
                .await
                .map(|meta| meta.len() as i64)
                .unwrap_or(offset + body.len() as i64);
            let complete = if query.kind.trim() == "tail" { 1 } else { 0 };
            sqlx::query(
                "INSERT INTO recordings (filename, size, complete, note, created_at, updated_at)
                 VALUES (?, ?, ?, '', ?, ?)
                 ON CONFLICT(filename) DO UPDATE SET
                    size = excluded.size,
                    complete = MAX(recordings.complete, excluded.complete),
                    updated_at = excluded.updated_at",
            )
            .bind(&filename)
            .bind(size)
            .bind(complete)
            .bind(now)
            .bind(now)
            .execute(&state.db)
            .await?;
        }
        "remove" => {
            let _ = tokio::fs::remove_file(&path).await;
            sqlx::query("DELETE FROM recordings WHERE filename = ?")
                .bind(&filename)
                .execute(&state.db)
                .await?;
        }
        other => {
            return Err(AppError::BadRequest(format!(
                "unknown record type: {other}"
            )))
        }
    }

    Ok(Json(json!({ "success": true })))
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RecordQuery {
    #[serde(default)]
    pub current: Option<i64>,
    #[serde(default)]
    pub page_size: Option<i64>,
    #[serde(default)]
    pub q: Option<String>,
    #[serde(default)]
    pub device: Option<String>,
    #[serde(default)]
    pub complete: Option<i64>,
}

#[derive(Debug, sqlx::FromRow)]
struct RecordingRow {
    filename: String,
    size: i64,
    complete: i64,
    note: String,
    created_at: i64,
    updated_at: i64,
}

pub async fn list(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<RecordQuery>,
) -> Result<Json<Value>, AppError> {
    require(&auth, "recording.read")?;
    let current = query.current.unwrap_or(1).max(1);
    let page_size = query.page_size.unwrap_or(50).clamp(1, 200);
    let offset = (current - 1) * page_size;

    let mut conditions = String::from(" WHERE 1=1");
    let q = query
        .q
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| format!("%{value}%"));
    let device = query
        .device
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| format!("%{value}%"));
    if q.is_some() {
        conditions.push_str(" AND (filename LIKE ? OR note LIKE ?)");
    }
    if device.is_some() {
        conditions.push_str(" AND filename LIKE ?");
    }
    if query.complete.is_some() {
        conditions.push_str(" AND complete = ?");
    }

    let count_sql = format!("SELECT COUNT(*) FROM recordings{conditions}");
    let mut count = sqlx::query_scalar::<_, i64>(&count_sql);
    if let Some(q) = &q {
        count = count.bind(q).bind(q);
    }
    if let Some(device) = &device {
        count = count.bind(device);
    }
    if let Some(complete) = query.complete {
        count = count.bind(complete);
    }
    let total = count.fetch_one(&state.db).await?;

    let list_sql = format!(
        "SELECT filename, size, complete, note, created_at, updated_at
         FROM recordings{conditions}
         ORDER BY created_at DESC, filename DESC LIMIT ? OFFSET ?"
    );
    let mut rows = sqlx::query_as::<_, RecordingRow>(&list_sql);
    if let Some(q) = &q {
        rows = rows.bind(q).bind(q);
    }
    if let Some(device) = &device {
        rows = rows.bind(device);
    }
    if let Some(complete) = query.complete {
        rows = rows.bind(complete);
    }
    let rows = rows
        .bind(page_size)
        .bind(offset)
        .fetch_all(&state.db)
        .await?;

    let data: Vec<Value> = rows
        .iter()
        .map(|row| {
            json!({
                "filename": row.filename,
                "device_id": filename_device(&row.filename),
                "size": row.size,
                "complete": row.complete != 0,
                "note": row.note,
                "created_at": iso_from_secs(Some(row.created_at)),
                "created_at_ts": row.created_at,
                "updated_at": iso_from_secs(Some(row.updated_at)),
                "url": format!("/api/recordings/{}/download", urlencoding::encode(&row.filename)),
            })
        })
        .collect();
    Ok(Json(
        json!({ "total": total, "data": data, "current": current, "pageSize": page_size }),
    ))
}

pub async fn update_note(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(filename): Path<String>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, AppError> {
    require(&auth, "recording.manage")?;
    let filename = sanitize_filename(&filename)?;
    let note = body
        .get("note")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim();
    if note.chars().count() > 240 {
        return Err(AppError::BadRequest("备注过长。".to_owned()));
    }
    sqlx::query("UPDATE recordings SET note = ?, updated_at = ? WHERE filename = ?")
        .bind(note)
        .bind(now_secs())
        .bind(&filename)
        .execute(&state.db)
        .await?;
    super::audit(&state, &auth.username, "recording.note", &filename, note).await;
    Ok(Json(json!({ "success": true })))
}

pub async fn delete(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(filename): Path<String>,
) -> Result<Json<Value>, AppError> {
    require(&auth, "recording.manage")?;
    let filename = sanitize_filename(&filename)?;
    let _ = tokio::fs::remove_file(state.cfg.recordings_dir.join(&filename)).await;
    sqlx::query("DELETE FROM recordings WHERE filename = ?")
        .bind(&filename)
        .execute(&state.db)
        .await?;
    super::audit(&state, &auth.username, "recording.delete", &filename, "").await;
    Ok(Json(json!({ "success": true })))
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct DownloadQuery {
    #[serde(default)]
    pub token: Option<String>,
}

pub async fn download(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<DownloadQuery>,
    Path(filename): Path<String>,
) -> Result<Response, AppError> {
    let token = query
        .token
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .or_else(|| {
            headers
                .get(header::AUTHORIZATION)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| {
                    value
                        .strip_prefix("Bearer ")
                        .or_else(|| value.strip_prefix("bearer "))
                        .map(str::to_owned)
                })
        })
        .ok_or_else(|| AppError::Unauthorized("缺少访问令牌。".to_owned()))?;
    let auth = auth::user_from_token(&state, &token).await?;
    require(&auth, "recording.read")?;

    let filename = sanitize_filename(&filename)?;
    let exists: Option<i64> = sqlx::query_scalar("SELECT 1 FROM recordings WHERE filename = ?")
        .bind(&filename)
        .fetch_optional(&state.db)
        .await?;
    if exists.is_none() {
        return Err(AppError::NotFound("录像不存在。".to_owned()));
    }

    let path = state.cfg.recordings_dir.join(&filename);
    let meta = tokio::fs::metadata(&path)
        .await
        .map_err(|_| AppError::NotFound("录像文件不存在。".to_owned()))?;
    let size = meta.len();
    let range = headers
        .get(header::RANGE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| parse_range(value, size));

    let (start, end, status) = match range {
        Some((start, end)) => (start, end, StatusCode::PARTIAL_CONTENT),
        None => (0, size.saturating_sub(1), StatusCode::OK),
    };
    let length = if size == 0 { 0 } else { end - start + 1 };

    let mut file = tokio::fs::File::open(&path)
        .await
        .map_err(|err| AppError::Internal(format!("cannot open recording: {err}")))?;
    if start > 0 {
        file.seek(std::io::SeekFrom::Start(start))
            .await
            .map_err(|err| AppError::Internal(format!("cannot seek recording: {err}")))?;
    }
    let reader = file.take(length);

    let mut response = Response::new(Body::from_stream(ReaderStream::new(reader)));
    *response.status_mut() = status;
    let out = response.headers_mut();
    out.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static(content_type(&filename)),
    );
    out.insert(header::ACCEPT_RANGES, HeaderValue::from_static("bytes"));
    out.insert(
        header::CONTENT_LENGTH,
        HeaderValue::from_str(&length.to_string()).unwrap_or(HeaderValue::from_static("0")),
    );
    out.insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_str(&format!("inline; filename=\"{filename}\""))
            .unwrap_or(HeaderValue::from_static("inline")),
    );
    if status == StatusCode::PARTIAL_CONTENT {
        out.insert(
            header::CONTENT_RANGE,
            HeaderValue::from_str(&format!("bytes {start}-{end}/{size}"))
                .unwrap_or(HeaderValue::from_static("bytes 0-0/0")),
        );
    }
    Ok(response)
}

fn parse_range(value: &str, size: u64) -> Option<(u64, u64)> {
    let value = value.trim().strip_prefix("bytes=")?;
    let (start, end) = value.split_once('-')?;
    if size == 0 {
        return None;
    }
    if start.is_empty() {
        let suffix: u64 = end.parse().ok()?;
        if suffix == 0 {
            return None;
        }
        let start = size.saturating_sub(suffix);
        return Some((start, size - 1));
    }
    let start: u64 = start.parse().ok()?;
    if start >= size {
        return None;
    }
    let end = if end.is_empty() {
        size - 1
    } else {
        end.parse::<u64>().ok()?.min(size - 1)
    };
    (start <= end).then_some((start, end))
}

fn content_type(filename: &str) -> &'static str {
    let lower = filename.to_ascii_lowercase();
    if lower.ends_with(".mp4") {
        "video/mp4"
    } else if lower.ends_with(".webm") {
        "video/webm"
    } else if lower.ends_with(".mkv") {
        "video/x-matroska"
    } else if lower.ends_with(".mov") {
        "video/quicktime"
    } else {
        "application/octet-stream"
    }
}

pub fn sanitize_filename(raw: &str) -> Result<String, AppError> {
    let base = raw
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or_default()
        .trim();
    if base.is_empty() || base == "." || base == ".." {
        return Err(AppError::BadRequest("录像文件名无效。".to_owned()));
    }
    let mut out = String::with_capacity(base.len().min(MAX_FILENAME_LEN));
    for ch in base.chars().take(MAX_FILENAME_LEN) {
        if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_' | ' ') {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    let trimmed = out.trim_matches('.').to_owned();
    if trimmed.is_empty()
        || trimmed == "."
        || trimmed == ".."
        || !trimmed.contains('.')
        || trimmed.starts_with('.')
    {
        return Err(AppError::BadRequest("录像文件名无效。".to_owned()));
    }
    Ok(trimmed)
}

fn filename_device(filename: &str) -> String {
    let mut parts = filename.split('_');
    match parts.next() {
        Some("incoming") | Some("outgoing") => parts.next().unwrap_or_default().to_owned(),
        _ => String::new(),
    }
}

fn require(auth: &AuthUser, permission: &str) -> Result<(), AppError> {
    if auth.can(permission) {
        Ok(())
    } else {
        Err(AppError::Forbidden("没有权限执行此操作。".to_owned()))
    }
}
