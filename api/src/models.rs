use serde::Deserialize;
use serde_json::{json, Map, Value};

pub const ONLINE_WINDOW_SECS: i64 = 120;

pub fn now_secs() -> i64 {
    chrono::Utc::now().timestamp()
}

pub fn iso_from_secs(secs: Option<i64>) -> Option<String> {
    secs.and_then(|value| chrono::DateTime::<chrono::Utc>::from_timestamp(value, 0))
        .map(|time| time.to_rfc3339_opts(chrono::SecondsFormat::Secs, true))
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct DeviceInfo {
    #[serde(default)]
    pub os: String,
    #[serde(default, rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub name: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct LoginRequest {
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub password: String,
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub uuid: String,
    #[serde(default, rename = "autoLogin")]
    pub auto_login: bool,
    #[serde(default, rename = "type")]
    pub auth_type: String,
    #[serde(default, rename = "verificationCode")]
    pub verification_code: String,
    #[serde(default, rename = "tfaCode")]
    pub tfa_code: String,
    #[serde(default)]
    pub secret: String,
    #[serde(default, rename = "deviceInfo")]
    pub device_info: DeviceInfo,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct CurrentUserRequest {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub uuid: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct HeartbeatRequest {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub uuid: String,
    #[serde(default)]
    pub ver: Option<Value>,
    #[serde(default)]
    pub conns: Vec<Value>,
    #[serde(default)]
    pub modified_at: Option<i64>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct AssignRequest {
    #[serde(default, rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub value: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct UserCreateRequest {
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub password: String,
    #[serde(default)]
    pub display_name: String,
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub note: String,
    #[serde(default)]
    pub is_admin: bool,
    #[serde(default)]
    pub email_verification: Option<bool>,
    #[serde(default)]
    pub email_alarm_notification: Option<bool>,
    #[serde(default)]
    pub roles: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct UserUpdateRequest {
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub status: Option<i64>,
    #[serde(default)]
    pub is_admin: Option<bool>,
    #[serde(default)]
    pub email_verification: Option<bool>,
    #[serde(default)]
    pub email_alarm_notification: Option<bool>,
    #[serde(default)]
    pub roles: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct UserGroupsRequest {
    #[serde(default)]
    pub groups: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct GroupCreateRequest {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct GroupUpdateRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PageQuery {
    #[serde(default)]
    pub current: Option<i64>,
    #[serde(default)]
    pub page_size: Option<i64>,
    #[serde(default)]
    pub accessible: Option<String>,
    #[serde(default)]
    pub status: Option<i64>,
}

impl PageQuery {
    pub fn bounds(&self) -> (i64, i64) {
        let current = self.current.unwrap_or(1).max(1);
        let page_size = self.page_size.unwrap_or(100).clamp(1, 1000);
        (current, page_size)
    }

    pub fn offset(&self) -> i64 {
        let (current, page_size) = self.bounds();
        (current - 1) * page_size
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct UserRow {
    pub id: i64,
    pub username: String,
    pub display_name: String,
    pub password_hash: String,
    pub email: String,
    pub note: String,
    pub avatar: String,
    pub status: i64,
    pub is_admin: i64,
    pub totp_secret: String,
    pub totp_enabled: i64,
    pub email_verification: i64,
    pub email_alarm_notification: i64,
    pub email_verified: i64,
    pub ldap_dn: String,
    pub oidc_subject: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct DeviceRow {
    pub guid: String,
    pub id: String,
    pub uuid: Option<String>,
    pub user: Option<String>,
    pub device_group_guid: Option<String>,
    pub device_group_name: String,
    pub device_name: String,
    pub note: String,
    pub disabled: i64,
    pub last_online: Option<i64>,
    pub info: String,
    pub version: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct GroupRow {
    pub guid: String,
    pub name: String,
    pub note: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub device_count: i64,
}

pub fn user_json(row: &UserRow) -> Value {
    user_json_with(row, &[], &[])
}

pub fn user_json_with(row: &UserRow, permissions: &[String], roles: &[String]) -> Value {
    let third_auth_type = if !row.oidc_subject.is_empty() {
        Value::String("oidc".to_owned())
    } else if !row.ldap_dn.is_empty() {
        Value::String("ldap".to_owned())
    } else {
        Value::Null
    };
    json!({
        "id": row.id,
        "name": row.username,
        "display_name": row.display_name,
        "avatar": row.avatar,
        "email": row.email,
        "note": row.note,
        "status": row.status,
        "is_admin": row.is_admin != 0,
        "tfa_enabled": row.totp_enabled != 0,
        "permissions": permissions,
        "roles": roles,
        "info": {
            "email_verification": row.email_verification != 0,
            "email_alarm_notification": row.email_alarm_notification != 0,
            "email_verified": row.email_verified != 0,
            "login_device_whitelist": [],
            "other": {}
        },
        "third_auth_type": third_auth_type,
        "verifier": "",
        "created_at": iso_from_secs(Some(row.created_at)),
        "updated_at": iso_from_secs(Some(row.updated_at)),
    })
}

pub fn group_json(row: &GroupRow) -> Value {
    json!({
        "guid": row.guid,
        "name": row.name,
        "note": row.note,
        "device_count": row.device_count,
        "created_at": iso_from_secs(Some(row.created_at)),
        "updated_at": iso_from_secs(Some(row.updated_at)),
    })
}

pub fn device_json(row: &DeviceRow, now: i64) -> Value {
    let info = parse_info(&row.info);
    let disabled = row.disabled != 0;
    let online = is_online(row, now);
    let device_name = if !row.device_name.is_empty() {
        row.device_name.clone()
    } else {
        info.get("device_name")
            .and_then(Value::as_str)
            .filter(|name| !name.is_empty())
            .or_else(|| info.get("hostname").and_then(Value::as_str))
            .unwrap_or(row.id.as_str())
            .to_owned()
    };

    json!({
        "guid": row.guid,
        "id": row.id,
        "uuid": row.uuid,
        "user": row.user,
        "user_name": row.user,
        "device_group_guid": row.device_group_guid,
        "device_group_name": row.device_group_name,
        "device_name": device_name,
        "note": row.note,
        "status": if online { 1 } else { 0 },
        "online": online,
        "disabled": disabled,
        "last_online": iso_from_secs(row.last_online),
        "info": info,
        "version": row.version,
        "created_at": iso_from_secs(Some(row.created_at)),
        "updated_at": iso_from_secs(Some(row.updated_at)),
    })
}

pub fn peer_json(row: &DeviceRow, now: i64) -> Value {
    let owner = row.user.clone().unwrap_or_default();
    json!({
        "id": row.id,
        "guid": row.guid,
        "uuid": row.uuid,
        "info": parse_info(&row.info),
        "status": if is_online(row, now) { 1 } else { 0 },
        "disabled": row.disabled != 0,
        "user": owner,
        "user_name": owner,
        "device_group_guid": row.device_group_guid,
        "device_group_name": row.device_group_name,
        "note": row.note,
    })
}

pub fn parse_info(raw: &str) -> Value {
    serde_json::from_str::<Value>(raw)
        .ok()
        .filter(Value::is_object)
        .unwrap_or_else(|| Value::Object(Map::new()))
}

fn is_online(row: &DeviceRow, now: i64) -> bool {
    row.disabled == 0
        && row
            .last_online
            .map(|last| now - last <= ONLINE_WINDOW_SECS)
            .unwrap_or(false)
}
