use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{0}")]
    BadRequest(String),
    #[error("{0}")]
    Unauthorized(String),
    #[error("{0}")]
    Forbidden(String),
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Conflict(String),
    #[error("{0}")]
    TooManyRequests(String),
    #[error("{0}")]
    Internal(String),
    #[error(transparent)]
    Database(#[from] sqlx::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            Self::BadRequest(message) => (StatusCode::BAD_REQUEST, message.clone()),
            Self::Unauthorized(message) => (StatusCode::UNAUTHORIZED, message.clone()),
            Self::Forbidden(message) => (StatusCode::FORBIDDEN, message.clone()),
            Self::NotFound(message) => (StatusCode::NOT_FOUND, message.clone()),
            Self::Conflict(message) => (StatusCode::CONFLICT, message.clone()),
            Self::TooManyRequests(message) => (StatusCode::TOO_MANY_REQUESTS, message.clone()),
            Self::Database(err) => {
                tracing::error!(error = %err, "database error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "数据库错误。".to_owned(),
                )
            }
            Self::Internal(message) => {
                tracing::error!(error = %message, "internal error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "服务器内部错误。".to_owned(),
                )
            }
        };
        (status, Json(json!({ "error": message }))).into_response()
    }
}
