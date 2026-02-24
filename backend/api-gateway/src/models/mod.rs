pub mod error;

pub use error::ApiError;

use serde::{Deserialize, Serialize};

/// Token查询请求
#[derive(Debug, Deserialize)]
pub struct TokenQueryRequest {
    pub model: String,
    pub input_tokens: u32,
    pub output_tokens: u32,
}

/// Token查询响应
#[derive(Debug, Serialize)]
pub struct TokenQueryResponse {
    pub status: String,
    pub cost: i64,
    pub balance: i64,
    pub can_proceed: bool,
}

/// 聊天请求（OpenAI兼容格式）
#[derive(Debug, Deserialize, Clone)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(default)]
    pub stream: bool,
    #[serde(default)]
    pub max_tokens: u32,
    #[serde(default)]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
}

/// 聊天消息
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// 聊天响应（非流式）
#[derive(Debug, Serialize)]
pub struct ChatResponse {
    pub id: String,
    pub object: String,
    pub created: i64,
    pub model: String,
    pub choices: Vec<ChatChoice>,
    pub usage: ChatUsage,
}

/// 聊天选择
#[derive(Debug, Serialize)]
pub struct ChatChoice {
    pub index: i32,
    pub message: ChatMessage,
    pub finish_reason: String,
}

/// Token使用统计
#[derive(Debug, Serialize)]
pub struct ChatUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// 流式聊天响应块
#[derive(Debug, Serialize)]
pub struct ChatStreamChunk {
    pub id: String,
    pub object: String,
    pub created: i64,
    pub model: String,
    pub choices: Vec<StreamChoice>,
}

/// 流式选择
#[derive(Debug, Serialize)]
pub struct StreamChoice {
    pub index: i32,
    pub delta: StreamDelta,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
}

/// 流式增量
#[derive(Debug, Serialize)]
pub struct StreamDelta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
}

/// 模型信息
#[derive(Debug, Serialize, Clone)]
pub struct Model {
    pub id: String,
    pub object: String,
    pub owned_by: String,
}

/// 模型列表响应
#[derive(Debug, Serialize)]
pub struct ModelsResponse {
    pub object: String,
    pub data: Vec<Model>,
}

/// 健康检查响应
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub timestamp: String,
    pub services: ServicesHealth,
}

/// 服务健康状态
#[derive(Debug, Serialize)]
pub struct ServicesHealth {
    pub database: String,
    pub redis: String,
    pub upstream: String,
}

/// 用户信息（从Token解析）
#[derive(Debug, Clone)]
pub struct UserContext {
    pub user_id: i64,
    pub token: String,
    pub tier: UserTier,
}

/// 用户套餐等级
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserTier {
    Base,
    Pro,
    Max,
}

impl UserTier {
    pub fn qps_limit(&self) -> usize {
        match self {
            UserTier::Base => 10,
            UserTier::Pro => 100,
            UserTier::Max => 200,
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "base" => Some(UserTier::Base),
            "pro" => Some(UserTier::Pro),
            "max" => Some(UserTier::Max),
            _ => None,
        }
    }
}

/// 计费记录
#[derive(Debug, Clone)]
pub struct BillingRecord {
    pub user_id: i64,
    pub model: String,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub cost: i64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Token价格配置
#[derive(Debug, Clone)]
pub struct TokenPricing {
    pub input_price_per_million: i64,
    pub output_price_per_million: i64,
}

impl TokenPricing {
    pub fn calculate_cost(&self, input_tokens: u32, output_tokens: u32) -> i64 {
        let input_cost = (input_tokens as i64 * self.input_price_per_million) / 1_000_000;
        let output_cost = (output_tokens as i64 * self.output_price_per_million) / 1_000_000;
        input_cost + output_cost
    }
}

/// 模型配置
#[derive(Debug, Clone)]
pub struct ModelConfig {
    pub name: String,
    pub pricing: TokenPricing,
    pub max_tokens: u32,
}

/// 请求元数据（用于响应头）
#[derive(Debug, Clone)]
pub struct RequestMetadata {
    pub request_id: String,
    pub user_id: i64,
    pub start_time: chrono::DateTime<chrono::Utc>,
    pub model: String,
}

/// 限流结果
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RateLimitResult {
    Allowed,
    Limited { retry_after: u64 },
}

// ===== Anthropic API 格式支持 =====

/// Anthropic Messages API 请求
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AnthropicRequest {
    pub model: String,
    pub messages: Vec<AnthropicMessage>,
    #[serde(default)]
    pub max_tokens: u32,
    #[serde(default)]
    pub stream: bool,
    #[serde(default)]
    pub temperature: Option<f32>,
    #[serde(default)]
    pub top_p: Option<f32>,
    #[serde(default)]
    pub top_k: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    #[serde(default)]
    pub tools: Vec<AnthropicTool>,
}

/// Anthropic 消息
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AnthropicMessage {
    pub role: String,
    pub content: AnthropicContent,
}

/// Anthropic 内容（支持文本和工具使用）
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(untagged)]
pub enum AnthropicContent {
    Simple(String),
    Blocks(Vec<AnthropicContentBlock>),
}

