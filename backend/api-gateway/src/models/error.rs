use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

/// API错误类型
#[derive(Error, Debug)]
pub enum ApiError {
    #[error("Unauthorized")]
    Unauthorized,

    #[error("Invalid token: {0}")]
    InvalidToken(String),

    #[error("Insufficient balance")]
    InsufficientBalance { required: i64, balance: i64 },

    #[error("Rate limit exceeded")]
    RateLimitExceeded { limit: usize, used: usize },

    #[error("Upstream error: {0}")]
    Upstream(String),

    #[error("Upstream error: status={status}, message={message}")]
    UpstreamError { status: u16, message: String },

    #[error("Upstream timeout")]
    UpstreamTimeout,

    #[error("Database error: {0}")]
    Database(String),

    #[error("Redis error: {0}")]
    Redis(String),

    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("Model not found: {0}")]
    ModelNotFound(String),

    #[error("Internal error")]
    Internal,

    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error_code, message) = match self {
            ApiError::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                "unauthorized",
                "未授权的访问".to_string(),
            ),
            ApiError::InvalidToken(msg) => (
                StatusCode::UNAUTHORIZED,
                "invalid_token",
                format!("无效的Token: {}", msg),
            ),
            ApiError::InsufficientBalance { required, balance } => (
                StatusCode::PAYMENT_REQUIRED,
                "insufficient_balance",
                format!("余额不足: 需要{}积分, 当前{}积分", required, balance),
            ),
            ApiError::RateLimitExceeded { limit, used } => (
                StatusCode::TOO_MANY_REQUESTS,
                "rate_limit_exceeded",
                format!("请求过于频繁: 限制{} QPS, 已使用{} QPS", limit, used),
            ),
            ApiError::Upstream(msg) => (
                StatusCode::BAD_GATEWAY,
                "upstream_error",
                format!("上游服务异常: {}", msg),
            ),
            ApiError::UpstreamError { status, message } => (
                StatusCode::BAD_GATEWAY,
                "upstream_error",
                format!("上游服务异常 [{}]: {}", status, message),
            ),
            ApiError::UpstreamTimeout => (
                StatusCode::GATEWAY_TIMEOUT,
                "upstream_timeout",
                "上游服务超时".to_string(),
            ),
            ApiError::Database(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "database_error",
                format!("数据库错误: {}", msg),
            ),
            ApiError::Redis(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "redis_error",
                format!("缓存错误: {}", msg),
            ),
            ApiError::InvalidRequest(msg) => (
                StatusCode::BAD_REQUEST,
                "invalid_request",
                format!("请求参数错误: {}", msg),
            ),
            ApiError::ModelNotFound(model) => (
                StatusCode::NOT_FOUND,
                "model_not_found",
                format!("模型不存在: {}", model),
            ),
            ApiError::Internal => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal_error",
                "内部错误".to_string(),
            ),
            ApiError::ServiceUnavailable(msg) => (
                StatusCode::SERVICE_UNAVAILABLE,
                "service_unavailable",
                format!("服务不可用: {}", msg),
            ),
        };

        let body = json!({
            "error": error_code,
            "message": message,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });

        (status, Json(body)).into_response()
    }
}

/// 从其他错误类型转换为ApiError
impl From<redis::RedisError> for ApiError {
    fn from(err: redis::RedisError) -> Self {
        ApiError::Redis(err.to_string())
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(err: sqlx::Error) -> Self {
        ApiError::Database(err.to_string())
    }
}

impl From<reqwest::Error> for ApiError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            ApiError::UpstreamTimeout
        } else {
            ApiError::Upstream(err.to_string())
        }
    }
}
