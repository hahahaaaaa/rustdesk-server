use axum::extract::FromRequestParts;
use axum::http::{header, request::Parts};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::models::{now_secs, UserRow};
use crate::AppState;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub uid: i64,
    pub adm: bool,
    pub jti: String,
    pub iat: i64,
    pub exp: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub purpose: Option<String>,
}

pub const TFA_CHALLENGE_TTL_SECS: i64 = 300;

#[derive(Debug, Clone)]
pub struct AuthUser {
    pub id: i64,
    pub username: String,
    pub is_admin: bool,
    pub jti: String,
    pub exp: i64,
}

pub fn create_token(state: &AppState, user: &UserRow) -> Result<String, AppError> {
    let issued_at = now_secs();
    let claims = Claims {
        sub: user.username.clone(),
        uid: user.id,
        adm: user.is_admin != 0,
        jti: uuid::Uuid::new_v4().simple().to_string(),
        iat: issued_at,
        exp: issued_at + state.cfg.token_ttl_seconds(),
        purpose: None,
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(state.jwt_secret.as_bytes()),
    )
    .map_err(|err| AppError::Internal(format!("failed to sign access token: {err}")))
}

pub fn create_tfa_challenge(
    state: &AppState,
    user: &UserRow,
) -> Result<(String, Claims), AppError> {
    let issued_at = now_secs();
    let claims = Claims {
        sub: user.username.clone(),
        uid: user.id,
        adm: false,
        jti: uuid::Uuid::new_v4().simple().to_string(),
        iat: issued_at,
        exp: issued_at + TFA_CHALLENGE_TTL_SECS,
        purpose: Some("tfa".to_owned()),
    };
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(state.jwt_secret.as_bytes()),
    )
    .map_err(|err| AppError::Internal(format!("failed to sign two-factor challenge: {err}")))?;
    Ok((token, claims))
}

pub fn decode_tfa_challenge(state: &AppState, token: &str) -> Result<Claims, AppError> {
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(state.jwt_secret.as_bytes()),
        &Validation::new(Algorithm::HS256),
    )
    .map_err(|_| AppError::Unauthorized("两步验证会话无效或已过期，请重新登录。".to_owned()))?;
    if token_data.claims.purpose.as_deref() != Some("tfa") {
        return Err(AppError::Unauthorized(
            "两步验证会话无效或已过期，请重新登录。".to_owned(),
        ));
    }
    Ok(token_data.claims)
}

pub async fn revoke_token(state: &AppState, jti: &str, exp: i64) -> Result<(), AppError> {
    let now = now_secs();
    sqlx::query("INSERT OR REPLACE INTO revoked_tokens (jti, expires_at) VALUES (?, ?)")
        .bind(jti)
        .bind(exp)
        .execute(&state.db)
        .await?;
    sqlx::query("DELETE FROM revoked_tokens WHERE expires_at < ?")
        .bind(now)
        .execute(&state.db)
        .await?;
    Ok(())
}

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let header = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .ok_or_else(|| AppError::Unauthorized("缺少访问令牌。".to_owned()))?;
        let token = header
            .strip_prefix("Bearer ")
            .or_else(|| header.strip_prefix("bearer "))
            .ok_or_else(|| AppError::Unauthorized("访问令牌格式不正确。".to_owned()))?;

        let validation = Validation::new(Algorithm::HS256);
        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(state.jwt_secret.as_bytes()),
            &validation,
        )
        .map_err(|_| AppError::Unauthorized("访问令牌无效或已过期。".to_owned()))?;
        let claims = token_data.claims;
        if claims.purpose.is_some() {
            return Err(AppError::Unauthorized("访问令牌无效。".to_owned()));
        }

        let revoked: Option<i64> =
            sqlx::query_scalar("SELECT 1 FROM revoked_tokens WHERE jti = ?")
                .bind(&claims.jti)
                .fetch_optional(&state.db)
                .await?;
        if revoked.is_some() {
            return Err(AppError::Unauthorized("访问令牌已失效。".to_owned()));
        }

        let user: Option<(i64, String, i64, i64)> =
            sqlx::query_as("SELECT id, username, status, is_admin FROM users WHERE id = ?")
                .bind(claims.uid)
                .fetch_optional(&state.db)
                .await?;
        let (id, username, status, is_admin) =
            user.ok_or_else(|| AppError::Unauthorized("账号不存在。".to_owned()))?;
        if status != 1 {
            return Err(AppError::Unauthorized("账号已被禁用。".to_owned()));
        }

        Ok(Self {
            id,
            username,
            is_admin: is_admin != 0,
            jti: claims.jti,
            exp: claims.exp,
        })
    }
}
