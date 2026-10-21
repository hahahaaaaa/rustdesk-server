#![forbid(unsafe_code)]

mod auth;
mod config;
mod db;
mod error;
mod models;
mod routes;
mod totp;

use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::SqlitePool,
    pub cfg: Arc<config::Config>,
    pub jwt_secret: Arc<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_tracing();

    let cfg = config::Config::from_env();
    let db = db::connect(&cfg.db_path).await?;
    db::init_schema(&db).await?;
    db::seed_admin(&db, &cfg).await?;
    let jwt_secret = db::load_or_create_jwt_secret(&db, cfg.jwt_secret.as_deref()).await?;

    let state = AppState {
        db,
        cfg: Arc::new(cfg),
        jwt_secret: Arc::new(jwt_secret),
    };

    let bind = format!("{}:{}", state.cfg.bind, state.cfg.port);
    let addr: std::net::SocketAddr = bind
        .parse()
        .map_err(|err| format!("invalid API_BIND/API_PORT combination {bind}: {err}"))?;
    let listener = tokio::net::TcpListener::bind(addr).await?;

    tracing::info!("RustDesk API listening on http://{addr}");
    tracing::info!("database: {}", state.cfg.db_path);
    match state.cfg.console_dir.as_deref() {
        Some(dir) => tracing::info!("serving web console from {dir}"),
        None => tracing::info!("web console bundle not found, API-only mode"),
    }

    let app = routes::router(state);
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

fn init_tracing() {
    use tracing_subscriber::EnvFilter;

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("rustdesk_api=info,tower_http=warn"));
    tracing_subscriber::fmt().with_env_filter(filter).init();
}

async fn shutdown_signal() {
    match tokio::signal::ctrl_c().await {
        Ok(()) => tracing::info!("shutdown signal received"),
        Err(err) => tracing::warn!(error = %err, "failed to listen for shutdown signal"),
    }
}
