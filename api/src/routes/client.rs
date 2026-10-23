use axum::{
    extract::{rejection::JsonRejection, Query, State},
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde_json::{json, Value};
use sqlx::{QueryBuilder, Sqlite};

use crate::auth::AuthUser;
use crate::error::AppError;
use crate::models::{
    group_json, now_secs, peer_json, user_json_with, DeviceRow, GroupRow, HeartbeatRequest,
    PageQuery, UserRow,
};
use crate::rbac;
use crate::AppState;

const USER_COLUMNS: &str = "u.id, u.username, u.display_name, u.password_hash, u.email, u.note, u.avatar, u.status, u.is_admin, u.totp_secret, u.totp_enabled, u.email_verification, u.email_alarm_notification, u.email_verified, u.ldap_dn, u.oidc_subject, u.created_at, u.updated_at";

pub async fn heartbeat(
    State(state): State<AppState>,
    payload: Result<Json<HeartbeatRequest>, JsonRejection>,
) -> Result<Json<Value>, AppError> {
    let Json(request) =
        payload.map_err(|err| AppError::BadRequest(format!("请求格式错误：{err}")))?;
    let id = request.id.trim();
    if id.is_empty() {
        return Ok(Json(json!({})));
    }
    let uuid = request.uuid.trim();
    let now = now_secs();
    let mut existing = find_device(&state, id, uuid).await?;
    if existing.is_none() {
        let guid = uuid::Uuid::new_v4().to_string();
        let owner: Option<String> =
            sqlx::query_scalar("SELECT username FROM device_bindings WHERE device_id = ?")
                .bind(id)
                .fetch_optional(&state.db)
                .await?;
        sqlx::query(
            "INSERT OR IGNORE INTO devices
                (guid, id, uuid, user, device_name, note, disabled, last_online, info, version, created_at, updated_at)
             VALUES (?, ?, ?, ?, '', '', 0, ?, '{}', '', ?, ?)",
        )
        .bind(&guid)
        .bind(id)
        .bind(none_if_empty(uuid))
        .bind(owner)
        .bind(now)
        .bind(now)
        .bind(now)
        .execute(&state.db)
        .await?;
        existing = find_device(&state, id, uuid).await?;
    }

    let mut response = serde_json::Map::new();
    if let Some((guid, stored_uuid)) = existing {
        if !uuid_matches(&stored_uuid, uuid) {
            return Ok(Json(json!({})));
        }
        sqlx::query("UPDATE devices SET last_online = ?, updated_at = ? WHERE guid = ?")
            .bind(now)
            .bind(now)
            .bind(&guid)
            .execute(&state.db)
            .await?;
        if let Some((modified_at, config)) = resolve_strategy(&state, &guid).await? {
            response.insert("modified_at".to_owned(), json!(modified_at));
            response.insert(
                "strategy".to_owned(),
                json!({ "config_options": config }),
            );
        }
    }

    Ok(Json(Value::Object(response)))
}

async fn resolve_strategy(
    state: &AppState,
    guid: &str,
) -> Result<Option<(i64, Value)>, AppError> {
    let row: Option<(i64, String)> = sqlx::query_as(
        "SELECT s.updated_at, s.config
         FROM strategies s
         JOIN strategy_assignments a ON a.strategy_guid = s.guid
         JOIN devices d ON d.guid = ?
         WHERE s.enabled = 1 AND (
             (a.kind = 'device' AND (a.target_id = d.guid OR a.target_id = d.id))
             OR (a.kind = 'user' AND d.user IS NOT NULL AND a.target_id = d.user)
             OR (a.kind = 'group' AND d.device_group_guid IS NOT NULL
                 AND a.target_id = d.device_group_guid)
         )
         ORDER BY CASE a.kind WHEN 'device' THEN 0 WHEN 'user' THEN 1 ELSE 2 END
         LIMIT 1",
    )
    .bind(guid)
    .fetch_optional(&state.db)
    .await?;
    Ok(row.map(|(modified_at, config)| {
        (modified_at, super::strategies::parse_config(&config))
    }))
}

pub async fn sysinfo(
    State(state): State<AppState>,
    payload: Result<Json<Value>, JsonRejection>,
) -> Response {
    let result = match payload {
        Ok(Json(body)) => store_sysinfo(&state, &body).await,
        Err(err) => Err(AppError::BadRequest(format!(
            "请求格式错误：{err}"
        ))),
    };

    match result {
        Ok(()) => text_response(StatusCode::OK, "SYSINFO_UPDATED"),
        Err(AppError::BadRequest(message)) => {
            tracing::warn!(error = %message, "sysinfo rejected");
            text_response(StatusCode::OK, "ID_NOT_FOUND")
        }
        Err(err) => {
            tracing::warn!(error = %err, "failed to store sysinfo");
            text_response(StatusCode::INTERNAL_SERVER_ERROR, "SYSINFO_ERROR")
        }
    }
}

async fn store_sysinfo(state: &AppState, body: &Value) -> Result<(), AppError> {
    let id = body_str(body, "id").trim();
    if id.is_empty() {
        return Err(AppError::BadRequest("missing device id".to_owned()));
    }
    let uuid = body_str(body, "uuid").trim();
    let version = body
        .get("version")
        .map(|value| match value {
            Value::String(text) => text.clone(),
            other => other.to_string(),
        })
        .unwrap_or_default();
    let now = now_secs();
    let owner: Option<String> =
        sqlx::query_scalar("SELECT username FROM device_bindings WHERE device_id = ?")
            .bind(id)
            .fetch_optional(&state.db)
            .await?;
    let mut existing = find_device(state, id, uuid).await?;
    if existing.is_none() {
        let guid = uuid::Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT OR IGNORE INTO devices
                (guid, id, uuid, user, device_name, note, disabled, last_online, info, version, created_at, updated_at)
             VALUES (?, ?, ?, ?, '', '', 0, ?, ?, ?, ?, ?)",
        )
        .bind(&guid)
        .bind(id)
        .bind(none_if_empty(uuid))
        .bind(&owner)
        .bind(now)
        .bind(body.to_string())
        .bind(&version)
        .bind(now)
        .bind(now)
        .execute(&state.db)
        .await?;
        existing = find_device(state, id, uuid).await?;
    }
    let Some((guid, stored_uuid)) = existing else {
        return Err(AppError::BadRequest(
            "device id is not registered".to_owned(),
        ));
    };
    if !uuid_matches(&stored_uuid, uuid) {
        return Err(AppError::BadRequest("device uuid mismatch".to_owned()));
    }

    sqlx::query(
        "UPDATE devices SET
            uuid = COALESCE(NULLIF(?, ''), uuid),
            user = COALESCE(user, ?),
            info = ?,
            version = ?,
            last_online = ?,
            updated_at = ?
         WHERE guid = ?",
    )
    .bind(uuid)
    .bind(&owner)
    .bind(body.to_string())
    .bind(&version)
    .bind(now)
    .bind(now)
    .bind(&guid)
    .execute(&state.db)
    .await?;

    Ok(())
}

