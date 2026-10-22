use axum::{extract::State, Json};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::auth::AuthUser;
use crate::db;
use crate::directory::{self, LdapConfig};
use crate::error::AppError;
use crate::mailer::{self, SmtpConfig};
use crate::AppState;

#[derive(Debug, Clone, Deserialize, Default)]
pub struct SmtpSettingsRequest {
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub host: Option<String>,
    #[serde(default)]
    pub port: Option<i64>,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default)]
    pub clear_password: Option<bool>,
    #[serde(default)]
    pub from: Option<String>,
    #[serde(default)]
    pub from_name: Option<String>,
    #[serde(default)]
    pub encryption: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct SmtpTestRequest {
    #[serde(default)]
    pub to: String,
    #[serde(flatten)]
    pub settings: SmtpSettingsRequest,
}

pub async fn get_smtp(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, AppError> {
    require(&auth, "settings.read")?;
    let encryption = {
        let value = setting(&state, "smtp_encryption").await;
        if value.is_empty() {
            "starttls".to_owned()
        } else {
            value
        }
    };
    Ok(Json(json!({
        "enabled": setting(&state, "smtp_enabled").await == "1",
        "host": setting(&state, "smtp_host").await,
        "port": setting(&state, "smtp_port").await.parse::<i64>().unwrap_or(587),
        "username": setting(&state, "smtp_username").await,
        "password_set": !setting(&state, "smtp_password").await.is_empty(),
        "from": setting(&state, "smtp_from").await,
        "from_name": setting(&state, "smtp_from_name").await,
        "encryption": encryption,
    })))
}

pub async fn save_smtp(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(body): Json<SmtpSettingsRequest>,
) -> Result<Json<Value>, AppError> {
    require(&auth, "settings.write")?;
    save_smtp_values(&state, &body).await?;
    super::audit(&state, &auth.username, "settings.smtp", "", "").await;
    Ok(Json(json!({ "success": true })))
}

pub async fn test_smtp(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(body): Json<SmtpTestRequest>,
) -> Result<Json<Value>, AppError> {
    require(&auth, "settings.write")?;
    let to = if body.to.trim().is_empty() {
        auth_email(&state, &auth.username).await.unwrap_or_default()
    } else {
        body.to.trim().to_owned()
    };
    if to.is_empty() {
        return Err(AppError::BadRequest("请填写测试收件人邮箱。".to_owned()));
    }
    let config = resolve_smtp(&state, &body.settings).await?;
    match mailer::send_with_config(
        &config,
        &to,
        "RustDesk SMTP 测试邮件",
        "这是一封来自 RustDesk 控制台的测试邮件，如果您收到它，说明 SMTP 配置正确。\n".to_owned(),
    )
    .await
    {
        Ok(()) => Ok(Json(json!({ "success": true, "message": format!("测试邮件已发送至 {to}") }))),
        Err(err) => Ok(Json(json!({ "success": false, "error": err }))),
    }
}

async fn save_smtp_values(
    state: &AppState,
    body: &SmtpSettingsRequest,
) -> Result<(), AppError> {
    if let Some(value) = body.enabled {
        db::set_setting(&state.db, "smtp_enabled", if value { "1" } else { "0" }).await?;
    }
    if let Some(value) = &body.host {
        db::set_setting(&state.db, "smtp_host", value.trim()).await?;
    }
    if let Some(value) = body.port {
        if !(1..=65535).contains(&value) {
            return Err(AppError::BadRequest("SMTP 端口无效。".to_owned()));
        }
        db::set_setting(&state.db, "smtp_port", &value.to_string()).await?;
    }
    if let Some(value) = &body.username {
        db::set_setting(&state.db, "smtp_username", value.trim()).await?;
    }
    if body.clear_password.unwrap_or(false) {
        db::set_setting(&state.db, "smtp_password", "").await?;
    } else if let Some(value) = &body.password {
        if !value.is_empty() {
            db::set_setting(&state.db, "smtp_password", value).await?;
        }
    }
    if let Some(value) = &body.from {
        db::set_setting(&state.db, "smtp_from", value.trim()).await?;
    }
    if let Some(value) = &body.from_name {
        db::set_setting(&state.db, "smtp_from_name", value.trim()).await?;
    }
    if let Some(value) = &body.encryption {
        let value = value.trim();
        if !matches!(value, "none" | "starttls" | "tls") {
            return Err(AppError::BadRequest("SMTP 加密方式无效。".to_owned()));
        }
        db::set_setting(&state.db, "smtp_encryption", value).await?;
    }
    Ok(())
}

async fn resolve_smtp(
    state: &AppState,
    body: &SmtpSettingsRequest,
) -> Result<SmtpConfig, AppError> {
    let stored_host = setting(state, "smtp_host").await;
    let stored_port = setting(state, "smtp_port").await;
    let stored_username = setting(state, "smtp_username").await;
    let stored_password = setting(state, "smtp_password").await;
    let stored_from = setting(state, "smtp_from").await;
    let stored_from_name = setting(state, "smtp_from_name").await;
    let stored_encryption = setting(state, "smtp_encryption").await;

    let pick = |value: &Option<String>, stored: &str| -> String {
        value
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or(stored)
            .to_owned()
    };
    let config = SmtpConfig {
        host: pick(&body.host, &stored_host),
        port: body
            .port
            .unwrap_or_else(|| stored_port.parse().unwrap_or(587))
            .clamp(1, 65535) as u16,
        username: pick(&body.username, &stored_username),
        password: body
            .password
            .clone()
            .filter(|value| !value.is_empty())
            .unwrap_or(stored_password),
        from: pick(&body.from, &stored_from),
        from_name: pick(&body.from_name, &stored_from_name),
        encryption: {
            let value = pick(&body.encryption, &stored_encryption);
            if value.is_empty() {
                "starttls".to_owned()
            } else {
                value
            }
        },
    };
    if config.host.is_empty() || config.from.is_empty() {
        return Err(AppError::BadRequest(
            "请先填写 SMTP 服务器和发件人地址。".to_owned(),
        ));
    }
    Ok(config)
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct LdapSettingsRequest {
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub host: Option<String>,
    #[serde(default)]
    pub port: Option<i64>,
    #[serde(default)]
    pub encryption: Option<String>,
    #[serde(default)]
    pub bind_dn: Option<String>,
    #[serde(default)]
    pub bind_password: Option<String>,
    #[serde(default)]
    pub clear_password: Option<bool>,
    #[serde(default)]
    pub base_dn: Option<String>,
    #[serde(default)]
    pub user_filter: Option<String>,
    #[serde(default)]
    pub auto_create: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct LdapTestRequest {
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub password: String,
    #[serde(flatten)]
    pub settings: LdapSettingsRequest,
}

pub async fn get_ldap(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, AppError> {
    require(&auth, "settings.read")?;
    let encryption = {
        let value = setting(&state, "ldap_encryption").await;
        if value.is_empty() {
            "none".to_owned()
        } else {
            value
        }
    };
    Ok(Json(json!({
        "enabled": setting(&state, "ldap_enabled").await == "1",
        "host": setting(&state, "ldap_host").await,
        "port": setting(&state, "ldap_port").await.parse::<i64>().unwrap_or(389),
        "encryption": encryption,
        "bind_dn": setting(&state, "ldap_bind_dn").await,
        "bind_password_set": !setting(&state, "ldap_bind_password").await.is_empty(),
        "base_dn": setting(&state, "ldap_base_dn").await,
        "user_filter": setting(&state, "ldap_user_filter").await,
        "auto_create": setting(&state, "ldap_auto_create").await != "0",
    })))
}

pub async fn save_ldap(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(body): Json<LdapSettingsRequest>,
) -> Result<Json<Value>, AppError> {
    require(&auth, "settings.write")?;
    if let Some(value) = body.enabled {
        db::set_setting(&state.db, "ldap_enabled", if value { "1" } else { "0" }).await?;
    }
    if let Some(value) = &body.host {
        db::set_setting(&state.db, "ldap_host", value.trim()).await?;
    }
    if let Some(value) = body.port {
        if !(1..=65535).contains(&value) {
            return Err(AppError::BadRequest("LDAP 端口无效。".to_owned()));
        }
        db::set_setting(&state.db, "ldap_port", &value.to_string()).await?;
    }
    if let Some(value) = &body.encryption {
        let value = value.trim();
        if !matches!(value, "none" | "ldaps") {
            return Err(AppError::BadRequest("LDAP 加密方式无效。".to_owned()));
        }
        db::set_setting(&state.db, "ldap_encryption", value).await?;
    }
    if let Some(value) = &body.bind_dn {
        db::set_setting(&state.db, "ldap_bind_dn", value.trim()).await?;
    }
    if body.clear_password.unwrap_or(false) {
        db::set_setting(&state.db, "ldap_bind_password", "").await?;
    } else if let Some(value) = &body.bind_password {
        if !value.is_empty() {
            db::set_setting(&state.db, "ldap_bind_password", value).await?;
        }
    }
    if let Some(value) = &body.base_dn {
        db::set_setting(&state.db, "ldap_base_dn", value.trim()).await?;
    }
    if let Some(value) = &body.user_filter {
        db::set_setting(&state.db, "ldap_user_filter", value.trim()).await?;
    }
    if let Some(value) = body.auto_create {
        db::set_setting(&state.db, "ldap_auto_create", if value { "1" } else { "0" }).await?;
    }
    super::audit(&state, &auth.username, "settings.ldap", "", "").await;
    Ok(Json(json!({ "success": true })))
}

pub async fn test_ldap(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(body): Json<LdapTestRequest>,
) -> Result<Json<Value>, AppError> {
    require(&auth, "settings.write")?;
    let stored_host = setting(&state, "ldap_host").await;
    let stored_port = setting(&state, "ldap_port").await;
    let stored_encryption = setting(&state, "ldap_encryption").await;
    let stored_bind_dn = setting(&state, "ldap_bind_dn").await;
    let stored_password = setting(&state, "ldap_bind_password").await;
    let stored_base_dn = setting(&state, "ldap_base_dn").await;
    let stored_filter = setting(&state, "ldap_user_filter").await;
    let pick = |value: &Option<String>, stored: &str| -> String {
        value
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or(stored)
            .to_owned()
    };
    let config = LdapConfig {
        host: pick(&body.settings.host, &stored_host),
        port: body
            .settings
            .port
            .unwrap_or_else(|| stored_port.parse().unwrap_or(389))
            .clamp(1, 65535) as u16,
        encryption: {
            let value = pick(&body.settings.encryption, &stored_encryption);
            if value.is_empty() {
                "none".to_owned()
            } else {
                value
            }
        },
        bind_dn: pick(&body.settings.bind_dn, &stored_bind_dn),
        bind_password: body
            .settings
            .bind_password
            .clone()
            .filter(|value| !value.is_empty())
            .unwrap_or(stored_password),
        base_dn: pick(&body.settings.base_dn, &stored_base_dn),
        user_filter: {
            let value = pick(&body.settings.user_filter, &stored_filter);
            if value.is_empty() {
                "(uid={username})".to_owned()
            } else {
                value
            }
        },
    };
    if config.host.is_empty() || config.base_dn.is_empty() {
        return Err(AppError::BadRequest("请先填写 LDAP 地址和基准 DN。".to_owned()));
    }

    if !body.username.trim().is_empty() {
        let value =
            match directory::authenticate(&config, body.username.trim(), &body.password).await {
                Ok(Some(dn)) => json!({ "success": true, "dn": dn }),
                Ok(None) => json!({ "success": false, "error": "LDAP 用户认证失败。" }),
                Err(err) => json!({ "success": false, "error": err }),
            };
        return Ok(Json(value));
    }
    match directory::test_bind(&config).await {
        Ok(()) => Ok(Json(json!({ "success": true }))),
        Err(err) => Ok(Json(json!({ "success": false, "error": err }))),
    }
}

async fn auth_email(state: &AppState, username: &str) -> Option<String> {
    sqlx::query_scalar::<_, String>("SELECT email FROM users WHERE username = ?")
        .bind(username)
        .fetch_optional(&state.db)
        .await
        .ok()
        .flatten()
        .filter(|email| !email.is_empty())
}

async fn setting(state: &AppState, key: &str) -> String {
    db::get_setting(&state.db, key)
        .await
        .ok()
        .flatten()
        .unwrap_or_default()
}

fn require(auth: &AuthUser, permission: &str) -> Result<(), AppError> {
    if auth.can(permission) {
        Ok(())
    } else {
        Err(AppError::Forbidden("没有权限执行此操作。".to_owned()))
    }
}
