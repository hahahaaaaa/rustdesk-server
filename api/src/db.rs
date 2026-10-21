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
        totp_secret TEXT NOT NULL DEFAULT '',
        totp_enabled INTEGER NOT NULL DEFAULT 0,
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
    "CREATE TABLE IF NOT EXISTS tfa_challenges (
        jti TEXT PRIMARY KEY,
        username TEXT NOT NULL,
        attempts INTEGER NOT NULL DEFAULT 0,
        expires_at INTEGER NOT NULL
    )",
    "CREATE INDEX IF NOT EXISTS idx_tfa_challenges_exp ON tfa_challenges(expires_at)",
    "CREATE TABLE IF NOT EXISTS strategies (
        guid TEXT PRIMARY KEY,
        name TEXT NOT NULL COLLATE NOCASE UNIQUE,
        note TEXT NOT NULL DEFAULT '',
        enabled INTEGER NOT NULL DEFAULT 1,
        config TEXT NOT NULL DEFAULT '{}',
        created_at INTEGER NOT NULL,
        updated_at INTEGER NOT NULL
    )",
    "CREATE TABLE IF NOT EXISTS strategy_assignments (
        strategy_guid TEXT NOT NULL REFERENCES strategies(guid) ON DELETE CASCADE,
        kind TEXT NOT NULL,
        target_id TEXT NOT NULL,
        created_at INTEGER NOT NULL,
        PRIMARY KEY (kind, target_id)
    )",
    "CREATE INDEX IF NOT EXISTS idx_strategy_assignments_strategy
        ON strategy_assignments(strategy_guid)",
    "CREATE TABLE IF NOT EXISTS address_books (
        guid TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        owner TEXT NOT NULL,
        note TEXT NOT NULL DEFAULT '',
        info TEXT NOT NULL DEFAULT '{}',
        is_personal INTEGER NOT NULL DEFAULT 0,
        created_at INTEGER NOT NULL,
        updated_at INTEGER NOT NULL
    )",
    "CREATE UNIQUE INDEX IF NOT EXISTS idx_address_books_shared_name
        ON address_books(name COLLATE NOCASE) WHERE is_personal = 0",
    "CREATE UNIQUE INDEX IF NOT EXISTS idx_address_books_personal_owner
        ON address_books(owner) WHERE is_personal = 1",
    "CREATE TABLE IF NOT EXISTS ab_peers (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        ab_guid TEXT NOT NULL REFERENCES address_books(guid) ON DELETE CASCADE,
        peer_id TEXT NOT NULL,
        hash TEXT NOT NULL DEFAULT '',
        password TEXT NOT NULL DEFAULT '',
        username TEXT NOT NULL DEFAULT '',
        hostname TEXT NOT NULL DEFAULT '',
        platform TEXT NOT NULL DEFAULT '',
        alias TEXT NOT NULL DEFAULT '',
        tags TEXT NOT NULL DEFAULT '[]',
        force_always_relay INTEGER NOT NULL DEFAULT 0,
        rdp_port TEXT NOT NULL DEFAULT '',
        rdp_username TEXT NOT NULL DEFAULT '',
        login_name TEXT NOT NULL DEFAULT '',
        device_group_name TEXT NOT NULL DEFAULT '',
        note TEXT NOT NULL DEFAULT '',
        created_at INTEGER NOT NULL,
        updated_at INTEGER NOT NULL,
        UNIQUE(ab_guid, peer_id)
    )",
    "CREATE INDEX IF NOT EXISTS idx_ab_peers_ab ON ab_peers(ab_guid)",
    "CREATE TABLE IF NOT EXISTS ab_tags (
        ab_guid TEXT NOT NULL REFERENCES address_books(guid) ON DELETE CASCADE,
        name TEXT NOT NULL,
        color INTEGER NOT NULL,
        created_at INTEGER NOT NULL,
        PRIMARY KEY (ab_guid, name)
    )",
    "CREATE TABLE IF NOT EXISTS ab_shares (
        ab_guid TEXT NOT NULL REFERENCES address_books(guid) ON DELETE CASCADE,
        user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
        rule INTEGER NOT NULL DEFAULT 1,
        created_at INTEGER NOT NULL,
        PRIMARY KEY (ab_guid, user_id)
    )",
    "CREATE INDEX IF NOT EXISTS idx_ab_shares_user ON ab_shares(user_id)",
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
    ensure_column(pool, "users", "totp_secret", "TEXT NOT NULL DEFAULT ''").await?;
    ensure_column(pool, "users", "totp_enabled", "INTEGER NOT NULL DEFAULT 0").await?;
    Ok(())
}

async fn ensure_column(
    pool: &SqlitePool,
    table: &str,
    column: &str,
    definition: &str,
) -> Result<(), sqlx::Error> {
    let exists: i64 = sqlx::query_scalar(&format!(
        "SELECT COUNT(*) FROM pragma_table_info('{table}') WHERE name = ?"
    ))
    .bind(column)
    .fetch_one(pool)
    .await?;
    if exists == 0 {
        sqlx::query(&format!(
            "ALTER TABLE {table} ADD COLUMN {column} {definition}"
        ))
        .execute(pool)
        .await?;
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
