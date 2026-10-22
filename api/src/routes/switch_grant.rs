use axum::{extract::State, Json};
use base64::Engine;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde_json::{json, Value};
use sqlx::{Connection, SqliteConnection};

use crate::error::AppError;
use crate::models::now_secs;
use crate::AppState;

const CLOCK_SKEW_SECS: i64 = 300;
const GRANT_TTL_SECS: i64 = 24 * 3600;

pub async fn register(
    State(state): State<AppState>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, AppError> {
    let id = body
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim();
    let verifier = body
        .get("switch_code_verifier")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim();
    let signature = body
        .get("signature")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim();
    let timestamp = body
        .get("timestamp")
        .map(|value| match value {
            Value::String(text) => text.trim().to_owned(),
            other => other.to_string(),
        })
        .unwrap_or_default();

    if id.is_empty() || verifier.is_empty() {
        return Ok(Json(
            json!({ "accepted": false, "error": "invalid switch grant request" }),
        ));
    }

    let now = now_secs();
    let ts: i64 = match timestamp.parse() {
        Ok(ts) => ts,
        Err(_) => {
            return Ok(Json(
                json!({ "accepted": false, "server_time": now, "error": "invalid timestamp" }),
            ))
        }
    };
    if (now - ts).abs() > CLOCK_SKEW_SECS {
        return Ok(Json(
            json!({ "accepted": false, "server_time": now, "error": "timestamp out of range" }),
        ));
    }

    let Some(public_key) = peer_public_key(&state, id).await else {
        return Ok(Json(
            json!({ "accepted": false, "error": "device public key not found" }),
        ));
    };
    let key_bytes: [u8; 32] = match public_key.len() {
        32 => public_key.try_into().unwrap_or([0u8; 32]),
        _ => {
            return Ok(Json(
                json!({ "accepted": false, "error": "invalid device public key" }),
            ))
        }
    };
    let verifying_key = match VerifyingKey::from_bytes(&key_bytes) {
        Ok(key) => key,
        Err(_) => {
            return Ok(Json(
                json!({ "accepted": false, "error": "invalid device public key" }),
            ))
        }
    };
    let sig_bytes = match base64::engine::general_purpose::STANDARD.decode(signature) {
        Ok(bytes) if bytes.len() == 64 => bytes,
        _ => {
            return Ok(Json(
                json!({ "accepted": false, "error": "invalid signature encoding" }),
            ))
        }
    };
    let signature = match Signature::from_slice(&sig_bytes) {
        Ok(signature) => signature,
        Err(_) => {
            return Ok(Json(
                json!({ "accepted": false, "error": "invalid signature" }),
            ))
        }
    };

    let mut message = Vec::with_capacity(13 + id.len() + 1 + verifier.len() + 1 + timestamp.len());
    message.extend_from_slice(b"switch-grant\0");
    message.extend_from_slice(id.as_bytes());
    message.push(0);
    message.extend_from_slice(verifier.as_bytes());
    message.push(0);
    message.extend_from_slice(timestamp.as_bytes());

    if verifying_key.verify(&message, &signature).is_err() {
        return Ok(Json(
            json!({ "accepted": false, "error": "signature verification failed" }),
        ));
    }

    sqlx::query(
        "INSERT INTO switch_grants (device_id, verifier, created_at, expires_at)
         VALUES (?, ?, ?, ?)
         ON CONFLICT(device_id, verifier) DO UPDATE SET
            created_at = excluded.created_at,
            expires_at = excluded.expires_at",
    )
    .bind(id)
    .bind(verifier)
    .bind(now)
    .bind(now + GRANT_TTL_SECS)
    .execute(&state.db)
    .await?;
    let _ = sqlx::query("DELETE FROM switch_grants WHERE expires_at < ?")
        .bind(now)
        .execute(&state.db)
        .await;

    Ok(Json(json!({ "accepted": true, "server_time": now })))
}

async fn peer_public_key(state: &AppState, id: &str) -> Option<Vec<u8>> {
    if let Some(path) = state.cfg.hbbs_db_path.as_ref() {
        if path.is_file() {
            let options = sqlx::sqlite::SqliteConnectOptions::new()
                .filename(path)
                .read_only(true);
            if let Ok(mut conn) = SqliteConnection::connect_with(&options).await {
                let key = sqlx::query_scalar::<_, Vec<u8>>(
                    "SELECT pk FROM peer WHERE id = ? LIMIT 1",
                )
                .bind(id)
                .fetch_optional(&mut conn)
                .await
                .ok()
                .flatten();
                let _ = conn.close().await;
                if let Some(key) = key.filter(|key| key.len() == 32) {
                    return Some(key);
                }
            }
        }
    }

    let info: Option<String> = sqlx::query_scalar("SELECT info FROM devices WHERE id = ? LIMIT 1")
        .bind(id)
        .fetch_optional(&state.db)
        .await
        .ok()
        .flatten();
    let info = info?;
    let value: Value = serde_json::from_str(&info).ok()?;
    let pk = value.get("pk").and_then(Value::as_str)?;
    base64::engine::general_purpose::STANDARD.decode(pk).ok()
}
