mod address_book;
mod audit;
mod auth;
mod client;
mod console;
mod deploy;
mod logs;
mod oidc;
mod records;
mod relays;
mod roles;
mod settings_ext;
mod strategies;
mod switch_grant;
mod tfa;
mod users;
mod webcontrol;

use axum::{
    extract::DefaultBodyLimit,
    http::{HeaderValue, StatusCode},
    routing::{delete, get, patch, post, put},
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

pub fn spawn_background_tasks(state: AppState) {
    relays::spawn_sync(state);
}

pub fn router(state: AppState) -> Router {
    let cors = build_cors(&state.cfg);
    let api = Router::new()
        .route("/api/health", get(health))
        .route("/api/oidc/status", get(oidc::status))
        .route("/api/oidc/auth", post(oidc::auth_start))
        .route("/api/oidc/auth-query", get(oidc::auth_query))
        .route("/api/oidc/callback", get(oidc::callback))
        .route("/api/oidc/exchange", post(oidc::web_exchange))
        .route("/api/switch-grant", post(switch_grant::register))
        .route("/api/audit/conn", post(logs::ingest_conn))
        .route("/api/audit/file", post(logs::ingest_file))
        .route("/api/audit/alarm", post(logs::ingest_alarm))
        .route("/api/audits/conn", get(logs::list_conn))
        .route("/api/audits/file", get(logs::list_file))
        .route("/api/audits/alarm", get(logs::list_alarm))
        .route(
            "/api/audits/{kind}/{id}",
            patch(logs::update_note).delete(logs::delete_log),
        )
        .route(
            "/api/record",
            post(records::upload).layer(DefaultBodyLimit::max(64 * 1024 * 1024)),
        )
        .route("/api/recordings", get(records::list))
        .route(
            "/api/recordings/{filename}",
            patch(records::update_note).delete(records::delete),
        )
        .route(
            "/api/recordings/{filename}/download",
            get(records::download),
        )
        .route(
            "/api/devices/deploy",
            post(deploy::deploy).layer(DefaultBodyLimit::max(64 * 1024)),
        )
        .route("/api/devices/cli", post(deploy::cli))
        .route(
            "/api/deployment/tokens",
            get(deploy::list_tokens).post(deploy::create_token),
        )
        .route(
            "/api/deployment/tokens/{id}",
            patch(deploy::update_token).delete(deploy::delete_token),
        )
        .route(
            "/api/relays",
            get(relays::list).post(relays::create),
        )
        .route("/api/relays/sync", post(relays::sync))
        .route(
            "/api/relays/{guid}",
            patch(relays::update).delete(relays::delete),
        )
        .route(
            "/api/settings/oidc",
            get(oidc::get_settings).put(oidc::save_settings),
        )
        .route("/api/settings/oidc/test", post(oidc::test_settings))
        .route(
            "/api/settings/smtp",
            get(settings_ext::get_smtp).put(settings_ext::save_smtp),
        )
        .route("/api/settings/smtp/test", post(settings_ext::test_smtp))
        .route(
            "/api/settings/ldap",
            get(settings_ext::get_ldap).put(settings_ext::save_ldap),
        )
        .route("/api/settings/ldap/test", post(settings_ext::test_ldap))
        .route("/api/permissions", get(roles::list_permissions))
        .route("/api/roles", get(roles::list).post(roles::create))
        .route(
            "/api/roles/{guid}",
            patch(roles::update).delete(roles::delete),
        )
        .route("/api/web-control/session", post(webcontrol::session))
        .route("/api/web-control/ws", get(webcontrol::ws_proxy))
        .route("/api/web-control/relay", get(webcontrol::relay_proxy))
        .route("/api/login", post(auth::login))
        .route("/api/logout", post(auth::logout))
        .route("/api/currentUser", post(auth::current_user))
        .route("/api/login-options", get(auth::login_options))
        .route("/api/2fa", get(tfa::status))
        .route("/api/2fa/setup", post(tfa::setup))
        .route("/api/2fa/enable", post(tfa::enable))
        .route("/api/2fa/disable", post(tfa::disable))
        .route("/api/users/{id}/2fa/reset", post(tfa::reset_user))
        .route("/api/heartbeat", post(client::heartbeat))
        .route("/api/sysinfo", post(client::sysinfo))
        .route(
            "/api/sysinfo_ver",
            post(client::sysinfo_ver).get(client::sysinfo_ver),
        )
        .route(
            "/api/users",
            get(client::list_users).post(users::create_user),
        )
        .route(
            "/api/users/{id}",
            patch(users::update_user).delete(users::delete_user),
        )
        .route(
            "/api/users/{id}/device-groups",
            get(users::get_user_groups).put(users::set_user_groups),
        )
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
        )
        .route("/api/audit", get(audit::list_audit))
        .route("/api/audit/actions", get(audit::list_actions))
        .route(
            "/api/strategies",
            get(strategies::list_strategies).post(strategies::create_strategy),
        )
        .route("/api/strategies/assign", post(strategies::assign))
        .route(
            "/api/strategies/{guid}",
            get(strategies::get_strategy)
                .patch(strategies::update_strategy)
                .delete(strategies::delete_strategy),
        )
        .route(
            "/api/strategies/{guid}/status",
            put(strategies::set_status),
        )
        .route(
            "/api/strategies/{guid}/assignments",
            get(strategies::get_assignments),
        )
        .route("/api/ab/settings", post(address_book::settings))
        .route("/api/ab/personal", post(address_book::personal))
        .route(
            "/api/ab/shared/profiles",
            post(address_book::shared_profiles),
        )
        .route("/api/ab/peers", post(address_book::peers))
        .route("/api/ab/tags/{ab}", post(address_book::tags))
        .route("/api/ab/peer/add/{ab}", post(address_book::add_peer))
        .route(
            "/api/ab/peer/update/{ab}",
            put(address_book::update_peer),
        )
        .route(
            "/api/ab/peer/{ab}",
            delete(address_book::delete_peers),
        )
        .route("/api/ab/tag/add/{ab}", post(address_book::add_tag))
        .route(
            "/api/ab/tag/rename/{ab}",
            put(address_book::rename_tag),
        )
        .route(
            "/api/ab/tag/update/{ab}",
            put(address_book::update_tag),
        )
        .route(
            "/api/ab/tag/{ab}",
            delete(address_book::delete_tag),
        )
        .route(
            "/api/address-books",
            get(address_book::list_books).post(address_book::create_book),
        )
        .route(
            "/api/address-books/{guid}",
            patch(address_book::update_book).delete(address_book::delete_book),
        )
        .route(
            "/api/address-books/{guid}/shares",
            get(address_book::get_shares).put(address_book::set_shares),
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
    let device_id = device_id.trim();
    let device_uuid = device_uuid.trim();
    if device_id.is_empty() {
        return Ok(());
    }
    let binding: Option<(String, String)> =
        sqlx::query_as("SELECT username, uuid FROM device_bindings WHERE device_id = ?")
            .bind(device_id)
            .fetch_optional(&state.db)
            .await?;
    let device: Option<(String, String)> =
        sqlx::query_as("SELECT uuid, user FROM devices WHERE id = ?")
            .bind(device_id)
            .fetch_optional(&state.db)
            .await?;

    if let Some((bound_user, bound_uuid)) = binding.as_ref() {
        if !bound_user.eq_ignore_ascii_case(username) {
            let expected = if !bound_uuid.is_empty() {
                Some(bound_uuid.as_str())
            } else {
                device
                    .as_ref()
                    .map(|(uuid, _)| uuid.as_str())
                    .filter(|uuid| !uuid.is_empty())
            };
            match expected {
                Some(expected) if !device_uuid.is_empty() && device_uuid == expected => {}
                _ => {
                    tracing::warn!(
                        device_id,
                        username,
                        "refusing to rebind a device to another user without uuid proof"
                    );
                    return Ok(());
                }
            }
        }
    } else if let Some((stored_uuid, stored_user)) = device.as_ref() {
        if !stored_user.is_empty() && !stored_user.eq_ignore_ascii_case(username) {
            if stored_uuid.is_empty() || device_uuid.is_empty() || device_uuid != stored_uuid {
                tracing::warn!(
                    device_id,
                    username,
                    "refusing to claim a device bound to another user"
                );
                return Ok(());
            }
        }
        if !stored_uuid.is_empty() && device_uuid != stored_uuid {
            tracing::warn!(device_id, username, "refusing to bind a device with mismatched uuid");
            return Ok(());
        }
    }

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
