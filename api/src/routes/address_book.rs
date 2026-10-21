use std::collections::{HashMap, HashSet};

use axum::{
    extract::{rejection::JsonRejection, Path, Query, State},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Map, Value};
use sqlx::{QueryBuilder, Sqlite};

use crate::auth::AuthUser;
use crate::error::AppError;
use crate::models::{iso_from_secs, now_secs, parse_info, PageQuery};
use crate::AppState;

#[derive(Debug, Clone, sqlx::FromRow)]
struct BookRow {
    guid: String,
    name: String,
    owner: String,
    note: String,
    info: String,
    is_personal: i64,
    created_at: i64,
    updated_at: i64,
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct BookListRow {
    guid: String,
    name: String,
    owner: String,
    note: String,
    info: String,
    created_at: i64,
    updated_at: i64,
    peer_count: i64,
    share_count: i64,
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct SharedProfileRow {
    guid: String,
    name: String,
    owner: String,
    note: String,
    info: String,
    rule: i64,
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct PeerRow {
    peer_id: String,
    hash: String,
    password: String,
    username: String,
    hostname: String,
    platform: String,
    alias: String,
    tags: String,
    force_always_relay: i64,
    rdp_port: String,
    rdp_username: String,
    login_name: String,
    device_group_name: String,
    note: String,
    same_server: i64,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AbPeersQuery {
    #[serde(default)]
    pub current: Option<i64>,
    #[serde(default)]
    pub page_size: Option<i64>,
    #[serde(default)]
    pub ab: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct TagPayload {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub color: Option<i64>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct RenameTagPayload {
    #[serde(default)]
    pub old: String,
    #[serde(default)]
    pub new: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct BookListQuery {
    #[serde(default)]
    pub current: Option<i64>,
    #[serde(default)]
    pub page_size: Option<i64>,
    #[serde(default)]
    pub q: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct BookPayload {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub password: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct BookUpdatePayload {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub password: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ShareEntry {
    #[serde(default)]
    pub user_id: i64,
    #[serde(default)]
    pub rule: i64,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct SharesPayload {
    #[serde(default)]
    pub shares: Vec<ShareEntry>,
}

pub async fn settings(State(state): State<AppState>, _auth: AuthUser) -> Json<Value> {
    Json(json!({ "max_peer_one_ab": state.cfg.ab_max_peer_one_ab }))
}

pub async fn personal(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, AppError> {
    let guid = ensure_personal_book(&state, &auth).await?;
    Ok(Json(json!({ "guid": guid })))
}

pub async fn shared_profiles(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<PageQuery>,
) -> Result<Json<Value>, AppError> {
    let (_, page_size) = query.bounds();
    let offset = query.offset();
    let total: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM address_books b
         JOIN ab_shares s ON s.ab_guid = b.guid
         WHERE s.user_id = ? AND b.is_personal = 0",
    )
    .bind(auth.id)
    .fetch_one(&state.db)
    .await?;
    let rows: Vec<SharedProfileRow> = sqlx::query_as(
        "SELECT b.guid, b.name, b.owner, b.note, b.info, s.rule
         FROM address_books b
         JOIN ab_shares s ON s.ab_guid = b.guid
         WHERE s.user_id = ? AND b.is_personal = 0
         ORDER BY b.name COLLATE NOCASE, b.guid
         LIMIT ? OFFSET ?",
    )
    .bind(auth.id)
    .bind(page_size)
    .bind(offset)
    .fetch_all(&state.db)
    .await?;

    let data: Vec<Value> = rows
        .iter()
        .map(|row| {
            json!({
                "guid": row.guid,
                "name": row.name,
                "owner": row.owner,
                "note": row.note,
                "info": parse_info(&row.info),
                "rule": row.rule,
            })
        })
        .collect();
    Ok(Json(json!({ "total": total, "data": data })))
}

pub async fn peers(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<AbPeersQuery>,
) -> Result<Json<Value>, AppError> {
    let ab = query
        .ab
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| AppError::BadRequest("缺少地址簿参数。".to_owned()))?;
    let (book, _) = resolve_book(&state, &auth, ab).await?;
    let current = query.current.unwrap_or(1).max(1);
    let page_size = query.page_size.unwrap_or(100).clamp(1, 1000);
    let offset = (current - 1) * page_size;

    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM ab_peers WHERE ab_guid = ?")
        .bind(ab)
        .fetch_one(&state.db)
        .await?;
    let rows: Vec<PeerRow> = sqlx::query_as(
        "SELECT p.peer_id, p.hash, p.password, p.username, p.hostname, p.platform,
                p.alias, p.tags, p.force_always_relay, p.rdp_port, p.rdp_username,
                p.login_name, p.device_group_name, p.note,
                CASE WHEN d.id IS NULL THEN 0 ELSE 1 END AS same_server
         FROM ab_peers p
         LEFT JOIN devices d ON d.id = p.peer_id
         WHERE p.ab_guid = ?
         ORDER BY p.id
         LIMIT ? OFFSET ?",
    )
    .bind(ab)
    .bind(page_size)
    .bind(offset)
    .fetch_all(&state.db)
    .await?;

    let personal = book.is_personal != 0 && book.owner == auth.username;
    let data: Vec<Value> = rows.iter().map(|row| peer_json(row, personal)).collect();
    Ok(Json(json!({ "total": total, "data": data })))
}

pub async fn tags(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(ab): Path<String>,
) -> Result<Json<Value>, AppError> {
    resolve_book(&state, &auth, &ab).await?;
    let rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT name, color FROM ab_tags WHERE ab_guid = ? ORDER BY name COLLATE NOCASE",
    )
    .bind(&ab)
    .fetch_all(&state.db)
    .await?;
    let data: Vec<Value> = rows
        .iter()
        .map(|(name, color)| json!({ "name": name, "color": color }))
        .collect();
    Ok(Json(Value::Array(data)))
}

pub async fn add_peer(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(ab): Path<String>,
    payload: Result<Json<Value>, JsonRejection>,
) -> Result<Json<Value>, AppError> {
    let (book, rule) = resolve_book(&state, &auth, &ab).await?;
    require_write(rule)?;
    let Json(body) = payload
        .map_err(|err| AppError::BadRequest(format!("请求格式错误：{err}")))?;

    let peer_id = string_field(&body, "id");
    validate_peer_id(&peer_id)?;
    if state.cfg.ab_max_peer_one_ab > 0 {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM ab_peers WHERE ab_guid = ?")
            .bind(&ab)
            .fetch_one(&state.db)
            .await?;
        if count >= state.cfg.ab_max_peer_one_ab {
            return Err(AppError::BadRequest("地址簿条目已达上限。".to_owned()));
        }
    }
    let exists: Option<i64> =
        sqlx::query_scalar("SELECT 1 FROM ab_peers WHERE ab_guid = ? AND peer_id = ?")
            .bind(&ab)
            .bind(&peer_id)
            .fetch_optional(&state.db)
            .await?;
    if exists.is_some() {
        return Err(AppError::Conflict("该设备已在地址簿中。".to_owned()));
    }

    let personal = book.is_personal != 0 && book.owner == auth.username;
    let hash = if personal {
        string_field(&body, "hash")
    } else {
        String::new()
    };
    let password = if book.is_personal != 0 {
        String::new()
    } else {
        string_field(&body, "password")
    };
    let tags = sanitize_tags(&state, &ab, &body).await?;
    let tags = serde_json::to_string(&tags).unwrap_or_else(|_| "[]".to_owned());
    let now = now_secs();

    sqlx::query(
        "INSERT INTO ab_peers
            (ab_guid, peer_id, hash, password, username, hostname, platform, alias, tags,
             force_always_relay, rdp_port, rdp_username, login_name, device_group_name, note,
             created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&ab)
    .bind(&peer_id)
    .bind(hash)
    .bind(password)
    .bind(text_field(&body, "username"))
    .bind(text_field(&body, "hostname"))
    .bind(string_field(&body, "platform"))
    .bind(text_field(&body, "alias"))
    .bind(tags)
    .bind(if bool_field(&body, "forceAlwaysRelay") {
        1
    } else {
        0
    })
    .bind(string_field(&body, "rdpPort"))
    .bind(text_field(&body, "rdpUsername"))
    .bind(text_field(&body, "loginName"))
    .bind(text_field(&body, "device_group_name"))
    .bind(text_field(&body, "note"))
    .bind(now)
    .bind(now)
    .execute(&state.db)
    .await?;

    Ok(Json(json!({ "success": true })))
}

pub async fn update_peer(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(ab): Path<String>,
    payload: Result<Json<Value>, JsonRejection>,
) -> Result<Json<Value>, AppError> {
    let (book, rule) = resolve_book(&state, &auth, &ab).await?;
    require_write(rule)?;
    let Json(body) = payload
        .map_err(|err| AppError::BadRequest(format!("请求格式错误：{err}")))?;

    let peer_id = string_field(&body, "id");
    if peer_id.is_empty() {
        return Err(AppError::BadRequest("缺少设备 ID。".to_owned()));
    }
    let personal = book.is_personal != 0 && book.owner == auth.username;

    let mut qb = QueryBuilder::new("UPDATE ab_peers SET updated_at = ");
    qb.push_bind(now_secs());

    let text_fields = [
        ("alias", "alias"),
        ("username", "username"),
        ("hostname", "hostname"),
        ("platform", "platform"),
        ("rdpPort", "rdp_port"),
        ("rdpUsername", "rdp_username"),
        ("loginName", "login_name"),
        ("device_group_name", "device_group_name"),
        ("note", "note"),
        ("hash", "hash"),
        ("password", "password"),
    ];
    for (key, column) in text_fields {
        if !body.get(key).is_some_and(Value::is_string) {
            continue;
        }
        if key == "hash" && !personal {
            continue;
        }
        if key == "password" && book.is_personal != 0 {
            continue;
        }
        qb.push(format!(", {column} = "));
        qb.push_bind(text_field(&body, key));
    }
    if body.get("forceAlwaysRelay").is_some() {
        qb.push(", force_always_relay = ");
        qb.push_bind(if bool_field(&body, "forceAlwaysRelay") {
            1
        } else {
            0
        });
    }
    if body.get("tags").is_some() {
        let tags = sanitize_tags(&state, &ab, &body).await?;
        qb.push(", tags = ");
        qb.push_bind(serde_json::to_string(&tags).unwrap_or_else(|_| "[]".to_owned()));
    }

    qb.push(" WHERE ab_guid = ");
    qb.push_bind(&ab);
    qb.push(" AND peer_id = ");
    qb.push_bind(&peer_id);
    let result = qb.build().execute(&state.db).await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("未找到地址簿条目。".to_owned()));
    }

    Ok(Json(json!({ "success": true })))
}

pub async fn delete_peers(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(ab): Path<String>,
    payload: Result<Json<Vec<String>>, JsonRejection>,
) -> Result<Json<Value>, AppError> {
    let (_, rule) = resolve_book(&state, &auth, &ab).await?;
    require_write(rule)?;
    let Json(ids) = payload
        .map_err(|err| AppError::BadRequest(format!("请求格式错误：{err}")))?;

    let mut tx = state.db.begin().await?;
    for peer_id in &ids {
        let peer_id = peer_id.trim();
        if peer_id.is_empty() {
            continue;
        }
        sqlx::query("DELETE FROM ab_peers WHERE ab_guid = ? AND peer_id = ?")
            .bind(&ab)
            .bind(peer_id)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;

    Ok(Json(json!({ "success": true })))
}

pub async fn add_tag(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(ab): Path<String>,
    payload: Result<Json<TagPayload>, JsonRejection>,
) -> Result<Json<Value>, AppError> {
    let (_, rule) = resolve_book(&state, &auth, &ab).await?;
    require_write(rule)?;
    let Json(request) = payload
        .map_err(|err| AppError::BadRequest(format!("请求格式错误：{err}")))?;
    let name = request.name.trim();
    validate_tag_name(name)?;
    let color = request.color.unwrap_or_else(|| default_tag_color(name));

    match sqlx::query(
        "INSERT INTO ab_tags (ab_guid, name, color, created_at) VALUES (?, ?, ?, ?)",
    )
    .bind(&ab)
    .bind(name)
    .bind(color)
    .bind(now_secs())
    .execute(&state.db)
    .await
    {
        Ok(_) => Ok(Json(json!({ "success": true }))),
        Err(sqlx::Error::Database(err)) if err.is_unique_violation() => {
            Err(AppError::Conflict("同名标签已存在。".to_owned()))
        }
        Err(err) => Err(err.into()),
    }
}

pub async fn rename_tag(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(ab): Path<String>,
    payload: Result<Json<RenameTagPayload>, JsonRejection>,
) -> Result<Json<Value>, AppError> {
    let (_, rule) = resolve_book(&state, &auth, &ab).await?;
    require_write(rule)?;
    let Json(request) = payload
        .map_err(|err| AppError::BadRequest(format!("请求格式错误：{err}")))?;
    let old = request.old.trim();
    let new = request.new.trim();
    validate_tag_name(old)?;
    validate_tag_name(new)?;
    if old == new {
        return Ok(Json(json!({ "success": true })));
    }

    let exists: Option<i64> =
        sqlx::query_scalar("SELECT 1 FROM ab_tags WHERE ab_guid = ? AND name = ?")
            .bind(&ab)
            .bind(new)
            .fetch_optional(&state.db)
            .await?;
    if exists.is_some() {
        return Err(AppError::Conflict("同名标签已存在。".to_owned()));
    }

    let mut tx = state.db.begin().await?;
    let updated = sqlx::query("UPDATE ab_tags SET name = ? WHERE ab_guid = ? AND name = ?")
        .bind(new)
        .bind(&ab)
        .bind(old)
        .execute(&mut *tx)
        .await?;
    if updated.rows_affected() == 0 {
        return Err(AppError::NotFound("未找到标签。".to_owned()));
    }
    rename_tag_in_peers(&mut tx, &ab, old, new).await?;
    tx.commit().await?;

    Ok(Json(json!({ "success": true })))
}

pub async fn update_tag(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(ab): Path<String>,
    payload: Result<Json<TagPayload>, JsonRejection>,
) -> Result<Json<Value>, AppError> {
    let (_, rule) = resolve_book(&state, &auth, &ab).await?;
    require_write(rule)?;
    let Json(request) = payload
        .map_err(|err| AppError::BadRequest(format!("请求格式错误：{err}")))?;
    let name = request.name.trim();
    validate_tag_name(name)?;
    let color = request.color.unwrap_or_else(|| default_tag_color(name));

    let result = sqlx::query("UPDATE ab_tags SET color = ? WHERE ab_guid = ? AND name = ?")
        .bind(color)
        .bind(&ab)
        .bind(name)
        .execute(&state.db)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("未找到标签。".to_owned()));
    }

    Ok(Json(json!({ "success": true })))
}

pub async fn delete_tag(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(ab): Path<String>,
    payload: Result<Json<Vec<String>>, JsonRejection>,
) -> Result<Json<Value>, AppError> {
    let (_, rule) = resolve_book(&state, &auth, &ab).await?;
    require_write(rule)?;
    let Json(names) = payload
        .map_err(|err| AppError::BadRequest(format!("请求格式错误：{err}")))?;

    let mut tx = state.db.begin().await?;
    for name in &names {
        let name = name.trim();
        if name.is_empty() {
            continue;
        }
        sqlx::query("DELETE FROM ab_tags WHERE ab_guid = ? AND name = ?")
            .bind(&ab)
            .bind(name)
            .execute(&mut *tx)
            .await?;
        delete_tag_from_peers(&mut tx, &ab, name).await?;
    }
    tx.commit().await?;

    Ok(Json(json!({ "success": true })))
}

async fn ensure_personal_book(state: &AppState, auth: &AuthUser) -> Result<String, AppError> {
    if let Some(guid) = sqlx::query_scalar::<_, String>(
        "SELECT guid FROM address_books WHERE owner = ? AND is_personal = 1",
    )
    .bind(&auth.username)
    .fetch_optional(&state.db)
    .await?
    {
        return Ok(guid);
    }

    let guid = uuid::Uuid::new_v4().to_string();
    let now = now_secs();
    let name = format!("Personal address book ({})", auth.username);
    sqlx::query(
        "INSERT OR IGNORE INTO address_books
            (guid, name, owner, note, info, is_personal, created_at, updated_at)
         VALUES (?, ?, ?, '', '{}', 1, ?, ?)",
    )
    .bind(&guid)
    .bind(&name)
    .bind(&auth.username)
    .bind(now)
    .bind(now)
    .execute(&state.db)
    .await?;

    let stored = sqlx::query_scalar::<_, String>(
        "SELECT guid FROM address_books WHERE owner = ? AND is_personal = 1",
    )
    .bind(&auth.username)
    .fetch_one(&state.db)
    .await?;
    Ok(stored)
}

async fn resolve_book(
    state: &AppState,
    auth: &AuthUser,
    guid: &str,
) -> Result<(BookRow, i64), AppError> {
    let book: Option<BookRow> = sqlx::query_as(
        "SELECT guid, name, owner, note, info, is_personal, created_at, updated_at
         FROM address_books WHERE guid = ?",
    )
    .bind(guid)
    .fetch_optional(&state.db)
    .await?;
    let book = book.ok_or_else(|| AppError::NotFound("未找到地址簿。".to_owned()))?;

    if book.is_personal != 0 {
        if book.owner == auth.username {
            return Ok((book, 3));
        }
        return Err(AppError::NotFound("未找到地址簿。".to_owned()));
    }
    if auth.is_admin {
        return Ok((book, 3));
    }

    let rule: Option<i64> =
        sqlx::query_scalar("SELECT rule FROM ab_shares WHERE ab_guid = ? AND user_id = ?")
            .bind(guid)
            .bind(auth.id)
            .fetch_optional(&state.db)
            .await?;
    let rule = rule.ok_or_else(|| AppError::NotFound("未找到地址簿。".to_owned()))?;
    Ok((book, rule))
}

async fn sanitize_tags(
    state: &AppState,
    ab: &str,
    body: &Value,
) -> Result<Vec<String>, AppError> {
    let Some(tags) = body.get("tags").and_then(Value::as_array) else {
        return Ok(Vec::new());
    };
    let known: Vec<String> = sqlx::query_scalar("SELECT name FROM ab_tags WHERE ab_guid = ?")
        .bind(ab)
        .fetch_all(&state.db)
        .await?;
    let known: HashSet<String> = known.into_iter().collect();
    let mut result = Vec::new();
    for tag in tags {
        let Some(tag) = tag.as_str() else {
            continue;
        };
        let tag = tag.trim();
        if tag.is_empty() || tag.chars().count() > 64 {
            continue;
        }
        if known.contains(tag) && !result.iter().any(|existing| existing == tag) {
            result.push(tag.to_owned());
        }
    }
    Ok(result)
}

async fn rename_tag_in_peers(
    tx: &mut sqlx::Transaction<'_, Sqlite>,
    ab: &str,
    old: &str,
    new: &str,
) -> Result<(), AppError> {
    let peers: Vec<(i64, String)> =
        sqlx::query_as("SELECT id, tags FROM ab_peers WHERE ab_guid = ?")
            .bind(ab)
            .fetch_all(&mut **tx)
            .await?;
    for (id, tags) in peers {
        let mut tags = parse_tags(&tags);
        let mut changed = false;
        for tag in tags.iter_mut() {
            if tag == old {
                *tag = new.to_owned();
                changed = true;
            }
        }
        if changed {
            let tags = serde_json::to_string(&tags).unwrap_or_else(|_| "[]".to_owned());
            sqlx::query("UPDATE ab_peers SET tags = ? WHERE id = ?")
                .bind(tags)
                .bind(id)
                .execute(&mut **tx)
                .await?;
        }
    }
    Ok(())
}

async fn delete_tag_from_peers(
    tx: &mut sqlx::Transaction<'_, Sqlite>,
    ab: &str,
    name: &str,
) -> Result<(), AppError> {
    let peers: Vec<(i64, String)> =
        sqlx::query_as("SELECT id, tags FROM ab_peers WHERE ab_guid = ?")
            .bind(ab)
            .fetch_all(&mut **tx)
            .await?;
    for (id, tags) in peers {
        let mut tags = parse_tags(&tags);
        let before = tags.len();
        tags.retain(|tag| tag != name);
        if tags.len() != before {
            let tags = serde_json::to_string(&tags).unwrap_or_else(|_| "[]".to_owned());
            sqlx::query("UPDATE ab_peers SET tags = ? WHERE id = ?")
                .bind(tags)
                .bind(id)
                .execute(&mut **tx)
                .await?;
        }
    }
    Ok(())
}

fn peer_json(row: &PeerRow, personal: bool) -> Value {
    json!({
        "id": row.peer_id,
        "hash": if personal { row.hash.clone() } else { String::new() },
        "password": if personal { String::new() } else { row.password.clone() },
        "username": row.username,
        "hostname": row.hostname,
        "platform": row.platform,
        "alias": row.alias,
        "tags": parse_tags(&row.tags),
        "forceAlwaysRelay": if row.force_always_relay != 0 { "true" } else { "false" },
        "rdpPort": row.rdp_port,
        "rdpUsername": row.rdp_username,
        "loginName": row.login_name,
        "device_group_name": row.device_group_name,
        "note": row.note,
        "same_server": row.same_server != 0,
    })
}

fn parse_tags(raw: &str) -> Vec<String> {
    serde_json::from_str::<Vec<String>>(raw).unwrap_or_default()
}

fn string_field(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_owned()
}

fn text_field(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

fn bool_field(value: &Value, key: &str) -> bool {
    match value.get(key) {
        Some(Value::Bool(value)) => *value,
        Some(Value::String(value)) => value == "true" || value == "1",
        Some(Value::Number(value)) => value.as_i64().unwrap_or(0) != 0,
        _ => false,
    }
}

fn validate_peer_id(peer_id: &str) -> Result<(), AppError> {
    if peer_id.is_empty() {
        return Err(AppError::BadRequest("缺少设备 ID。".to_owned()));
    }
    if peer_id.chars().count() > 64 {
        return Err(AppError::BadRequest("设备 ID 过长。".to_owned()));
    }
    if peer_id.chars().any(char::is_whitespace) {
        return Err(AppError::BadRequest("设备 ID 不能包含空格。".to_owned()));
    }
    Ok(())
}

fn validate_tag_name(name: &str) -> Result<(), AppError> {
    if name.is_empty() {
        return Err(AppError::BadRequest("标签名称不能为空。".to_owned()));
    }
    if name.chars().count() > 64 {
        return Err(AppError::BadRequest("标签名称过长。".to_owned()));
    }
    Ok(())
}

fn require_write(rule: i64) -> Result<(), AppError> {
    if rule >= 2 {
        Ok(())
    } else {
        Err(AppError::Forbidden("该地址簿为只读。".to_owned()))
    }
}

fn default_tag_color(name: &str) -> i64 {
    const COLORS: [i64; 10] = [
        0xFF2563EB, 0xFF7C3AED, 0xFFDB2777, 0xFFDC2626, 0xFFEA580C, 0xFFCA8A04, 0xFF16A34A,
        0xFF0D9488, 0xFF0284C7, 0xFF4F46E5,
    ];
    let hash = name.bytes().fold(0u64, |acc, byte| {
        acc.wrapping_mul(31).wrapping_add(u64::from(byte))
    });
    COLORS[(hash % COLORS.len() as u64) as usize]
}

pub async fn list_books(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<BookListQuery>,
) -> Result<Json<Value>, AppError> {
    require_admin(&auth)?;
    let current = query.current.unwrap_or(1).max(1);
    let page_size = query.page_size.unwrap_or(50).clamp(1, 200);
    let offset = (current - 1) * page_size;
    let term = query.q.unwrap_or_default().trim().to_owned();

    let mut count =
        QueryBuilder::new("SELECT COUNT(*) FROM address_books b WHERE b.is_personal = 0");
    push_search(&mut count, &term);
    let total: i64 = count.build_query_scalar().fetch_one(&state.db).await?;

    let mut list = QueryBuilder::new(
        "SELECT b.guid, b.name, b.owner, b.note, b.info, b.created_at, b.updated_at,
                (SELECT COUNT(*) FROM ab_peers p WHERE p.ab_guid = b.guid) AS peer_count,
                (SELECT COUNT(*) FROM ab_shares s WHERE s.ab_guid = b.guid) AS share_count
         FROM address_books b WHERE b.is_personal = 0",
    );
    push_search(&mut list, &term);
    list.push(" ORDER BY b.name COLLATE NOCASE, b.guid LIMIT ");
    list.push_bind(page_size);
    list.push(" OFFSET ");
    list.push_bind(offset);
    let rows: Vec<BookListRow> = list.build_query_as().fetch_all(&state.db).await?;

    let data: Vec<Value> = rows
        .iter()
        .map(|row| {
            json!({
                "guid": row.guid,
                "name": row.name,
                "owner": row.owner,
                "note": row.note,
                "info": parse_info(&row.info),
                "peer_count": row.peer_count,
                "share_count": row.share_count,
                "created_at": iso_from_secs(Some(row.created_at)),
                "updated_at": iso_from_secs(Some(row.updated_at)),
            })
        })
        .collect();
    Ok(Json(json!({ "total": total, "data": data })))
}

pub async fn create_book(
    State(state): State<AppState>,
    auth: AuthUser,
    payload: Result<Json<BookPayload>, JsonRejection>,
) -> Result<Json<Value>, AppError> {
    require_admin(&auth)?;
    let Json(request) = payload
        .map_err(|err| AppError::BadRequest(format!("请求格式错误：{err}")))?;
    let name = request.name.trim();
    validate_book_name(name)?;
    let note = request.note.unwrap_or_default();
    validate_note(&note)?;
    let password = request.password.unwrap_or_default();
    validate_password(&password)?;

    let info = if password.trim().is_empty() {
        Map::new()
    } else {
        let mut map = Map::new();
        map.insert(
            "password".to_owned(),
            Value::String(password.trim().to_owned()),
        );
        map
    };
    let guid = uuid::Uuid::new_v4().to_string();
    let now = now_secs();

    match sqlx::query(
        "INSERT INTO address_books
            (guid, name, owner, note, info, is_personal, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, 0, ?, ?)",
    )
    .bind(&guid)
    .bind(name)
    .bind(&auth.username)
    .bind(&note)
    .bind(Value::Object(info.clone()).to_string())
    .bind(now)
    .bind(now)
    .execute(&state.db)
    .await
    {
        Ok(_) => {}
        Err(sqlx::Error::Database(err)) if err.is_unique_violation() => {
            return Err(AppError::Conflict("同名地址簿已存在。".to_owned()));
        }
        Err(err) => return Err(err.into()),
    }

    super::audit(
        &state,
        &auth.username,
        "address_book.create",
        &guid,
        name,
    )
    .await;

    Ok(Json(json!({
        "guid": guid,
        "name": name,
        "owner": auth.username,
        "note": note,
        "info": Value::Object(info),
        "peer_count": 0,
        "share_count": 0,
        "created_at": iso_from_secs(Some(now)),
        "updated_at": iso_from_secs(Some(now)),
    })))
}

pub async fn update_book(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(guid): Path<String>,
    payload: Result<Json<BookUpdatePayload>, JsonRejection>,
) -> Result<Json<Value>, AppError> {
    require_admin(&auth)?;
    let book = load_shared_book(&state, &guid).await?;
    let Json(request) = payload
        .map_err(|err| AppError::BadRequest(format!("请求格式错误：{err}")))?;

    let name = match request.name {
        Some(name) => {
            let name = name.trim().to_owned();
            validate_book_name(&name)?;
            name
        }
        None => book.name.clone(),
    };
    let note = request.note.unwrap_or_else(|| book.note.clone());
    validate_note(&note)?;

    let mut info = parse_info(&book.info)
        .as_object()
        .cloned()
        .unwrap_or_default();
    if let Some(password) = request.password {
        let password = password.trim();
        validate_password(password)?;
        if password.is_empty() {
            info.remove("password");
        } else {
            info.insert(
                "password".to_owned(),
                Value::String(password.to_owned()),
            );
        }
    }

    match sqlx::query(
        "UPDATE address_books
         SET name = ?, note = ?, info = ?, updated_at = ?
         WHERE guid = ? AND is_personal = 0",
    )
    .bind(&name)
    .bind(&note)
    .bind(Value::Object(info).to_string())
    .bind(now_secs())
    .bind(&guid)
    .execute(&state.db)
    .await
    {
        Ok(_) => {}
        Err(sqlx::Error::Database(err)) if err.is_unique_violation() => {
            return Err(AppError::Conflict("同名地址簿已存在。".to_owned()));
        }
        Err(err) => return Err(err.into()),
    }

    super::audit(
        &state,
        &auth.username,
        "address_book.update",
        &guid,
        &name,
    )
    .await;
    Ok(Json(json!({ "success": true })))
}

pub async fn delete_book(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(guid): Path<String>,
) -> Result<Json<Value>, AppError> {
    require_admin(&auth)?;
    let book = load_shared_book(&state, &guid).await?;
    sqlx::query("DELETE FROM address_books WHERE guid = ? AND is_personal = 0")
        .bind(&guid)
        .execute(&state.db)
        .await?;
    super::audit(
        &state,
        &auth.username,
        "address_book.delete",
        &guid,
        &book.name,
    )
    .await;
    Ok(Json(json!({ "success": true })))
}

pub async fn get_shares(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(guid): Path<String>,
) -> Result<Json<Value>, AppError> {
    require_admin(&auth)?;
    load_shared_book(&state, &guid).await?;
    let rows: Vec<(i64, String, String, i64)> = sqlx::query_as(
        "SELECT s.user_id, u.username, u.display_name, s.rule
         FROM ab_shares s
         JOIN users u ON u.id = s.user_id
         WHERE s.ab_guid = ?
         ORDER BY u.username COLLATE NOCASE",
    )
    .bind(&guid)
    .fetch_all(&state.db)
    .await?;
    let shares: Vec<Value> = rows
        .iter()
        .map(|(user_id, name, display_name, rule)| {
            json!({
                "user_id": user_id,
                "name": name,
                "display_name": display_name,
                "rule": rule,
            })
        })
        .collect();
    Ok(Json(json!({ "shares": shares })))
}

pub async fn set_shares(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(guid): Path<String>,
    payload: Result<Json<SharesPayload>, JsonRejection>,
) -> Result<Json<Value>, AppError> {
    require_admin(&auth)?;
    load_shared_book(&state, &guid).await?;
    let Json(request) = payload
        .map_err(|err| AppError::BadRequest(format!("请求格式错误：{err}")))?;

    let mut shares: HashMap<i64, i64> = HashMap::new();
    for entry in request.shares {
        if entry.user_id <= 0 {
            continue;
        }
        if !(1..=3).contains(&entry.rule) {
            return Err(AppError::BadRequest("共享权限无效。".to_owned()));
        }
        shares.insert(entry.user_id, entry.rule);
    }
    for user_id in shares.keys() {
        let exists: Option<i64> = sqlx::query_scalar("SELECT 1 FROM users WHERE id = ?")
            .bind(user_id)
            .fetch_optional(&state.db)
            .await?;
        if exists.is_none() {
            return Err(AppError::BadRequest("用户不存在。".to_owned()));
        }
    }

    let now = now_secs();
    let mut tx = state.db.begin().await?;
    sqlx::query("DELETE FROM ab_shares WHERE ab_guid = ?")
        .bind(&guid)
        .execute(&mut *tx)
        .await?;
    for (user_id, rule) in &shares {
        sqlx::query(
            "INSERT INTO ab_shares (ab_guid, user_id, rule, created_at) VALUES (?, ?, ?, ?)",
        )
        .bind(&guid)
        .bind(user_id)
        .bind(rule)
        .bind(now)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;

    super::audit(
        &state,
        &auth.username,
        "address_book.share",
        &guid,
        &shares.len().to_string(),
    )
    .await;
    Ok(Json(json!({ "success": true })))
}

async fn load_shared_book(state: &AppState, guid: &str) -> Result<BookRow, AppError> {
    let book: Option<BookRow> = sqlx::query_as(
        "SELECT guid, name, owner, note, info, is_personal, created_at, updated_at
         FROM address_books WHERE guid = ? AND is_personal = 0",
    )
    .bind(guid)
    .fetch_optional(&state.db)
    .await?;
    book.ok_or_else(|| AppError::NotFound("未找到地址簿。".to_owned()))
}

fn push_search(qb: &mut QueryBuilder<'_, Sqlite>, term: &str) {
    if term.is_empty() {
        return;
    }
    let pattern = format!("%{term}%");
    qb.push(" AND (b.name LIKE ");
    qb.push_bind(pattern.clone());
    qb.push(" OR b.owner LIKE ");
    qb.push_bind(pattern);
    qb.push(")");
}

fn validate_book_name(name: &str) -> Result<(), AppError> {
    if name.is_empty() {
        return Err(AppError::BadRequest("地址簿名称不能为空。".to_owned()));
    }
    if name.chars().count() > 64 {
        return Err(AppError::BadRequest("地址簿名称过长。".to_owned()));
    }
    Ok(())
}

fn validate_note(note: &str) -> Result<(), AppError> {
    if note.chars().count() > 240 {
        return Err(AppError::BadRequest("备注过长。".to_owned()));
    }
    Ok(())
}

fn validate_password(password: &str) -> Result<(), AppError> {
    if password.chars().count() > 128 {
        return Err(AppError::BadRequest("密码过长。".to_owned()));
    }
    Ok(())
}

fn require_admin(auth: &AuthUser) -> Result<(), AppError> {
    if auth.is_admin {
        Ok(())
    } else {
        Err(AppError::Forbidden("需要管理员权限。".to_owned()))
    }
}
