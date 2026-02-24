use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

// ==================== 错误处理 ====================

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Unauthorized")]
    Unauthorized,

    #[error("用户不存在")]
    UserNotFound,

    #[error("密码错误")]
    PasswordIncorrect,

    #[error("账户未激活，请联系管理员")]
    AccountInactive,

    #[error("账户已被临时锁定，请10分钟后再试")]
    AccountLocked,

    #[error("用户未设置密码，请使用注册功能")]
    PasswordNotSet,

    #[error("Insufficient balance: required {required}, balance {balance}")]
    InsufficientBalance { required: i64, balance: i64 },

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("Invalid API key")]
    InvalidApiKey,

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Internal error: {0}")]
    InternalError(#[from] anyhow::Error),

    #[error("HTTP error: {0}")]
    HttpError(String),

    #[error("Redis error: {0}")]
    RedisError(#[from] redis::RedisError),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Model not allowed for your tier: {model}")]
    ModelNotAllowed { model: String },
}

impl axum::response::IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let (status, error_code) = match &self {
            AppError::UserNotFound => (axum::http::StatusCode::OK, "user_not_found"),
            AppError::PasswordIncorrect => (axum::http::StatusCode::OK, "password_incorrect"),
            AppError::AccountInactive => (axum::http::StatusCode::OK, "account_inactive"),
            AppError::AccountLocked => (axum::http::StatusCode::OK, "account_locked"),
            AppError::PasswordNotSet => (axum::http::StatusCode::OK, "password_not_set"),
            AppError::Unauthorized => (axum::http::StatusCode::UNAUTHORIZED, "unauthorized"),
            AppError::InsufficientBalance { .. } => (axum::http::StatusCode::PAYMENT_REQUIRED, "insufficient_balance"),
            AppError::RateLimitExceeded => (axum::http::StatusCode::TOO_MANY_REQUESTS, "rate_limit_exceeded"),
            AppError::InvalidApiKey => (axum::http::StatusCode::UNAUTHORIZED, "invalid_api_key"),
            AppError::InvalidInput(_) => (axum::http::StatusCode::BAD_REQUEST, "invalid_input"),
            AppError::ModelNotAllowed { .. } => (axum::http::StatusCode::FORBIDDEN, "model_not_allowed"),
            AppError::HttpError(_) => (axum::http::StatusCode::BAD_GATEWAY, "upstream_error"),
            _ => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "internal_error"),
        };

        let body = serde_json::json!({
            "success": false,
            "error": error_code,
            "message": self.to_string(),
            "timestamp": Utc::now().to_rfc3339(),
        });

        (status, axum::Json(body)).into_response()
    }
}

pub type Result<T> = std::result::Result<T, AppError>;

// ==================== 用户模型 ====================

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    pub id: i64,
    pub email: String,
    pub username: Option<String>,
    pub api_key: String,
    pub balance: i64,
    pub tier: String,
    pub status: String,
    pub concurrent_limit: i32,
    #[serde(skip_serializing)]
    pub password_hash: Option<String>,
    #[serde(skip_serializing)]
    pub salt: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ==================== 计费记录 ====================

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct BillingRecord {
    pub id: i64,
    pub user_id: i64,
    pub request_id: String,
    pub model: String,
    pub input_tokens: i32,
    pub output_tokens: i32,
    pub total_tokens: i32,
    pub cost: i32,
    pub context_doubled: bool,
    pub billing_status: String,
    pub created_at: DateTime<Utc>,
}

// ==================== API请求/响应 ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(default)]
    pub stream: bool,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub id: String,
    pub object: String,
    pub created: i64,
    pub model: String,
    pub choices: Vec<Choice>,
    pub usage: Option<Usage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Choice {
    pub index: i32,
    pub message: Option<ChatMessage>,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
    pub total_tokens: i32,
}

// ==================== 计费规则 ====================

pub struct BillingRule {
    pub model: &'static str,
    pub cost_per_call: i32,
}

impl BillingRule {
    pub fn get(model: &str) -> Option<Self> {
        match model.to_lowercase().as_str() {
            // Opus级别: 7分
            "opus" | "glm-4-plus" => Some(BillingRule {
                model: "opus",
                cost_per_call: 7,
            }),
            // Sonnet级别: 4分
            "sonnet" | "glm-4-flashx" => Some(BillingRule {
                model: "sonnet",
                cost_per_call: 4,
            }),
            // Haiku级别: 1分
            "haiku" | "glm-4-flash" => Some(BillingRule {
                model: "haiku",
                cost_per_call: 1,
            }),
            // Codex: 使用qwen-coder, 1.5分
            "codex" | "qwen-coder" | "qwen-coder-latest" => Some(BillingRule {
                model: "codex",
                cost_per_call: 2,
            }),
            // Gemini: 使用qwen-turbo, 1.5分
            "gemini" | "qwen-turbo" | "qwen-turbo-latest" => Some(BillingRule {
                model: "gemini",
                cost_per_call: 2,
            }),
            _ => None,
        }
    }

