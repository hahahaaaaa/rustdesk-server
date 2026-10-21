use axum::{
    extract::{Query, State},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::{QueryBuilder, Sqlite};

use crate::auth::AuthUser;
use crate::error::AppError;
use crate::models::iso_from_secs;
use crate::AppState;

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AuditQuery {
    #[serde(default)]
    pub current: Option<i64>,
    #[serde(default)]
    pub page_size: Option<i64>,
    #[serde(default)]
    pub actor: Option<String>,
    #[serde(default)]
    pub action: Option<String>,
    #[serde(default)]
    pub q: Option<String>,
}

pub async fn list_audit(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<AuditQuery>,
) -> Result<Json<Value>, AppError> {
    require_admin(&auth)?;
    let current = query.current.unwrap_or(1).max(1);
    let page_size = query.page_size.unwrap_or(50).clamp(1, 200);
    let offset = (current - 1) * page_size;

    let mut count = QueryBuilder::new("SELECT COUNT(*) FROM audit_logs a WHERE 1=1");
    push_filters(&mut count, &query);
    let total: i64 = count.build_query_scalar().fetch_one(&state.db).await?;

    let mut list = QueryBuilder::new(
        "SELECT a.id, a.actor, a.action, a.target, a.detail, a.created_at
         FROM audit_logs a WHERE 1=1",
    );
    push_filters(&mut list, &query);
    list.push(" ORDER BY a.id DESC LIMIT ");
    list.push_bind(page_size);
    list.push(" OFFSET ");
    list.push_bind(offset);
    let rows: Vec<(i64, String, String, String, String, i64)> =
        list.build_query_as().fetch_all(&state.db).await?;

    let data: Vec<Value> = rows
        .iter()
        .map(|(id, actor, action, target, detail, created_at)| {
            json!({
                "id": id,
                "actor": actor,
                "action": action,
                "target": target,
                "detail": detail,
                "created_at": iso_from_secs(Some(*created_at)),
            })
        })
        .collect();

    Ok(Json(json!({ "total": total, "data": data })))
}

pub async fn list_actions(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, AppError> {
    require_admin(&auth)?;
    let actions: Vec<String> =
        sqlx::query_scalar("SELECT DISTINCT action FROM audit_logs ORDER BY action")
            .fetch_all(&state.db)
            .await?;
    Ok(Json(json!({ "actions": actions })))
}

fn push_filters(qb: &mut QueryBuilder<'_, Sqlite>, query: &AuditQuery) {
    if let Some(actor) = query.actor.as_deref().map(str::trim).filter(|v| !v.is_empty()) {
        qb.push(" AND a.actor LIKE ");
        qb.push_bind(format!("%{actor}%"));
    }
    if let Some(action) = query.action.as_deref().map(str::trim).filter(|v| !v.is_empty()) {
        qb.push(" AND a.action = ");
        qb.push_bind(action.to_owned());
    }
    if let Some(term) = query.q.as_deref().map(str::trim).filter(|v| !v.is_empty()) {
        let pattern = format!("%{term}%");
        qb.push(" AND (a.target LIKE ");
        qb.push_bind(pattern.clone());
        qb.push(" OR a.detail LIKE ");
        qb.push_bind(pattern);
        qb.push(")");
    }
}

fn require_admin(auth: &AuthUser) -> Result<(), AppError> {
    if auth.is_admin {
        Ok(())
    } else {
        Err(AppError::Forbidden("需要管理员权限。".to_owned()))
    }
}