pub async fn sysinfo_ver() -> &'static str {
    "1"
}

pub async fn list_users(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(page): Query<PageQuery>,
) -> Result<Json<Value>, AppError> {
    let (_, page_size) = page.bounds();
    let offset = page.offset();

    let mut count = QueryBuilder::new("SELECT COUNT(*) FROM users u WHERE 1=1");
    push_user_scope(&mut count, &auth, page.status);
    let total: i64 = count.build_query_scalar().fetch_one(&state.db).await?;

    let mut list = QueryBuilder::new(format!("SELECT {USER_COLUMNS} FROM users u WHERE 1=1"));
    push_user_scope(&mut list, &auth, page.status);
    list.push(" ORDER BY u.username COLLATE NOCASE LIMIT ");
    list.push_bind(page_size);
    list.push(" OFFSET ");
    list.push_bind(offset);
    let rows = list
        .build_query_as::<UserRow>()
        .fetch_all(&state.db)
        .await?;

    let mut data: Vec<Value> = Vec::with_capacity(rows.len());
    for row in &rows {
        let permissions = rbac::load_permissions(&state.db, row.id, row.is_admin != 0).await?;
        let roles: Vec<String> = rbac::load_roles(&state.db, row.id)
            .await?
            .into_iter()
            .map(|(_, name)| name)
            .collect();
        data.push(user_json_with(row, &permissions, &roles));
    }
    Ok(Json(json!({ "total": total, "data": data })))
}