/// Anthropic 内容块
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AnthropicContentBlock {
    #[serde(rename = "type")]
    pub block_type: String,
    pub text: Option<String>,
    pub source: Option<AnthropicImageSource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub name: Option<String>,
    pub input: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_use_id: Option<String>,
    pub content: Option<Vec<AnthropicContentBlock>>,
    pub is_error: Option<bool>,
}

/// Anthropic 图片源
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AnthropicImageSource {
    #[serde(rename = "type")]
    pub source_type: String,
    pub media_type: String,
    pub data: String,
}

/// Anthropic 工具定义
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AnthropicTool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

/// Anthropic 响应（非流式）
#[derive(Debug, Serialize, Deserialize)]
pub struct AnthropicResponse {
    pub id: String,
    #[serde(rename = "type")]
    pub response_type: String,
    pub role: String,
    pub content: Vec<AnthropicContentBlock>,
    pub model: String,
    pub stop_reason: String,
    pub usage: AnthropicUsage,
}

/// Anthropic 使用统计
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AnthropicUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

/// Anthropic 流式事件
#[derive(Debug, Serialize, Clone)]
#[serde(untagged)]
pub enum AnthropicStreamEvent {
    #[serde(rename = "message_start")]
    MessageStart { message: AnthropicMessageStart },
    #[serde(rename = "message_delta")]
    MessageDelta { delta: AnthropicDelta, usage: AnthropicUsage },
    #[serde(rename = "message_stop")]
    MessageStop,
    #[serde(rename = "content_block_start")]
    ContentBlockStart { index: u32, content_block: Option<AnthropicContentBlock>, delta: Option<serde_json::Value> },
    #[serde(rename = "content_block_delta")]
    ContentBlockDelta { index: u32, delta: AnthropicDeltaContent },
    #[serde(rename = "content_block_stop")]
    ContentBlockStop { index: u32 },
    #[serde(rename = "ping")]
    Ping { id: String, ping_type: String },
    #[serde(rename = "error")]
    Error { error: AnthropicErrorInfo },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AnthropicMessageStart {
    pub id: String,
    #[serde(rename = "type")]
    pub message_type: String,
    pub role: String,
    pub content: Vec<AnthropicContentBlock>,
    pub model: String,
    pub stop_reason: Option<String>,
    pub usage: AnthropicUsage,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AnthropicDelta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequence: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AnthropicDeltaContent {
    #[serde(rename = "type")]
    pub delta_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub partial_json: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AnthropicErrorInfo {
    #[serde(rename = "type")]
    pub error_type: String,
    pub message: String,
}

/// 将 OpenAI 格式的 ChatMessage 转换为 Anthropic 格式
impl From<&ChatMessage> for AnthropicMessage {
    fn from(msg: &ChatMessage) -> Self {
        AnthropicMessage {
            role: msg.role.clone(),
            content: AnthropicContent::Simple(msg.content.clone()),
        }
    }
}

/// 模型名称映射：将 Claude Code 使用的完整模型名映射到内部简称
pub fn normalize_model_name(model: &str) -> String {
    // 处理 claude-3-5-sonnet-20241022 -> sonnet-4-5 或类似格式
    let model_lower = model.to_lowercase();

    if model_lower.contains("opus") || model_lower.contains("claude-opus") {
        "claude-opus-4-6".to_string()
    } else if model_lower.contains("sonnet") || model_lower.contains("claude-sonnet") {
        "claude-sonnet-4-5-20241022".to_string()
    } else if model_lower.contains("haiku") || model_lower.contains("claude-haiku") {
        "claude-haiku-4-5-20251001".to_string()
    } else {
        model.to_string()
    }
}

/// 将模型名称转换为上游 API 需要的格式
pub fn to_upstream_model_name(model: &str) -> String {
    let model_lower = model.to_lowercase();

    if model_lower.contains("opus") {
        "claude-3-5-sonnet-20241022".to_string() // 使用实际可用的模型
    } else if model_lower.contains("sonnet") {
        "claude-3-5-sonnet-20241022".to_string()
    } else if model_lower.contains("haiku") {
        "claude-3-5-haiku-20241022".to_string()
    } else {
        model.to_string()
    }
}
