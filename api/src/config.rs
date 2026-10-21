use std::env;
use std::path::Path;

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
        }
    }

    pub fn token_ttl_seconds(&self) -> i64 {
        self.token_ttl_hours.clamp(1, 24 * 365) * 3600
    }

    pub fn allow_any_origin(&self) -> bool {
        self.cors_origins.is_empty() || self.cors_origins.iter().any(|origin| origin == "*")
    }
}
