use std::env;
use std::path::{Path, PathBuf};

use base64::Engine;

#[derive(Clone, Debug)]
pub struct Config {
    pub bind: String,
    pub port: u16,
    pub db_path: String,
    pub jwt_secret: Option<String>,
    pub token_ttl_hours: i64,
    pub admin_username: String,
    pub admin_password: Option<String>,
    pub cors_origins: Vec<String>,
    pub console_dir: Option<String>,
    pub ab_max_peer_one_ab: i64,
    pub rendezvous_addr: String,
    pub relay_addr: String,
    pub server_public_key: Option<String>,
    pub server_public_key_file: Option<PathBuf>,
    pub license_key: Option<String>,
}

impl Config {
    pub fn from_env() -> Self {
        let cors_origins = env::var("API_CORS_ORIGINS")
            .unwrap_or_else(|_| "*".to_owned())
            .split(',')
            .map(str::trim)
            .filter(|origin| !origin.is_empty())
            .map(ToOwned::to_owned)
            .collect();

        let console_dir = env::var("API_CONSOLE_DIR")
            .ok()
            .filter(|dir| !dir.trim().is_empty())
            .or_else(|| {
                let default_dir = Path::new("web-console/dist");
                default_dir
                    .is_dir()
                    .then(|| default_dir.display().to_string())
            });

        Self {
            bind: env::var("API_BIND").unwrap_or_else(|_| "0.0.0.0".to_owned()),
            port: env::var("API_PORT")
                .ok()
                .and_then(|value| value.parse().ok())
                .unwrap_or(21114),
            db_path: env::var("API_DB_PATH").unwrap_or_else(|_| "api.sqlite3".to_owned()),
            jwt_secret: env::var("API_JWT_SECRET")
                .ok()
                .filter(|secret| !secret.is_empty()),
            token_ttl_hours: env::var("API_TOKEN_TTL_HOURS")
                .ok()
                .and_then(|value| value.parse().ok())
                .unwrap_or(168),
            admin_username: env::var("API_ADMIN_USERNAME")
                .ok()
                .filter(|name| !name.is_empty())
                .unwrap_or_else(|| "admin".to_owned()),
            admin_password: env::var("API_ADMIN_PASSWORD")
                .ok()
                .filter(|password| !password.is_empty()),
            cors_origins,
            console_dir,
            ab_max_peer_one_ab: env::var("API_AB_MAX_PEERS")
                .ok()
                .and_then(|value| value.parse().ok())
                .filter(|value| *value >= 0)
                .unwrap_or(0),
            rendezvous_addr: env::var("API_RENDEZVOUS_ADDR")
                .unwrap_or_else(|_| "127.0.0.1:21116".to_owned()),
            relay_addr: env::var("API_RELAY_ADDR").unwrap_or_else(|_| "127.0.0.1:21117".to_owned()),
            server_public_key: env::var("API_SERVER_PUBLIC_KEY")
                .ok()
                .map(|key| key.trim().to_owned())
                .filter(|key| !key.is_empty()),
            server_public_key_file: env::var("API_SERVER_PUBLIC_KEY_FILE")
                .ok()
                .map(|path| path.trim().to_owned())
                .filter(|path| !path.is_empty())
                .map(PathBuf::from),
            license_key: env::var("API_LICENSE_KEY")
                .ok()
                .map(|key| key.trim().to_owned())
                .filter(|key| !key.is_empty()),
        }
    }

    pub fn token_ttl_seconds(&self) -> i64 {
        self.token_ttl_hours.clamp(1, 24 * 365) * 3600
    }

    pub fn allow_any_origin(&self) -> bool {
        self.cors_origins.is_empty() || self.cors_origins.iter().any(|origin| origin == "*")
    }

    pub fn server_public_key(&self) -> Option<String> {
        if let Some(key) = self.server_public_key.as_deref() {
            if valid_public_key(key) {
                return Some(key.to_owned());
            }
        }

        let mut candidates = Vec::new();
        if let Some(path) = self.server_public_key_file.as_ref() {
            candidates.push(path.clone());
        }
        if let Some(parent) = Path::new(&self.db_path).parent() {
            candidates.push(parent.join("id_ed25519.pub"));
        }
        candidates.push(PathBuf::from("/data/id_ed25519.pub"));
        candidates.push(PathBuf::from("id_ed25519.pub"));

        candidates.into_iter().find_map(|path| {
            let key = std::fs::read_to_string(path).ok()?;
            let key = key.trim();
            valid_public_key(key).then(|| key.to_owned())
        })
    }
}

fn valid_public_key(key: &str) -> bool {
    base64::engine::general_purpose::STANDARD
        .decode(key)
        .is_ok_and(|bytes| bytes.len() == 32)
}