pub async fn list_peers(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(page): Query<PageQuery>,
) -> Result<Json<Value>, AppError> {
    let enabled_only = page.status == Some(1);
    let (total, rows) = query_devices(&state, &auth, &page, enabled_only).await?;
    let now = now_secs();
    let data: Vec<Value> = rows.iter().map(|row| peer_json(row, now)).collect();
    Ok(Json(json!({ "total": total, "data": data })))
}

pub async fn accessible_groups(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(page): Query<PageQuery>,
) -> Result<Json<Value>, AppError> {
    let (total, rows) = query_groups(&state, &auth, &page).await?;
    let data: Vec<Value> = rows.iter().map(group_json).collect();
    Ok(Json(json!({ "total": total, "data": data })))
}

pub(super) async fn query_devices(
    state: &AppState,
    auth: &AuthUser,
    page: &PageQuery,
    enabled_only: bool,
) -> Result<(i64, Vec<DeviceRow>), AppError> {
    let (_, page_size) = page.bounds();
    let offset = page.offset();

    let mut count = QueryBuilder::new("SELECT COUNT(*) FROM devices d WHERE 1=1");
    push_device_scope(&mut count, auth, enabled_only);
    let total: i64 = count.build_query_scalar().fetch_one(&state.db).await?;

    let mut list = QueryBuilder::new(
        "SELECT d.guid, d.id, d.uuid, d.user, d.device_group_guid,
                COALESCE(g.name, '') AS device_group_name,
                d.device_name, d.note, d.disabled, d.last_online, d.info, d.version,
                d.created_at, d.updated_at
         FROM devices d
         LEFT JOIN device_groups g ON g.guid = d.device_group_guid
         WHERE 1=1",
    );
    push_device_scope(&mut list, auth, enabled_only);
    list.push(" ORDER BY COALESCE(d.last_online, 0) DESC, d.created_at DESC LIMIT ");
    list.push_bind(page_size);
    list.push(" OFFSET ");
    list.push_bind(offset);
    let rows = list
        .build_query_as::<DeviceRow>()
        .fetch_all(&state.db)
        .await?;

    Ok((total, rows))
}

pub(super) async fn query_groups(
    state: &AppState,
    auth: &AuthUser,
    page: &PageQuery,
) -> Result<(i64, Vec<GroupRow>), AppError> {
    let (_, page_size) = page.bounds();
    let offset = page.offset();

    let mut count = QueryBuilder::new("SELECT COUNT(*) FROM device_groups g WHERE 1=1");
    push_group_scope(&mut count, auth);
    let total: i64 = count.build_query_scalar().fetch_one(&state.db).await?;

    let mut list = QueryBuilder::new(
        "SELECT g.guid, g.name, g.note, g.created_at, g.updated_at,
                (SELECT COUNT(*) FROM devices d WHERE d.device_group_guid = g.guid) AS device_count
         FROM device_groups g
         WHERE 1=1",
    );
    push_group_scope(&mut list, auth);
    list.push(" ORDER BY g.name COLLATE NOCASE LIMIT ");
    list.push_bind(page_size);
    list.push(" OFFSET ");
    list.push_bind(offset);
    let rows = list
        .build_query_as::<GroupRow>()
        .fetch_all(&state.db)
        .await?;

    Ok((total, rows))
}

