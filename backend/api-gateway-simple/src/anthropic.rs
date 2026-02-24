// ========== Anthropic API 完整兼容模块 ==========
// 功能：为 Claude Code 提供完整的 Anthropic API v1 兼容

use axum::{
    extract::{Request, State, Extension},
    http::{HeaderMap, StatusCode},
    response::{Json, IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::collections::HashMap;

use crate::{AppState, AuthenticatedUser, AuthError, ChatRequest, map_anthropic_model};

// ========== 请求结构 ==========

/// Anthropic Messages API 完整请求
#[derive(Debug, Clone, Deserialize)]
pub struct AnthropicMessagesRequest {
    pub model: String,
    pub messages: Vec<AnthropicMessage>,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    #[serde(default)]
    pub stream: bool,
    #[serde(default)]
    pub system: Option<String>,
    #[serde(default)]
    pub temperature: Option<f32>,
    #[serde(default)]
    pub top_p: Option<f32>,
    #[serde(default)]
    pub top_k: Option<u32>,
    #[serde(default)]
    pub tools: Option<Vec<Tool>>,
    #[serde(default)]
    pub tool_choice: Option<ToolChoice>,
    #[serde(default)]
    pub metadata: Option<Metadata>,
}

fn default_max_tokens() -> u32 { 4096 }

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AnthropicMessage {
    pub role: String,
    #[serde(default)]
    pub content: MessageContent,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum MessageContent {
    Text(String),
    Blocks(Vec<ContentBlock>),
}

impl Default for MessageContent {
    fn default() -> Self {
        MessageContent::Text(String::new())
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image")]
    Image { source: ImageSource },
    #[serde(rename = "tool_use")]
    ToolUse { id: String, name: String, input: serde_json::Value },
    #[serde(rename = "tool_result")]
    ToolResult { tool_use_id: String, content: String, is_error: Option<bool> },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ImageSource {
    #[serde(rename = "type")]
    pub media_type: String,
    pub data: String,
    pub media_type: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Tool {
    #[serde(rename = "type")]
    pub tool_type: String, // "custom"
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum ToolChoice {
    Auto { #[serde(rename = "type")] choice_type: String }, // "auto"
    Any { #[serde(rename = "type")] choice_type: String }, // "any"
    Tool { #[serde(rename = "type")] choice_type: String, name: String }, // "tool"
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Metadata {
    #[serde(default)]
    pub user_id: Option<String>,
}

// ========== 响应结构 ==========

/// Anthropic Messages API 完整响应
#[derive(Debug, Serialize)]
pub struct AnthropicMessagesResponse {
    pub id: String,
    #[serde(rename = "type")]
    pub response_type: String,
    pub role: String,
    pub content: Vec<ResponseContentBlock>,
    pub model: String,
    pub stop_reason: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequence: Option<String>,
    pub usage: AnthropicUsage,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _pending: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _request_id: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum ResponseContentBlock {
    Text { #[serde(rename = "type")] content_type: String, text: String },
    ToolUse { #[serde(rename = "type")] content_type: String, id: String, name: String, input: serde_json::Value },
}

#[derive(Debug, Serialize)]
pub struct AnthropicUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

/// 流式响应事件
#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum StreamEvent {
    #[serde(rename = "message_start")]
    MessageStart { message: StreamMessageInfo },
    #[serde(rename = "message_delta")]
    MessageDelta { delta: StreamDelta, usage: Option<StreamUsage> },
    #[serde(rename = "message_stop")]
    MessageStop,
    #[serde(rename = "content_block_start")]
    ContentBlockStart { index: u32, content_block: StreamContentBlock },
    #[serde(rename = "content_block_delta")]
    ContentBlockDelta { index: u32, delta: StreamDelta },
    #[serde(rename = "content_block_stop")]
    ContentBlockStop { index: u32 },
    #[serde(rename = "error")]
    Error { error: StreamError },
}

#[derive(Debug, Serialize)]
pub struct StreamMessageInfo {
    pub id: String,
    #[serde(rename = "type")]
    pub message_type: String,
    pub role: String,
    pub content: Vec<StreamContentBlock>,
    pub model: String,
    pub stop_reason: Option<String>,
    pub usage: Option<AnthropicUsage>,
}

#[derive(Debug, Serialize)]
pub struct StreamContentBlock {
    #[serde(rename = "type")]
    pub content_type: String,
    pub text: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct StreamDelta {
    #[serde(rename = "type")]
    pub delta_type: String,
    pub text: Option<String>,
    pub stop_reason: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct StreamUsage {
    pub output_tokens: u32,
}

#[derive(Debug, Serialize)]
pub struct StreamError {
    #[serde(rename = "type")]
    pub error_type: String,
    pub message: String,
}

// ========== 错误响应 ==========

/// Anthropic 标准错误响应
#[derive(Debug, Serialize)]
pub struct AnthropicErrorResponse {
    #[serde(rename = "type")]
    pub error_type: String,
    pub error: AnthropicErrorDetail,
}

#[derive(Debug, Serialize)]
pub struct AnthropicErrorDetail {
    #[serde(rename = "type")]
    pub detail_type: String,
    pub message: String,
}

#[derive(Debug)]
pub enum AnthropicErrorType {
    InvalidRequestError,
    AuthenticationError,
    PermissionError,
    NotFoundError,
    RateLimitError,
    ApiError,
    OverloadedError,
}

impl AnthropicErrorType {
    pub fn as_str(&self) -> &'static str {
        match self {
            AnthropicErrorType::InvalidRequestError => "invalid_request_error",
            AnthropicErrorType::AuthenticationError => "authentication_error",
            AnthropicErrorType::PermissionError => "permission_error",
            AnthropicErrorType::NotFoundError => "not_found_error",
            AnthropicErrorType::RateLimitError => "rate_limit_error",
            AnthropicErrorType::ApiError => "api_error",
            AnthropicErrorType::OverloadedError => "overloaded_error",
        }
    }
}

impl IntoResponse for AnthropicErrorResponse {
    fn into_response(self) -> Response {
        let status = match self.error.detail_type.as_str() {
            "invalid_request_error" => StatusCode::BAD_REQUEST,
            "authentication_error" => StatusCode::UNAUTHORIZED,
            "permission_error" => StatusCode::FORBIDDEN,
            "not_found_error" => StatusCode::NOT_FOUND,
            "rate_limit_error" => StatusCode::TOO_MANY_REQUESTS,
            "overloaded_error" => StatusCode::SERVICE_UNAVAILABLE,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };

        (status, Json(self)).into_response()
    }
}

impl From<AuthError> for AnthropicErrorResponse {
    fn from(err: AuthError) -> Self {
        match err {
            AuthError::InvalidToken(msg) => AnthropicErrorResponse {
                error_type: "error".to_string(),
                error: AnthropicErrorDetail {
                    detail_type: AnthropicErrorType::AuthenticationError.as_str().to_string(),
                    message: msg,
                },
            },
            AuthError::Forbidden => AnthropicErrorResponse {
                error_type: "error".to_string(),
                error: AnthropicErrorDetail {
                    detail_type: AnthropicErrorType::PermissionError.as_str().to_string(),
                    message: "Insufficient credits".to_string(),
                },
            },
            _ => AnthropicErrorResponse {
                error_type: "error".to_string(),
                error: AnthropicErrorDetail {
                    detail_type: AnthropicErrorType::ApiError.as_str().to_string(),
                    message: err.to_string(),
                },
            },
        }
    }
}

// ========== 辅助函数 ==========

/// 生成消息 ID（格式: msg_<base64>）
pub fn generate_message_id() -> String {
    format!("msg_{}", Uuid::new_v4())
}

/// 将 Anthropic 消息转换为内部 ChatMessage
pub fn anthropic_message_to_chat(msg: &AnthropicMessage) -> crate::ChatMessage {
    let content = match &msg.content {
        MessageContent::Text(text) => text.clone(),
        MessageContent::Blocks(blocks) => {
            blocks.iter()
                .filter_map(|b| match b {
                    ContentBlock::Text { text } => Some(text.clone()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("\n")
        }
    };
    crate::ChatMessage {
        role: msg.role.clone(),
        content,
    }
}

/// 将内部响应转换为 Anthropic 格式
pub fn create_anthropic_response(
    request_model: String,
    content: String,
    input_tokens: u32,
    output_tokens: u32,
    stop_reason: Option<String>,
) -> AnthropicMessagesResponse {
    AnthropicMessagesResponse {
        id: generate_message_id(),
        response_type: "message".to_string(),
        role: "assistant".to_string(),
        content: vec![
            ResponseContentBlock::Text {
                content_type: "text".to_string(),
                text: content,
            }
        ],
        model: request_model,
        stop_reason: stop_reason.unwrap_or("end_turn".to_string()),
        stop_sequence: None,
        usage: AnthropicUsage {
            input_tokens,
            output_tokens,
        },
        _pending: None,
        _request_id: Some(Uuid::new_v4().to_string()),
    }
}

/// 创建流式响应的 SSE 格式
pub fn create_sse_event(event_type: &str, data: &str) -> String {
    format!("event: {}\ndata: {}\n\n", event_type, data)
}

// ========== Handler ==========

/// Anthropic Messages API Handler（完整版）
pub async fn anthropic_messages_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Extension(user): Extension<AuthenticatedUser>,
    Json(req): AnthropicMessagesRequest,
) -> Result<Json<AnthropicMessagesResponse>, AnthropicErrorResponse> {
    use tracing::{info, error};

    let request_id = Uuid::new_v4();
    let start_time = std::time::Instant::now();

    info!(
        "Anthropic API 请求: request_id={}, user={}, model={}, stream={}",
        request_id, user.user_id, req.model, req.stream
    );

    // 验证请求
    if req.messages.is_empty() {
        return Err(AnthropicErrorResponse {
            error_type: "error".to_string(),
            error: AnthropicErrorDetail {
                detail_type: AnthropicErrorType::InvalidRequestError.as_str().to_string(),
                message: "messages array must not be empty".to_string(),
            },
        });
    }

    if req.max_tokens == 0 {
        return Err(AnthropicErrorResponse {
            error_type: "error".to_string(),
            error: AnthropicErrorDetail {
                detail_type: AnthropicErrorType::InvalidRequestError.as_str().to_string(),
                message: "max_tokens must be greater than 0".to_string(),
            },
        });
    }

    // 映射模型到池 ID
    let pool_id = map_anthropic_model(&req.model);

    // 转换消息格式
    let chat_req = ChatRequest {
        model: pool_id.clone(),
        messages: req.messages.iter()
            .map(anthropic_message_to_chat)
            .collect(),
        stream: req.stream,
        max_tokens: req.max_tokens,
    };

    // 估算输入 tokens
    let estimated_input_tokens: u32 = chat_req.messages.iter()
        .map(|m| m.content.len() as u32 / 4)
        .sum();

    // 检查余额
    let estimated_cost = crate::calculate_cost(&pool_id, estimated_input_tokens, req.max_tokens);
    if user.balance < estimated_cost {
        return Err(AnthropicErrorResponse {
            error_type: "error".to_string(),
            error: AnthropicErrorDetail {
                detail_type: AnthropicErrorType::InvalidRequestError.as_str().to_string(),
                message: "Insufficient credits".to_string(),
            },
        });
    }

    // 调用上游 API
    let api_result = state.pool_manager.call_api(&user.user_id, &chat_req, &pool_id).await;

    let (response_text, actual_input_tokens, actual_output_tokens, _, _, actual_provider, _) = match api_result {
        Ok(result) => {
            info!(
                "API 调用成功: request_id={}, pool={}, provider={}, time={}ms",
                request_id, result.pool_id, result.provider_id, result.response_time_ms
            );
            (result.content, result.input_tokens, result.output_tokens,
             result.pool_id, result.task_sequence, result.provider_id, result.used_fallback)
        }
        Err(e) => {
            error!("API 调用失败: request_id={}, error={}", request_id, e);
            return Err(AnthropicErrorResponse {
                error_type: "error".to_string(),
                error: AnthropicErrorDetail {
                    detail_type: AnthropicErrorType::ApiError.as_str().to_string(),
                    message: format!("API call failed: {}", e),
                },
            });
        }
    };

    // 使用实际或估算的 tokens
    let input_tokens = if actual_input_tokens > 0 { actual_input_tokens } else { estimated_input_tokens };
    let output_tokens = if actual_output_tokens > 0 { actual_output_tokens } else { (response_text.len() as u32 / 4) + 1 };
    let actual_cost = crate::calculate_cost(&pool_id, input_tokens, output_tokens);

    // 扣费
    if let Err(e) = sqlx::query(
        "UPDATE users SET balance = balance - $1, updated_at_old = EXTRACT(EPOCH FROM NOW())::BIGINT, updated_at = NOW()
         WHERE user_id = $2"
    )
    .bind(actual_cost)
    .bind(&user.user_id)
    .execute(&state.db)
    .await
    {
        error!("扣费失败: request_id={}, error={}", request_id, e);
    }

    let elapsed = start_time.elapsed().as_millis();
    info!(
        "请求完成: request_id={}, cost={}, elapsed={}ms",
        request_id, actual_cost, elapsed
    );

    // 返回 Anthropic 格式响应
    Ok(Json(create_anthropic_response(
        req.model,
        response_text,
        input_tokens,
        output_tokens,
        Some("end_turn".to_string()),
    )))
}
