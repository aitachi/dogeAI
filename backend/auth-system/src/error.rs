//! 认证系统错误类型

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;
use thiserror::Error;

/// 认证系统错误类型
#[derive(Error, Debug)]
pub enum AuthError {
    #[error("Token缺失或格式无效")]
    MissingToken,

    #[error("Token签名验证失败")]
    InvalidSignature,

    #[error("Token已过期")]
    ExpiredToken,

    #[error("Token尚未生效")]
    NotYetValidToken,

    #[error("Token已被撤销")]
    TokenRevoked,

    #[error("用户账户被暂停: {0}")]
    UserSuspended(String),

    #[error("用户不存在")]
    UserNotFound,

    #[error("权限不足")]
    InsufficientPermissions,

    #[error("模型访问受限: {0}")]
    ModelNotAllowed(String),

    #[error("数据库错误: {0}")]
    DatabaseError(#[from] sqlx::Error),

    #[error("Redis错误: {0}")]
    RedisError(#[from] redis::RedisError),

    #[error("JWT编码/解码错误: {0}")]
    JwtError(#[from] jsonwebtoken::errors::Error),

    #[error("内部错误: {0}")]
    InternalError(#[from] anyhow::Error),
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, error_code, message) = match &self {
            AuthError::MissingToken => (StatusCode::UNAUTHORIZED, "MISSING_TOKEN", self.to_string()),
            AuthError::InvalidSignature => (StatusCode::UNAUTHORIZED, "INVALID_SIGNATURE", self.to_string()),
            AuthError::ExpiredToken => (StatusCode::UNAUTHORIZED, "EXPIRED_TOKEN", self.to_string()),
            AuthError::NotYetValidToken => (StatusCode::UNAUTHORIZED, "NOT_YET_VALID", self.to_string()),
            AuthError::TokenRevoked => (StatusCode::UNAUTHORIZED, "TOKEN_REVOKED", self.to_string()),
            AuthError::UserSuspended(reason) => (StatusCode::FORBIDDEN, "USER_SUSPENDED", reason.clone()),
            AuthError::UserNotFound => (StatusCode::UNAUTHORIZED, "USER_NOT_FOUND", self.to_string()),
            AuthError::InsufficientPermissions => (StatusCode::FORBIDDEN, "INSUFFICIENT_PERMISSIONS", self.to_string()),
            AuthError::ModelNotAllowed(model) => (StatusCode::FORBIDDEN, "MODEL_NOT_ALLOWED", format!("无法访问模型: {}", model)),
            AuthError::DatabaseError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "DATABASE_ERROR", "数据库错误".to_string()),
            AuthError::RedisError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "REDIS_ERROR", "缓存错误".to_string()),
            AuthError::JwtError(_) => (StatusCode::UNAUTHORIZED, "JWT_ERROR", "Token解析错误".to_string()),
            AuthError::InternalError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR", "内部错误".to_string()),
        };

        let body = Json(json!({
            "error": error_code,
            "message": message,
        }));

        (status, body).into_response()
    }
}

pub type AuthResult<T> = Result<T, AuthError>;