fn push_device_scope(qb: &mut QueryBuilder<'_, Sqlite>, auth: &AuthUser, enabled_only: bool) {
    qb.push(" AND (");
    qb.push_bind(if auth.is_admin || auth.can("device.all") {
        1_i64
    } else {
        0_i64
    });
    qb.push(" = 1 OR d.user = ");
    qb.push_bind(auth.username.clone());
    qb.push(
        " OR d.device_group_guid IN (SELECT group_guid FROM user_device_groups WHERE user_id = ",
    );
    qb.push_bind(auth.id);
    qb.push("))");
    if enabled_only {
        qb.push(" AND d.disabled = 0");
    }
}

fn push_user_scope(qb: &mut QueryBuilder<'_, Sqlite>, auth: &AuthUser, status: Option<i64>) {
    if !auth.is_admin && !auth.can("user.read") {
        qb.push(" AND u.id = ");
        qb.push_bind(auth.id);
    }
    if let Some(status) = status {
        qb.push(" AND u.status = ");
        qb.push_bind(status);
    }
}

fn push_group_scope(qb: &mut QueryBuilder<'_, Sqlite>, auth: &AuthUser) {
    if auth.is_admin {
        return;
    }
    qb.push(" AND (g.guid IN (SELECT group_guid FROM user_device_groups WHERE user_id = ");
    qb.push_bind(auth.id);
    qb.push(") OR g.guid IN (SELECT device_group_guid FROM devices WHERE user = ");
    qb.push_bind(auth.username.clone());
    qb.push(" AND device_group_guid IS NOT NULL))");
}

async fn find_device(
    state: &AppState,
    id: &str,
    uuid: &str,
) -> Result<Option<(String, String)>, AppError> {
    if let Some(row) = sqlx::query_as::<_, (String, String)>(
        "SELECT guid, COALESCE(uuid, '') FROM devices WHERE id = ? LIMIT 1",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    {
        return Ok(Some(row));
    }
    if uuid.is_empty() {
        return Ok(None);
    }
    let Some((guid, stored_uuid)) = sqlx::query_as::<_, (String, String)>(
        "SELECT guid, COALESCE(uuid, '') FROM devices WHERE uuid = ? LIMIT 1",
    )
    .bind(uuid)
    .fetch_optional(&state.db)
    .await?
    else {
        return Ok(None);
    };
    let conflict: Option<i64> =
        sqlx::query_scalar("SELECT 1 FROM devices WHERE id = ? AND guid <> ?")
            .bind(id)
            .bind(&guid)
            .fetch_optional(&state.db)
            .await?;
    if conflict.is_some() {
        return Ok(None);
    }
    sqlx::query("UPDATE devices SET id = ?, updated_at = ? WHERE guid = ?")
        .bind(id)
        .bind(now_secs())
        .bind(&guid)
        .execute(&state.db)
        .await?;
    Ok(Some((guid, stored_uuid)))
}

fn uuid_matches(stored: &str, provided: &str) -> bool {
    stored.is_empty() || (!provided.is_empty() && stored == provided)
}

fn body_str<'a>(body: &'a Value, key: &str) -> &'a str {
    body.get(key).and_then(Value::as_str).unwrap_or("")
}

fn none_if_empty(value: &str) -> Option<&str> {
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

fn text_response(status: StatusCode, body: &'static str) -> Response {
    let mut response = (status, body).into_response();
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/plain; charset=utf-8"),
    );
    response
}

#[cfg(test)]
mod tests {
    use super::uuid_matches;

    #[test]
    fn uuid_matches_only_when_stored_or_equal() {
        assert!(uuid_matches("", "anything"));
        assert!(uuid_matches("abc", "abc"));
        assert!(!uuid_matches("abc", ""));
        assert!(!uuid_matches("abc", "abd"));
    }
}
