use ldap3::{LdapConnAsync, Scope, SearchEntry};
use serde_json::json;

use crate::db;
use crate::AppState;

#[derive(Debug, Clone)]
pub struct LdapConfig {
    pub host: String,
    pub port: u16,
    pub encryption: String,
    pub bind_dn: String,
    pub bind_password: String,
    pub base_dn: String,
    pub user_filter: String,
}

impl LdapConfig {
    pub async fn load(state: &AppState) -> Result<Option<Self>, sqlx::Error> {
        if db::get_setting(&state.db, "ldap_enabled").await?.as_deref() != Some("1") {
            return Ok(None);
        }
        let host = setting(state, "ldap_host").await;
        let base_dn = setting(state, "ldap_base_dn").await;
        if host.is_empty() || base_dn.is_empty() {
            return Ok(None);
        }
        let encryption = {
            let value = setting(state, "ldap_encryption").await;
            if value.is_empty() {
                "none".to_owned()
            } else {
                value
            }
        };
        let port = setting(state, "ldap_port")
            .await
            .parse()
            .unwrap_or(if encryption == "ldaps" { 636 } else { 389 });
        let filter = {
            let value = setting(state, "ldap_user_filter").await;
            if value.is_empty() {
                "(uid={username})".to_owned()
            } else {
                value
            }
        };
        Ok(Some(Self {
            host,
            port,
            encryption,
            bind_dn: setting(state, "ldap_bind_dn").await,
            bind_password: setting(state, "ldap_bind_password").await,
            base_dn,
            user_filter: filter,
        }))
    }

    fn url(&self) -> String {
        let scheme = if self.encryption == "ldaps" {
            "ldaps"
        } else {
            "ldap"
        };
        format!("{scheme}://{}:{}", self.host, self.port)
    }
}

async fn setting(state: &AppState, key: &str) -> String {
    db::get_setting(&state.db, key)
        .await
        .ok()
        .flatten()
        .unwrap_or_default()
}

pub async fn is_enabled(state: &AppState) -> bool {
    matches!(LdapConfig::load(state).await, Ok(Some(_)))
}

async fn connect(config: &LdapConfig) -> Result<ldap3::Ldap, String> {
    if config.encryption == "starttls" {
        return Err("当前 LDAP 客户端不支持 StartTLS，请改用 ldaps 或明文连接。".to_owned());
    }
    let url = config.url();
    let (conn, ldap) = LdapConnAsync::new(&url)
        .await
        .map_err(|err| format!("无法连接 LDAP 服务器：{err}"))?;
    ldap3::drive!(conn);
    Ok(ldap)
}

/// Returns the bound DN when the credentials are accepted.
pub async fn authenticate(
    config: &LdapConfig,
    username: &str,
    password: &str,
) -> Result<Option<String>, String> {
    if password.is_empty() {
        return Ok(None);
    }
    let mut ldap = connect(config).await?;
    if !config.bind_dn.is_empty() {
        let result = ldap
            .simple_bind(&config.bind_dn, &config.bind_password)
            .await
            .map_err(|err| format!("LDAP 服务账号绑定失败：{err}"))?;
        if result.rc != 0 {
            return Err(format!("LDAP 服务账号绑定失败：{}", result.text));
        }
    }
    let filter = config
        .user_filter
        .replace("{username}", &escape_filter_value(username));
    let (entries, _) = ldap
        .search(&config.base_dn, Scope::Subtree, &filter, vec!["dn"])
        .await
        .map_err(|err| format!("LDAP 用户查询失败：{err}"))?
        .success()
        .map_err(|err| format!("LDAP 用户查询失败：{err}"))?;
    let Some(entry) = entries.into_iter().next().map(SearchEntry::construct) else {
        return Ok(None);
    };
    let dn = entry.dn;
    let _ = ldap.unbind().await;

    let mut user_ldap = connect(config).await?;
    let result = user_ldap
        .simple_bind(&dn, password)
        .await
        .map_err(|err| format!("LDAP 登录失败：{err}"))?;
    let _ = user_ldap.unbind().await;
    if result.rc == 0 {
        Ok(Some(dn))
    } else {
        Ok(None)
    }
}

pub async fn test_bind(config: &LdapConfig) -> Result<(), String> {
    let mut ldap = connect(config).await?;
    if config.bind_dn.is_empty() {
        let result = ldap
            .simple_bind("", "")
            .await
            .map_err(|err| format!("LDAP 匿名绑定失败：{err}"))?;
        if result.rc != 0 {
            return Err(format!("LDAP 匿名绑定被拒绝：{}", result.text));
        }
    } else {
        let result = ldap
            .simple_bind(&config.bind_dn, &config.bind_password)
            .await
            .map_err(|err| format!("LDAP 绑定失败：{err}"))?;
        if result.rc != 0 {
            return Err(format!("LDAP 绑定被拒绝：{}", result.text));
        }
    }
    let (_, _) = ldap
        .search(&config.base_dn, Scope::Base, "(objectClass=*)", vec!["dn"])
        .await
        .map_err(|err| format!("LDAP 基准 DN 查询失败：{err}"))?
        .success()
        .map_err(|err| format!("LDAP 基准 DN 查询失败：{err}"))?;
    let _ = ldap.unbind().await;
    Ok(())
}

pub fn config_json(config: &LdapConfig) -> serde_json::Value {
    json!({
        "host": config.host,
        "port": config.port,
        "encryption": config.encryption,
        "base_dn": config.base_dn,
        "user_filter": config.user_filter,
        "bind_dn": config.bind_dn,
    })
}

fn escape_filter_value(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'*' => escaped.push_str("\\2a"),
            b'(' => escaped.push_str("\\28"),
            b')' => escaped.push_str("\\29"),
            b'\\' => escaped.push_str("\\5c"),
            0 => escaped.push_str("\\00"),
            other => escaped.push(other as char),
        }
    }
    escaped
}
