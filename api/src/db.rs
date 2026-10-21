use std::time::Duration;

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::{distributions::Alphanumeric, rngs::OsRng, Rng, RngCore};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::SqlitePool;

use crate::config::Config;
use crate::error::AppError;
use crate::models::now_secs;

const SCHEMA: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS users (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        username TEXT NOT NULL UNIQUE,
        display_name TEXT NOT NULL DEFAULT '',
        password_hash TEXT NOT NULL,
        email TEXT NOT NULL DEFAULT '',
        note TEXT NOT NULL DEFAULT '',
        avatar TEXT NOT NULL DEFAULT '',
        status INTEGER NOT NULL DEFAULT 1,
        is_admin INTEGER NOT NULL DEFAULT 0,
        created_at INTEGER NOT NULL,
        updated_at INTEGER NOT NULL
    )",
    "CREATE TABLE IF NOT EXISTS device_groups (
        guid TEXT PRIMARY KEY,
        name TEXT NOT NULL COLLATE NOCASE UNIQUE,
        note TEXT NOT NULL DEFAULT '',
        created_at INTEGER NOT NULL,
        updated_at INTEGER NOT NULL
    )",
    "CREATE TABLE IF NOT EXISTS devices (
        guid TEXT PRIMARY KEY,
        id TEXT NOT NULL,
        uuid TEXT,
        user TEXT,
        device_group_guid TEXT,
        device_name TEXT NOT NULL DEFAULT '',
        note TEXT NOT NULL DEFAULT '',
        disabled INTEGER NOT NULL DEFAULT 0,
        last_online INTEGER,
        info TEXT NOT NULL DEFAULT '{}',
        version TEXT NOT NULL DEFAULT '',
        created_at INTEGER NOT NULL,
        updated_at INTEGER NOT NULL
    )",
    "CREATE UNIQUE INDEX IF NOT EXISTS idx_devices_id ON devices(id)",
    "CREATE UNIQUE INDEX IF NOT EXISTS idx_devices_uuid ON devices(uuid)",
    "CREATE INDEX IF NOT EXISTS idx_devices_user ON devices(user)",
    "CREATE INDEX IF NOT EXISTS idx_devices_group ON devices(device_group_guid)",
    "CREATE TABLE IF NOT EXISTS user_device_groups (
        user_id INTEGER NOT NULL,
        group_guid TEXT NOT NULL,
        created_at INTEGER NOT NULL,
        PRIMARY KEY (user_id, group_guid)
    )",
    "CREATE TABLE IF NOT EXISTS device_bindings (
        device_id TEXT PRIMARY KEY,
        uuid TEXT,
        username TEXT NOT NULL,
        updated_at INTEGER NOT NULL
    )",
    "CREATE TABLE IF NOT EXISTS audit_logs (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        actor TEXT NOT NULL,
        action TEXT NOT NULL,
        target TEXT NOT NULL DEFAULT '',
        detail TEXT NOT NULL DEFAULT '',
        created_at INTEGER NOT NULL
    )",
    "CREATE TABLE IF NOT EXISTS settings (
        key TEXT PRIMARY KEY,
        value TEXT NOT NULL
    )",
    "CREATE TABLE IF NOT EXISTS revoked_tokens (
        jti TEXT PRIMARY KEY,
        expires_at INTEGER NOT NULL
    )",
    "CREATE INDEX IF NOT EXISTS idx_revoked_exp ON revoked_tokens(expires_at)",
];

pub async fn connect(path: &str) -> Result<SqlitePool, sqlx::Error> {
    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .busy_timeout(Duration::from_secs(5))
        .foreign_keys(true);
    SqlitePoolOptions::new()
        .max_connections(8)
        .connect_with(options)
        .await
}

pub async fn init_schema(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    for statement in SCHEMA {
        sqlx::query(statement).execute(pool).await?;
    }
    Ok(())
}

pub async fn seed_admin(pool: &SqlitePool, cfg: &Config) -> Result<(), AppError> {
    let users: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(pool)
        .await?;
    if users > 0 {
        return Ok(());
    }

    let (password, generated) = match cfg.admin_password.as_deref() {
        Some(password) => (password.to_owned(), false),
        None => (random_password(), true),
    };
    let password_hash = bcrypt::hash(&password, bcrypt::DEFAULT_COST)
        .map_err(|err| AppError::Internal(format!("failed to hash admin password: {err}")))?;
    let now = now_secs();
    sqlx::query(
        "INSERT INTO users
            (username, display_name, password_hash, email, note, avatar, status, is_admin, created_at, updated_at)
         VALUES (?, ?, ?, '', '', '', 1, 1, ?, ?)",
    )
    .bind(&cfg.admin_username)
    .bind(&cfg.admin_username)
    .bind(password_hash)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await?;

    if generated {
        tracing::warn!(
            username = %cfg.admin_username,
            password = %password,
            "created the initial administrator account; change this password after signing in"
        );
    } else {
        tracing::info!(
            username = %cfg.admin_username,
            "created the initial administrator account"
        );
    }
    Ok(())
}

pub async fn load_or_create_jwt_secret(
    pool: &SqlitePool,
    provided: Option<&str>,
) -> Result<String, AppError> {
    if let Some(secret) = provided.filter(|secret| !secret.is_empty()) {
        return Ok(secret.to_owned());
    }

    if let Some(secret) = sqlx::query_scalar::<_, String>(
        "SELECT value FROM settings WHERE key = 'jwt_secret'",
    )
    .fetch_optional(pool)
    .await?
    {
        return Ok(secret);
    }

    let secret = random_secret();
    sqlx::query("INSERT OR IGNORE INTO settings (key, value) VALUES ('jwt_secret', ?)")
        .bind(&secret)
        .execute(pool)
        .await?;
    let stored = sqlx::query_scalar::<_, String>(
        "SELECT value FROM settings WHERE key = 'jwt_secret'",
    )
    .fetch_one(pool)
    .await?;
    Ok(stored)
}

fn random_password() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(18)
        .map(char::from)
        .collect()
}

fn random_secret() -> String {
    let mut bytes = [0u8; 48];
    OsRng.fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}