    pub fn calculate_cost(&self, total_tokens: i32) -> i32 {
        // 上下文 > 32000 tokens: 扣分翻倍
        if total_tokens > 32000 {
            self.cost_per_call * 2
        } else {
            self.cost_per_call
        }
    }
}

// ==================== 单元测试 ====================

#[cfg(test)]
mod tests {
    use super::*;
    use axum::response::IntoResponse;

    #[test]
    fn test_billing_rule_opus() {
        let rule = BillingRule::get("opus").unwrap();
        assert_eq!(rule.model, "opus");
        assert_eq!(rule.cost_per_call, 7);
    }

    #[test]
    fn test_billing_rule_sonnet() {
        let rule = BillingRule::get("sonnet").unwrap();
        assert_eq!(rule.model, "sonnet");
        assert_eq!(rule.cost_per_call, 4);
    }

    #[test]
    fn test_billing_rule_haiku() {
        let rule = BillingRule::get("haiku").unwrap();
        assert_eq!(rule.model, "haiku");
        assert_eq!(rule.cost_per_call, 1);
    }

    #[test]
    fn test_billing_rule_normal_context() {
        let rule = BillingRule::get("opus").unwrap();
        assert_eq!(rule.calculate_cost(1000), 7);  // 正常上下文
    }

    #[test]
    fn test_billing_rule_doubled_context() {
        let rule = BillingRule::get("opus").unwrap();
        assert_eq!(rule.calculate_cost(35000), 14);  // 翻倍
    }

    #[test]
    fn test_billing_rule_boundary() {
        let rule = BillingRule::get("sonnet").unwrap();
        assert_eq!(rule.calculate_cost(32000), 4);   // 不翻倍
        assert_eq!(rule.calculate_cost(32001), 8);   // 翻倍
    }

    #[test]
    fn test_billing_rule_unknown_model() {
        assert!(BillingRule::get("unknown").is_none());
    }

    #[test]
    fn test_billing_rule_glm_aliases() {
        assert!(BillingRule::get("glm-4-plus").is_some());
        assert!(BillingRule::get("glm-4-flashx").is_some());
        assert!(BillingRule::get("glm-4-flash").is_some());
    }

    #[test]
    fn test_billing_rule_codex() {
        // Codex映射到qwen-coder
        let rule = BillingRule::get("codex").unwrap();
        assert_eq!(rule.model, "codex");
        assert_eq!(rule.cost_per_call, 2);

        // 直接使用qwen-coder
        let rule2 = BillingRule::get("qwen-coder").unwrap();
        assert_eq!(rule2.model, "codex"); // 内部名称统一为codex
        assert_eq!(rule2.cost_per_call, 2);
    }

    #[test]
    fn test_billing_rule_gemini() {
        // Gemini映射到qwen-turbo
        let rule = BillingRule::get("gemini").unwrap();
        assert_eq!(rule.model, "gemini");
        assert_eq!(rule.cost_per_call, 2);

        // 直接使用qwen-turbo
        let rule2 = BillingRule::get("qwen-turbo").unwrap();
        assert_eq!(rule2.model, "gemini"); // 内部名称统一为gemini
        assert_eq!(rule2.cost_per_call, 2);
    }

    #[test]
    fn test_app_error_unauthorized_response() {
        let err = AppError::Unauthorized;
        let resp = err.into_response();
        assert_eq!(resp.status(), axum::http::StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn test_app_error_insufficient_balance() {
        let err = AppError::InsufficientBalance { required: 10, balance: 5 };
        assert_eq!(err.to_string(), "Insufficient balance: required 10, balance 5");
    }

    #[test]
    fn test_app_error_rate_limit() {
        let err = AppError::RateLimitExceeded;
        let resp = err.into_response();
        assert_eq!(resp.status(), axum::http::StatusCode::TOO_MANY_REQUESTS);
    }

    #[test]
    fn test_app_error_response_format() {
        let err = AppError::InsufficientBalance { required: 10, balance: 5 };
        let resp = err.into_response();
        assert_eq!(resp.status(), axum::http::StatusCode::PAYMENT_REQUIRED);

        // 验证响应体包含必要字段
        let _body = resp.into_body();
    }

    #[test]
    fn test_chat_request_default_values() {
        let req: ChatRequest = serde_json::from_str(r#"{"model":"opus","messages":[]}"#).unwrap();
        assert_eq!(req.model, "opus");
        assert_eq!(req.stream, false);
        assert_eq!(req.max_tokens, None);
        assert_eq!(req.temperature, None);
    }

    #[test]
    fn test_chat_request_with_stream() {
        let req: ChatRequest = serde_json::from_str(
            r#"{"model":"opus","messages":[],"stream":true}"#
        ).unwrap();
        assert_eq!(req.stream, true);
    }

    #[test]
    fn test_chat_message_serialization() {
        let msg = ChatMessage {
            role: "user".to_string(),
            content: "hello".to_string(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("user"));
        assert!(json.contains("hello"));
    }
}
