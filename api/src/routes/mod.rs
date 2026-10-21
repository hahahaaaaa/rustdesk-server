mod auth;
mod client;
mod console;

use axum::{
    http::{HeaderValue, StatusCode},
    routing::{delete, get, patch, post},
    Json, Router,
};
use serde_json::json;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;

use crate::config::Config;
use crate::error::AppError;
use crate::models::now_secs;
use crate::AppState;

pub fn router(state: AppState) -> Router {
    let cors = build_cors(&state.cfg);
    let api = Router::new()
        .route("/api/health", get(health))
        .route("/api/login", post(auth::login))
        .route("/api/logout", post(auth::logout))
        .route("/api/currentUser", post(auth::current_user))
        .route("/api/login-options", get(auth::login_options))
        .route("/api/heartbeat", post(client::heartbeat))
        .route("/api/sysinfo", post(client::sysinfo))
        .route(
            "/api/sysinfo_ver",
            post(client::sysinfo_ver).get(client::sysinfo_ver),
        )
        .route("/api/users", get(client::list_users))
        .route("/api/peers", get(client::list_peers))
        .route(
            "/api/device-group/accessible",
            get(client::accessible_groups),
        )
        .route("/api/devices", get(console::list_devices))
        .route(
            "/api/devices/{guid}/assign",
            post(console::assign_device),
        )
        .route(
            "/api/devices/{guid}/disable",
            post(console::disable_device),
        )
        .route("/api/devices/{guid}/enable", post(console::enable_device))
        .route("/api/devices/{guid}", delete(console::delete_device))
        .route(
            "/api/device-groups",
            get(console::list_groups).post(console::create_group),
        )
        .route(
            "/api/device-groups/{guid}",
            patch(console::update_group).delete(console::delete_group),
        );

    let app = Router::new()
        .merge(api)
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state.clone());

    match state.cfg.console_dir.as_deref() {
        Some(dir) => {
            let index = format!("{dir}/index.html");
            app.fallback_service(
                ServeDir::new(dir).not_found_service(ServeFile::new(index)),
            )
        }
        None => app.fallback(get(index_info)),
    }
}

async fn health() -> Json<serde_json::Value> {
    Json(json!({
        "status": "ok",
        "service": "rustdesk-api",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

async fn index_info() -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::NOT_FOUND,
        Json(json!({
            "service": "rustdesk-api",
            "message": "Web console bundle not found. Build web-console and set API_CONSOLE_DIR.",
        })),
    )
}

fn build_cors(cfg: &Config) -> CorsLayer {
    let layer = CorsLayer::new().allow_methods(Any).allow_headers(Any);
    if cfg.allow_any_origin() {
        return layer.allow_origin(Any);
    }

    let origins: Vec<HeaderValue> = cfg
        .cors_origins
        .iter()
        .filter_map(|origin| match HeaderValue::from_str(origin) {
            Ok(value) => Some(value),
            Err(err) => {
                tracing::warn!(origin = %origin, error = %err, "ignoring invalid CORS origin");
                None
            }
        })
        .collect();
    if origins.is_empty() {
        tracing::warn!("no valid CORS origin configured, allowing any origin");
        layer.allow_origin(Any)
    } else {
        layer.allow_origin(origins)
    }
}

async fn bind_device(
    state: &AppState,
    device_id: &str,
    device_uuid: &str,
    username: &str,
) -> Result<(), AppError> {
    let now = now_secs();
    sqlx::query(
        "INSERT INTO device_bindings (device_id, uuid, username, updated_at)
         VALUES (?, ?, ?, ?)
         ON CONFLICT(device_id) DO UPDATE SET
            uuid = excluded.uuid,
            username = excluded.username,
            updated_at = excluded.updated_at",
    )
    .bind(device_id)
    .bind(device_uuid)
    .bind(username)
    .bind(now)
    .execute(&state.db)
    .await?;
    sqlx::query("UPDATE devices SET user = ?, updated_at = ? WHERE id = ?")
        .bind(username)
        .bind(now)
        .bind(device_id)
        .execute(&state.db)
        .await?;
    Ok(())
}

async fn audit(state: &AppState, actor: &str, action: &str, target: &str, detail: &str) {
    let result = sqlx::query(
        "INSERT INTO audit_logs (actor, action, target, detail, created_at) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(actor)
    .bind(action)
    .bind(target)
    .bind(detail)
    .bind(now_secs())
    .execute(&state.db)
    .await;
    if let Err(err) = result {
        tracing::warn!(error = %err, "failed to write audit log");
    }
}
