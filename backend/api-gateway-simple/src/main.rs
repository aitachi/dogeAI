// ========== API网关 - JWT认证版 + 多提供商系统 ==========
// 功能: JWT验证、密钥轮换、Token撤销、本地缓存
// 多提供商: GLM智谱AI、千问、DeepSeek自动故障转移

mod jwt;
mod provider;

use axum::{
    extract::{Request, State, Extension, Path},
    http::HeaderMap,
    response::Json,
    routing::{get, post},
    Router,
    middleware::{self, Next},
};
use redis::{AsyncCommands, Client as RedisClient};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::interval;
use tower_http::cors::CorsLayer;
use tracing::{info, warn, error};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use uuid::Uuid;

use jwt::{JwtAuthService, AuthenticatedUser, AuthError, TOKEN_TTL_SECS, KEY_ROTATION_HOURS};
use sha2::{Sha256, Digest};
use hex;
use reqwest::Client;
use rand::Rng;

// ========== 配置 ==========

#[derive(Clone)]
struct Config {
    database_url: String,
    redis_url: String,
    server_addr: SocketAddr,
    jwt_secret: String,
    upstream_api_url: String,
    upstream_api_key: String,
}

impl Config {
    fn from_env() -> Self {
        let jwt_secret = std::env::var("JWT_SECRET")
            .or_else(|_| std::env::var("JWT_SECRET_FILE")
                .map(|path| std::fs::read_to_string(path.trim())
                    .unwrap_or_default()
                    .trim().to_string()))
            .unwrap_or_else(|_| {
                warn!("使用默认JWT密钥，生产环境必须设置JWT_SECRET环境变量");
                "dev-secret-key-change-in-production-32bytes-min".to_string()
            });

        Config {
            database_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgresql://postgres:postgres@localhost/api_gateway".to_string()),
            redis_url: std::env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string()),
            server_addr: std::env::var("SERVER_ADDR")
                .unwrap_or_else(|_| "0.0.0.0:8081".to_string())
                .parse()
                .unwrap_or_else(|_| SocketAddr::from(([0, 0, 0, 0], 8081))),
            jwt_secret,
            upstream_api_url: std::env::var("UPSTREAM_API_URL")
                .unwrap_or_else(|_| "https://api.anthropic.com".to_string()),
            upstream_api_key: std::env::var("UPSTREAM_API_KEY")
                .unwrap_or_else(|_| "".to_string()),
        }
    }
}

// ========== 应用状态 ==========

#[derive(Clone)]
struct AppState {
    db: PgPool,
    redis: RedisClient,
    auth: Arc<JwtAuthService>,
    config: Config,
    http_client: Client,
    pool_manager: Arc<provider::ModelPoolManager>,
}

// ========== 数据模型 ==========

#[derive(Debug, Deserialize)]
struct TokenQueryRequest {
    model: String,
    input_tokens: u32,
    output_tokens: u32,
}

#[derive(Debug, Serialize)]
struct TokenQueryResponse {
    status: String,
    cost: i64,
    balance: i64,
    can_proceed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    #[serde(default)]
    stream: bool,
    #[serde(default = "default_max_tokens")]
    max_tokens: u32,
}

fn default_max_tokens() -> u32 { 2000 }

#[derive(Debug, Serialize)]
struct ChatResponse {
    id: String,
    object: String,
    created: i64,
    model: String,
    choices: Vec<ChatChoice>,
    usage: Usage,
    new_token: Option<String>,  // 自动刷新的新token
}

#[derive(Debug, Serialize)]
struct ChatChoice {
    index: i32,
    message: ChatMessage,
    finish_reason: String,
}

#[derive(Debug, Serialize)]
struct Usage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

// ========== Anthropic API 数据结构 ==========

/// Anthropic 消息内容 - 支持字符串和对象数组两种格式
#[derive(Debug, Clone, Serialize, Deserialize)]
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

impl MessageContent {
    /// 提取纯文本内容
    pub fn to_text(&self) -> String {
        match self {
            MessageContent::Text(s) => s.clone(),
            MessageContent::Blocks(blocks) => {
                blocks.iter()
                    .map(|b| match b {
                        ContentBlock::Text { text } => text.clone(),
                        ContentBlock::Image { .. } => "[图像]".to_string(),
                        ContentBlock::ToolUse { name, .. } => format!("[工具调用: {}]", name),
                        ContentBlock::ToolResult { .. } => "[工具结果]".to_string(),
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageSource {
    #[serde(rename = "type")]
    pub media_type: String,
    pub data: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicMessage {
    pub role: String,
    #[serde(default)]
    pub content: MessageContent,
}

#[derive(Debug, Deserialize)]
struct AnthropicMessagesRequest {
    model: String,
    messages: Vec<AnthropicMessage>,
    #[serde(default = "default_anthropic_max_tokens")]
    max_tokens: u32,
    #[serde(default)]
    stream: bool,
    #[serde(default)]
    system: Option<String>,
    #[serde(default)]
    temperature: Option<f32>,
    #[serde(default)]
    top_p: Option<f32>,
}

fn default_anthropic_max_tokens() -> u32 { 4096 }

#[derive(Debug, Serialize)]
struct AnthropicContentBlock {
    #[serde(rename = "type")]
    content_type: String,
    text: String,
}

#[derive(Debug, Serialize)]
struct AnthropicUsage {
    input_tokens: u32,
    output_tokens: u32,
}

#[derive(Debug, Serialize)]
struct AnthropicMessagesResponse {
    id: String,
    #[serde(rename = "type")]
    response_type: String,
    role: String,
    content: Vec<AnthropicContentBlock>,
    model: String,
    stop_reason: String,
    usage: AnthropicUsage,
}

/// Anthropic 标准错误响应
#[derive(Debug, Serialize)]
struct AnthropicErrorResponse {
    #[serde(rename = "type")]
    error_type: String,
    error: AnthropicErrorDetail,
}

#[derive(Debug, Serialize)]
struct AnthropicErrorDetail {
    #[serde(rename = "type")]
    detail_type: String,
    message: String,
}

impl From<AuthError> for AnthropicErrorResponse {
    fn from(err: AuthError) -> Self {
        let (detail_type, message) = match err {
            AuthError::InvalidToken(msg) => ("authentication_error", msg),
            AuthError::TokenExpired => ("authentication_error", "Token已过期".to_string()),
            AuthError::TokenRevoked => ("authentication_error", "Token已被撤销".to_string()),
            AuthError::UserNotFound(_) => ("authentication_error", "用户不存在".to_string()),
            AuthError::UserSuspended => ("authentication_error", "用户已被禁用".to_string()),
            AuthError::Forbidden => ("permission_error", "权限不足或余额不足".to_string()),
            AuthError::Database(msg) => ("api_error", msg),
            AuthError::Redis(msg) => ("api_error", msg),
            AuthError::Internal(msg) => ("api_error", msg),
        };
        AnthropicErrorResponse {
            error_type: "error".to_string(),
            error: AnthropicErrorDetail {
                detail_type: detail_type.to_string(),
                message,
            },
        }
    }
}

impl axum::response::IntoResponse for AnthropicErrorResponse {
    fn into_response(self) -> axum::response::Response {
        use axum::http::StatusCode;
        use axum::Json;

        let status = match self.error.detail_type.as_str() {
            "authentication_error" => StatusCode::UNAUTHORIZED,
            "permission_error" => StatusCode::FORBIDDEN,
            "invalid_request_error" => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };

        (status, Json(self)).into_response()
    }
}

// ========== 辅助函数 ==========

// 将 Anthropic 消息转换为内部 ChatMessage
fn anthropic_to_chat(msg: &AnthropicMessage) -> ChatMessage {
    ChatMessage {
        role: msg.role.clone(),
        content: msg.content.to_text(),
    }
}

// 将 Anthropic 模型名映射到内部池ID
fn map_anthropic_model(model: &str) -> String {
    // 移除可能的后缀，如 [1m], [200k] 等（Claude Code 有时会添加这些）
    let base_model = if let Some(bracket_pos) = model.find('[') {
        &model[..bracket_pos]
    } else {
        model
    };

    match base_model {
        // ========== Opus 系列（映射到 opus 池） ==========
        "claude-opus-4-20250514" |
        "claude-opus-4-5-20250514" |
        "claude-opus-4-5-20250929" |
        "claude-opus-4" |
        "claude-opus-4-6" |
        "claude-3-opus-20240229" => "opus".to_string(),

        // ========== Sonnet 系列（映射到 sonnet 池） ==========
        "claude-sonnet-4-20250514" |
        "claude-sonnet-4-5-20250514" |
        "claude-sonnet-4-5-20250929" |
        "claude-sonnet-4" |
        "claude-3-5-sonnet-20241022" |
        "claude-3-5-sonnet-20240620" => "sonnet".to_string(),

        // ========== Haiku 系列（映射到 haiku 池） ==========
        "claude-3-5-haiku-20241022" |
        "claude-haiku-4-20250514" |
        "claude-haiku-4-5-20251001" |
        "claude-haiku-4" |
        "claude-3-haiku-20240307" => "haiku".to_string(),

        // ========== 兼容短名称 ==========
        "opus" => "opus".to_string(),
        "sonnet" => "sonnet".to_string(),
        "haiku" => "haiku".to_string(),
        "codex" => "codex".to_string(),

        // ========== 默认使用 sonnet ==========
        _ => "sonnet".to_string(),
    }
}

// Anthropic 官方格式 - https://platform.claude.com/docs/en/api/models/list
#[derive(Debug, Serialize)]
struct ModelInfo {
    id: String,
    created_at: String,
    display_name: String,
    #[serde(rename = "type")]
    model_type: String,
}

#[derive(Debug, Serialize)]
struct ModelsResponse {
    data: Vec<ModelInfo>,
    first_id: String,
    has_more: bool,
    last_id: String,
}

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: String,
    timestamp: String,
    version: String,
    database: String,
    redis: String,
    jwt_key_version: i32,
    cache_stats: CacheStatsResponse,
}

#[derive(Debug, Serialize)]
struct CacheStatsResponse {
    size: u64,
    hit_count: u64,
    miss_count: u64,
    hit_rate: f64,
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
    message: String,
    timestamp: String,
    request_id: String,
}

// ========== 资源池相关响应 ==========

#[derive(Debug, Serialize)]
struct PoolStatusResponse {
    pool_id: String,
    pool_name: String,
    description: Option<String>,
    enabled: bool,
    routing_rules_count: i64,
    fallback_chain: Vec<String>,
}

#[derive(Debug, Serialize)]
struct PoolsListResponse {
    pools: Vec<PoolStatusResponse>,
    total_pools: usize,
    enabled_pools: usize,
    total_providers: usize,
    healthy_providers: usize,
}

#[derive(Debug, Serialize)]
struct UserTaskSequenceResponse {
    user_id: String,
    task_sequence: i64,
    pool_id: String,
    last_updated: i64,
}

// ========== 认证相关的API ==========

#[derive(Debug, Deserialize)]
struct LoginRequest {
    username: String,
    password: String,
}

#[derive(Debug, Serialize)]
struct LoginResponse {
    access_token: String,
    token_type: String,
    expires_in: i64,
    user_info: UserInfoResponse,
}

#[derive(Debug, Serialize)]
struct UserInfoResponse {
    user_id: String,
    username: String,
    tier: String,
    balance: i64,
}

// ========== 申请API相关 ==========

#[derive(Debug, Deserialize)]
struct ApplyRequest {
    email: String,
    username: String,
    #[serde(default)]
    full_name: Option<String>,
    #[serde(default)]
    reason: Option<String>,
}

#[derive(Debug, Serialize)]
struct ApplyResponse {
    status: String,
    message: String,
    api_key: Option<String>,
    user_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct CheckAvailabilityResponse {
    available: bool,
}

#[derive(Debug, Serialize)]
struct TokenRefreshResponse {
    new_token: Option<String>,
    expires_in: i64,
}

#[derive(Debug, Serialize)]
struct AuthTestResponse {
    status: String,
    user: AuthenticatedUserResponse,
    token_remaining_secs: i64,
}

#[derive(Debug, Serialize)]
struct AuthenticatedUserResponse {
    user_id: String,
    username: String,
    tier: String,
}

// ========== 中间件 ==========

/// Anthropic API 专用认证中间件 - 返回 Anthropic 格式错误
async fn anthropic_auth_middleware(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<axum::response::Response, AnthropicErrorResponse> {
    // 支持两种认证方式:
    // 1. Authorization: Bearer <token> (OpenAI 格式)
    // 2. x-api-key: <api_key> (Anthropic 格式 - Claude Code 专用)

    let token = if let Some(api_key) = req.headers()
        .get("x-api-key")
        .and_then(|h| h.to_str().ok())
    {
        // Anthropic 格式: x-api-key 直接作为 API key
        api_key.to_string()
    } else {
        // OpenAI 格式: Authorization: Bearer <token>
        let auth_header = req.headers()
            .get("Authorization")
            .and_then(|h| h.to_str().ok())
            .ok_or_else(|| AnthropicErrorResponse {
                error_type: "error".to_string(),
                error: AnthropicErrorDetail {
                    detail_type: "authentication_error".to_string(),
                    message: "缺少 x-api-key 或 Authorization 头".to_string(),
                },
            })?;

        if !auth_header.starts_with("Bearer ") {
            return Err(AnthropicErrorResponse {
                error_type: "error".to_string(),
                error: AnthropicErrorDetail {
                    detail_type: "authentication_error".to_string(),
                    message: "无效的 Authorization 格式".to_string(),
                },
            });
        }
        auth_header[7..].to_string()
    };

    // 验证Token
    let user_info = match state.auth.verify_token(&token).await {
        Ok(user) => user,
        Err(err) => {
            return Err(AnthropicErrorResponse::from(err));
        }
    };

    // 将用户信息添加到请求扩展
    let auth_user = AuthenticatedUser {
        user_id: user_info.user_id.clone(),
        username: user_info.username.clone(),
        tier: user_info.tier.clone(),
        scopes: user_info.scopes.clone(),
        balance: user_info.balance,
    };

    req.extensions_mut().insert(auth_user);
    req.extensions_mut().insert(user_info);

    Ok(next.run(req).await)
}

/// JWT认证中间件 - 支持 Bearer Token 和 x-api-key (Anthropic 格式)
async fn jwt_auth_middleware(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<axum::response::Response, AuthError> {
    // 跳过健康检查和公开端点
    let path = req.uri().path();
    let public_paths = [
        "/health",
        "/v1/models",
        "/api/apply",
        "/api/apply/check-availability",
        "/api/user/login",      // 公开登录接口
        "/api/user/register",   // 公开注册接口
        "/api/health",          // 公开健康检查
        "/api/stats",           // 公开统计数据
        "/api/admin/stats",     // 管理后台统计
        "/api/admin/users",     // 管理后台用户列表
        "/api/admin/recharge-cards",  // 管理后台充值卡
    ];
    let is_public = public_paths.iter().any(|p| path.starts_with(p)) || path.starts_with("/v1/auth/") || path.starts_with("/api/admin/");
    if is_public {
        return Ok(next.run(req).await);
    }

    // 支持两种认证方式:
    // 1. Authorization: Bearer <token> (OpenAI 格式)
    // 2. x-api-key: <api_key> (Anthropic 格式 - Claude Code 专用)

    let token = if let Some(api_key) = req.headers()
        .get("x-api-key")
        .and_then(|h| h.to_str().ok())
    {
        // Anthropic 格式: x-api-key 直接作为 API key
        api_key.to_string()
    } else {
        // OpenAI 格式: Authorization: Bearer <token>
        let auth_header = req.headers()
            .get("Authorization")
            .and_then(|h| h.to_str().ok())
            .ok_or_else(|| AuthError::InvalidToken("缺少Authorization头或x-api-key头".to_string()))?;

        if !auth_header.starts_with("Bearer ") {
            return Err(AuthError::InvalidToken("无效的Authorization格式".to_string()));
        }
        auth_header[7..].to_string()
    };

    // 验证Token (本地JWT验证 + Redis黑名单)
    let user_info = state.auth.verify_token(&token).await?;

    // 将用户信息添加到请求扩展
    let auth_user = AuthenticatedUser {
        user_id: user_info.user_id.clone(),
        username: user_info.username.clone(),
        tier: user_info.tier.clone(),
        scopes: user_info.scopes.clone(),
        balance: user_info.balance,
    };

    req.extensions_mut().insert(auth_user.clone());
    req.extensions_mut().insert(user_info);

    Ok(next.run(req).await)
}

// ========== Handler ==========

async fn health_handler(State(state): State<AppState>) -> Result<Json<HealthResponse>, AuthError> {
    // 检查数据库连接
    let db_status = sqlx::query("SELECT 1")
        .fetch_one(&state.db)
        .await
        .map(|_| "connected".to_string())
        .unwrap_or_else(|_| "disconnected".to_string());

    // 检查Redis连接
    let redis_status = state.redis
        .get_async_connection()
        .await
        .map(|_: redis::aio::Connection| "connected".to_string())
        .unwrap_or_else(|_| "disconnected".to_string());

    let cache_stats = state.auth.cache_stats();

    Ok(Json(HealthResponse {
        status: "healthy".to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        version: "3.0.0".to_string(),
        database: db_status,
        redis: redis_status,
        jwt_key_version: state.auth.key_version(),
        cache_stats: CacheStatsResponse {
            size: cache_stats.size,
            hit_count: cache_stats.hit_count,
            miss_count: cache_stats.miss_count,
            hit_rate: cache_stats.hit_rate,
        },
    }))
}

async fn models_handler(State(state): State<AppState>) -> Json<ModelsResponse> {
    // 从资源池系统获取可用模型
    let pools = state.pool_manager.list_pools().await;

    // 使用标准 Anthropic 模型名称（兼容 Claude Code）
    let mut models = vec![];

    // 检查每个池是否可用并添加对应的模型名称
    let pool_ids: Vec<String> = pools.iter().map(|p| p.pool_id.clone()).collect();

    // ========== Opus 系列（映射到 opus 池） ==========
    if pool_ids.contains(&"opus".to_string()) {
        models.push(ModelInfo {
            id: "claude-opus-4-6".to_string(),
            created_at: "2026-02-04T00:00:00Z".to_string(),
            display_name: "Claude Opus 4.6".to_string(),
            model_type: "model".to_string(),
        });
        models.push(ModelInfo {
            id: "claude-opus-4-5-20250929".to_string(),
            created_at: "2025-09-29T00:00:00Z".to_string(),
            display_name: "Claude Opus 4.5".to_string(),
            model_type: "model".to_string(),
        });
        models.push(ModelInfo {
            id: "claude-opus-4".to_string(),
            created_at: "2025-05-14T00:00:00Z".to_string(),
            display_name: "Claude Opus 4".to_string(),
            model_type: "model".to_string(),
        });
        models.push(ModelInfo {
            id: "claude-opus-4-20250514".to_string(),
            created_at: "2025-05-14T00:00:00Z".to_string(),
            display_name: "Claude Opus 4 (20250514)".to_string(),
            model_type: "model".to_string(),
        });
        models.push(ModelInfo {
            id: "claude-3-opus-20240229".to_string(),
            created_at: "2024-02-29T00:00:00Z".to_string(),
            display_name: "Claude 3 Opus".to_string(),
            model_type: "model".to_string(),
        });
    }

    // ========== Sonnet 系列（映射到 sonnet 池） ==========
    if pool_ids.contains(&"sonnet".to_string()) {
        models.push(ModelInfo {
            id: "claude-sonnet-4-5-20250929".to_string(),
            created_at: "2025-09-29T00:00:00Z".to_string(),
            display_name: "Claude Sonnet 4.5".to_string(),
            model_type: "model".to_string(),
        });
        models.push(ModelInfo {
            id: "claude-sonnet-4-5-20250914".to_string(),
            created_at: "2025-09-14T00:00:00Z".to_string(),
            display_name: "Claude Sonnet 4.5 (20250914)".to_string(),
            model_type: "model".to_string(),
        });
        models.push(ModelInfo {
            id: "claude-sonnet-4".to_string(),
            created_at: "2025-05-14T00:00:00Z".to_string(),
            display_name: "Claude Sonnet 4".to_string(),
            model_type: "model".to_string(),
        });
        models.push(ModelInfo {
            id: "claude-sonnet-4-20250514".to_string(),
            created_at: "2025-05-14T00:00:00Z".to_string(),
            display_name: "Claude Sonnet 4 (20250514)".to_string(),
            model_type: "model".to_string(),
        });
        models.push(ModelInfo {
            id: "claude-3-5-sonnet-20241022".to_string(),
            created_at: "2024-10-22T00:00:00Z".to_string(),
            display_name: "Claude 3.5 Sonnet (Latest)".to_string(),
            model_type: "model".to_string(),
        });
        models.push(ModelInfo {
            id: "claude-3-5-sonnet-20240620".to_string(),
            created_at: "2024-06-20T00:00:00Z".to_string(),
            display_name: "Claude 3.5 Sonnet".to_string(),
            model_type: "model".to_string(),
        });
    }

    // ========== Haiku 系列（映射到 haiku 池） ==========
    if pool_ids.contains(&"haiku".to_string()) {
        models.push(ModelInfo {
            id: "claude-haiku-4-5-20251001".to_string(),
            created_at: "2025-10-01T00:00:00Z".to_string(),
            display_name: "Claude Haiku 4.5".to_string(),
            model_type: "model".to_string(),
        });
        models.push(ModelInfo {
            id: "claude-haiku-4-20250514".to_string(),
            created_at: "2025-05-14T00:00:00Z".to_string(),
            display_name: "Claude Haiku 4".to_string(),
            model_type: "model".to_string(),
        });
        models.push(ModelInfo {
            id: "claude-3-5-haiku-20241022".to_string(),
            created_at: "2024-10-22T00:00:00Z".to_string(),
            display_name: "Claude 3.5 Haiku".to_string(),
            model_type: "model".to_string(),
        });
        models.push(ModelInfo {
            id: "claude-3-haiku-20240307".to_string(),
            created_at: "2024-03-07T00:00:00Z".to_string(),
            display_name: "Claude 3 Haiku".to_string(),
            model_type: "model".to_string(),
        });
    }

    // 按创建时间降序排序（最新的在前）
    models.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    let first_id = models.first().map(|m| m.id.clone()).unwrap_or_default();
    let last_id = models.last().map(|m| m.id.clone()).unwrap_or_default();

    Json(ModelsResponse {
        data: models,
        first_id,
        has_more: false,
        last_id,
    })
}

/// 登录并获取JWT Token
async fn login_handler(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, AuthError> {
    // 查询用户（包含密码哈希）
    let user = sqlx::query_as::<_, (String, String, String, i64, i32, Option<String>)>(
        "SELECT user_id, username, tier, balance, token_version, password_hash
         FROM users WHERE username = $1 AND active = true"
    )
    .bind(&req.username)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| AuthError::Database(e.to_string()))?
    .ok_or_else(|| AuthError::UserNotFound(req.username.clone()))?;

    // 验证密码
    let stored_hash = user.5.unwrap_or_default();
    let input_hash = format!("{:x}", Sha256::digest(req.password.as_bytes()));

    if stored_hash.is_empty() || stored_hash != input_hash {
        return Err(AuthError::InvalidToken("用户名或密码错误".to_string()));
    }

    // 密码验证通过，生成token
    let token = state.auth.create_token(
        &user.0,
        &user.1,
        &user.2,
        vec!["read".to_string(), "write".to_string()],
        user.4,
    ).map_err(|e| AuthError::Internal(e.to_string()))?;

    Ok(Json(LoginResponse {
        access_token: token,
        token_type: "Bearer".to_string(),
        expires_in: TOKEN_TTL_SECS,
        user_info: UserInfoResponse {
            user_id: user.0,
            username: user.1,
            tier: user.2,
            balance: user.3,
        },
    }))
}

/// ========== 用户 API（兼容 client.html）==========

/// 用户登录响应（兼容前端格式）
#[derive(Debug, Serialize)]
struct UserLoginResponse {
    success: bool,
    data: UserData,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
}

#[derive(Debug, Serialize)]
struct UserData {
    api_key: String,
    user_id: String,
    username: String,
    email: Option<String>,
    tier: String,
    balance: i64,
}

/// 用户登录请求（兼容前端格式）
#[derive(Debug, Deserialize)]
struct UserLoginRequest {
    account: String,  // 可以是用户名或邮箱
    password: String,
}

/// 用户登录（兼容 client.html）
async fn user_login_handler(
    State(state): State<AppState>,
    Json(req): Json<UserLoginRequest>,
) -> Json<UserLoginResponse> {
    // 查询用户
    let user = sqlx::query_as::<_, (String, String, String, i64, i32, Option<String>, Option<String>)>(
        "SELECT user_id, username, tier, balance, token_version, password_hash, email
         FROM users WHERE (username = $1 OR email = $1) AND active = true"
    )
    .bind(&req.account)
    .fetch_optional(&state.db)
    .await;

    let user = match user {
        Ok(Some(u)) => u,
        Ok(None) => {
            return Json(UserLoginResponse {
                success: false,
                data: UserData {
                    api_key: String::new(),
                    user_id: String::new(),
                    username: String::new(),
                    email: None,
                    tier: String::new(),
                    balance: 0,
                },
                error: Some("user_not_found".to_string()),
                message: Some("用户不存在".to_string()),
            });
        }
        Err(_) => {
            return Json(UserLoginResponse {
                success: false,
                data: UserData {
                    api_key: String::new(),
                    user_id: String::new(),
                    username: String::new(),
                    email: None,
                    tier: String::new(),
                    balance: 0,
                },
                error: Some("database_error".to_string()),
                message: Some("数据库错误".to_string()),
            });
        }
    };

    // 验证密码
    let stored_hash = user.5.unwrap_or_default();
    let input_hash = format!("{:x}", Sha256::digest(req.password.as_bytes()));

    if !stored_hash.is_empty() && stored_hash != input_hash {
        return Json(UserLoginResponse {
            success: false,
            data: UserData {
                api_key: String::new(),
                user_id: String::new(),
                username: String::new(),
                email: None,
                tier: String::new(),
                balance: 0,
            },
            error: Some("password_incorrect".to_string()),
            message: Some("密码错误".to_string()),
        });
    }

    // 获取用户的 API Key
    let existing_key = sqlx::query_scalar::<_, Option<String>>(
        "SELECT api_key FROM user_api_keys WHERE user_id = $1 AND is_active = true LIMIT 1"
    )
    .bind(&user.0)
    .fetch_one(&state.db)
    .await
    .ok()
    .flatten();

    let api_key = match existing_key {
        Some(k) => k,
        None => {
            // 没有 API Key，生成 Anthropic 格式 Key
            const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
            const KEY_LEN: usize = 52;
            let key_suffix: String = (0..KEY_LEN)
                .map(|_| {
                    let idx = rand::thread_rng().gen_range(0..CHARSET.len());
                    CHARSET[idx] as char
                })
                .collect();
            let new_key = format!("sk-ant-api03-{}", key_suffix);
            let _ = sqlx::query(
                "INSERT INTO user_api_keys (api_key, user_id, is_active) VALUES ($1, $2, true)"
            )
            .bind(&new_key)
            .bind(&user.0)
            .execute(&state.db)
            .await;
            new_key
        }
    };

    Json(UserLoginResponse {
        success: true,
        data: UserData {
            api_key,
            user_id: user.0,
            username: user.1,
            email: user.6,
            tier: user.2,
            balance: user.3,
        },
        error: None,
        message: None,
    })
}

/// 用户注册请求
#[derive(Debug, Deserialize)]
struct UserRegisterRequest {
    username: String,
    email: String,
    password: String,
}

/// 用户注册（兼容 client.html）
async fn user_register_handler(
    State(state): State<AppState>,
    Json(req): Json<UserRegisterRequest>,
) -> Json<UserLoginResponse> {
    // 验证用户名格式
    if req.username.len() < 3 || req.username.len() > 32 {
        return Json(UserLoginResponse {
            success: false,
            data: UserData {
                api_key: String::new(),
                user_id: String::new(),
                username: String::new(),
                email: None,
                tier: String::new(),
                balance: 0,
            },
            error: Some("invalid_input".to_string()),
            message: Some("用户名长度必须为3-32个字符".to_string()),
        });
    }

    if !req.username.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
        return Json(UserLoginResponse {
            success: false,
            data: UserData {
                api_key: String::new(),
                user_id: String::new(),
                username: String::new(),
                email: None,
                tier: String::new(),
                balance: 0,
            },
            error: Some("invalid_input".to_string()),
            message: Some("用户名只能包含字母、数字、下划线和连字符".to_string()),
        });
    }

    // 验证邮箱格式
    if !req.email.contains('@') || !req.email.contains('.') {
        return Json(UserLoginResponse {
            success: false,
            data: UserData {
                api_key: String::new(),
                user_id: String::new(),
                username: String::new(),
                email: None,
                tier: String::new(),
                balance: 0,
            },
            error: Some("invalid_input".to_string()),
            message: Some("邮箱格式不正确".to_string()),
        });
    }

    // 检查用户名是否已存在
    let exists = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM users WHERE username = $1"
    )
    .bind(&req.username)
    .fetch_one(&state.db)
    .await;

    let exists = match exists {
        Ok(count) => count > 0,
        Err(_) => {
            return Json(UserLoginResponse {
                success: false,
                data: UserData {
                    api_key: String::new(),
                    user_id: String::new(),
                    username: String::new(),
                    email: None,
                    tier: String::new(),
                    balance: 0,
                },
                error: Some("database_error".to_string()),
                message: Some("数据库错误".to_string()),
            });
        }
    };

    if exists {
        return Json(UserLoginResponse {
            success: false,
            data: UserData {
                api_key: String::new(),
                user_id: String::new(),
                username: String::new(),
                email: None,
                tier: String::new(),
                balance: 0,
            },
            error: Some("user_exists".to_string()),
            message: Some("用户名已被占用".to_string()),
        });
    }

    // 生成用户ID和 Anthropic 格式 API Key
    let user_id = format!("user_{}", Uuid::new_v4());
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    const KEY_LEN: usize = 52;
    let key_suffix: String = (0..KEY_LEN)
        .map(|_| {
            let idx = rand::thread_rng().gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect();
    let anthropic_api_key = format!("sk-ant-api03-{}", key_suffix);
    let password_hash = format!("{:x}", Sha256::digest(req.password.as_bytes()));

    // 创建新用户
    let result = sqlx::query(
        "INSERT INTO users (user_id, username, email, tier, balance, token_version, active, password_hash, created_at_old, updated_at_old, created_at, updated_at)
         VALUES ($1, $2, $3, 'base', 10000, 1, true, $4, EXTRACT(EPOCH FROM NOW())::BIGINT, EXTRACT(EPOCH FROM NOW())::BIGINT, NOW(), NOW())"
    )
    .bind(&user_id)
    .bind(&req.username)
    .bind(&req.email)
    .bind(&password_hash)
    .execute(&state.db)
    .await;

    if result.is_err() {
        return Json(UserLoginResponse {
            success: false,
            data: UserData {
                api_key: String::new(),
                user_id: String::new(),
                username: String::new(),
                email: None,
                tier: String::new(),
                balance: 0,
            },
            error: Some("database_error".to_string()),
            message: Some("注册失败".to_string()),
        });
    }

    // 插入 Anthropic 格式 API Key
    let _ = sqlx::query(
        "INSERT INTO user_api_keys (api_key, user_id, is_active) VALUES ($1, $2, true)"
    )
    .bind(&anthropic_api_key)
    .bind(&user_id)
    .execute(&state.db)
    .await;

    info!("新用户注册: username={}, user_id={}, email={}, api_key={}", req.username, user_id, req.email, anthropic_api_key);

    Json(UserLoginResponse {
        success: true,
        data: UserData {
            api_key: anthropic_api_key,
            user_id,
            username: req.username,
            email: Some(req.email),
            tier: "base".to_string(),
            balance: 10000,
        },
        error: None,
        message: None,
    })
}

/// 用户余额响应
#[derive(Debug, Serialize)]
struct BalanceResponse {
    balance: i64,
}

/// 用户余额查询（兼容 client.html）
async fn user_balance_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<BalanceResponse>, AuthError> {
    // 支持短 API Key 和 Authorization Bearer 两种方式
    let api_key = if let Some(key) = headers.get("x-api-key").and_then(|h| h.to_str().ok()) {
        key.to_string()
    } else if let Some(auth) = headers.get("authorization").and_then(|h| h.to_str().ok()) {
        auth.strip_prefix("Bearer ").unwrap_or(auth).to_string()
    } else {
        return Err(AuthError::InvalidToken("缺少认证信息".to_string()));
    };

    // 验证 API Key
    let user_info = state.auth.verify_token(&api_key).await?;

    Ok(Json(BalanceResponse {
        balance: user_info.balance,
    }))
}

/// 使用历史响应
#[derive(Debug, Serialize)]
struct HistoryResponse {
    records: Vec<HistoryRecord>,
}

#[derive(Debug, Serialize)]
struct HistoryRecord {
    created_at: i64,
    model: String,
    input_tokens: u32,
    output_tokens: u32,
    total_tokens: u32,
    cost: i64,
}

/// 用户使用历史（兼容 client.html）
async fn user_history_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<HistoryResponse>, AuthError> {
    let api_key = if let Some(key) = headers.get("x-api-key").and_then(|h| h.to_str().ok()) {
        key.to_string()
    } else if let Some(auth) = headers.get("authorization").and_then(|h| h.to_str().ok()) {
        auth.strip_prefix("Bearer ").unwrap_or(auth).to_string()
    } else {
        return Err(AuthError::InvalidToken("缺少认证信息".to_string()));
    };

    let user_info = state.auth.verify_token(&api_key).await?;

    let records = sqlx::query_as::<_, (i64, String, i32, i32, i32, i64)>(
        "SELECT created_at, model, prompt_tokens, completion_tokens, total_tokens, cost
         FROM request_logs WHERE user_id = $1 ORDER BY created_at DESC LIMIT 100"
    )
    .bind(&user_info.user_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| AuthError::Database(e.to_string()))?
    .into_iter()
    .map(|r| HistoryRecord {
        created_at: r.0,
        model: r.1,
        input_tokens: r.2 as u32,
        output_tokens: r.3 as u32,
        total_tokens: r.4 as u32,
        cost: r.5,
    })
    .collect();

    Ok(Json(HistoryResponse { records }))
}

/// 用户资料响应
#[derive(Debug, Serialize)]
struct UserProfileResponse {
    success: bool,
    data: UserProfileData,
}

#[derive(Debug, Serialize)]
struct UserProfileData {
    user_id: String,
    username: String,
    email: Option<String>,
    tier: String,
    balance: i64,
    created_at: i64,
}

/// 用户资料（需要认证）
async fn user_profile_handler(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Json(req): Json<ProfileRequest>,
) -> Result<Json<UserProfileResponse>, AuthError> {
    // 获取用户资料
    let profile = sqlx::query_as::<_, (String, String, Option<String>, String, i64, i64)>(
        "SELECT user_id, username, email, tier, balance, created_at_old
         FROM users WHERE user_id = $1"
    )
    .bind(&user.user_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| AuthError::Database(e.to_string()))?;

    Ok(Json(UserProfileResponse {
        success: true,
        data: UserProfileData {
            user_id: profile.0,
            username: profile.1,
            email: profile.2,
            tier: profile.3,
            balance: profile.4,
            created_at: profile.5,
        },
    }))
}

#[derive(Debug, Deserialize)]
struct ProfileRequest {
    #[serde(default)]
    username: Option<String>,
}

/// 修改密码响应
#[derive(Debug, Serialize)]
struct ChangePasswordResponse {
    success: bool,
    message: String,
}

#[derive(Debug, Deserialize)]
struct ChangePasswordRequest {
    old_password: String,
    new_password: String,
}

/// 修改密码（需要认证）
async fn user_change_password_handler(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Json(req): Json<ChangePasswordRequest>,
) -> Result<Json<ChangePasswordResponse>, AuthError> {
    if req.new_password.len() < 6 {
        return Ok(Json(ChangePasswordResponse {
            success: false,
            message: "新密码长度至少为6个字符".to_string(),
        }));
    }

    // 获取当前密码哈希
    let current_hash = sqlx::query_scalar::<_, String>(
        "SELECT password_hash FROM users WHERE user_id = $1"
    )
    .bind(&user.user_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| AuthError::Database(e.to_string()))?
    .unwrap_or_default();

    // 验证旧密码
    let old_hash = format!("{:x}", Sha256::digest(req.old_password.as_bytes()));
    if !current_hash.is_empty() && current_hash != old_hash {
        return Ok(Json(ChangePasswordResponse {
            success: false,
            message: "旧密码不正确".to_string(),
        }));
    }

    // 更新密码
    let new_hash = format!("{:x}", Sha256::digest(req.new_password.as_bytes()));
    sqlx::query("UPDATE users SET password_hash = $1 WHERE user_id = $2")
        .bind(&new_hash)
        .bind(&user.user_id)
        .execute(&state.db)
        .await
        .map_err(|e| AuthError::Database(e.to_string()))?;

    Ok(Json(ChangePasswordResponse {
        success: true,
        message: "密码修改成功".to_string(),
    }))
}

/// 兑值卡响应
#[derive(Debug, Serialize)]
struct RedeemResponse {
    success: bool,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    amount: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct RedeemRequest {
    code: String,
}

/// 充值卡兑换（需要认证）
async fn recharge_redeem_handler(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Json(req): Json<RedeemRequest>,
) -> Result<Json<RedeemResponse>, AuthError> {
    // 检查充值卡是否存在
    let card = sqlx::query_as::<_, (i32, i64, bool)>(
        "SELECT card_id, amount, used FROM recharge_cards WHERE card_code = $1"
    )
    .bind(&req.code.trim())
    .fetch_optional(&state.db)
    .await
    .map_err(|e| AuthError::Database(e.to_string()))?;

    let card = match card {
        Some(c) => c,
        None => {
            return Ok(Json(RedeemResponse {
                success: false,
                message: "充值卡不存在".to_string(),
                amount: None,
            }));
        }
    };

    if card.2 {
        return Ok(Json(RedeemResponse {
            success: false,
            message: "充值卡已被使用".to_string(),
            amount: None,
        }));
    }

    // 开启事务
    let mut tx = state.db.begin().await
        .map_err(|e| AuthError::Database(e.to_string()))?;

    // 标记充值卡为已使用
    sqlx::query("UPDATE recharge_cards SET used = true, used_by = $1, used_at = NOW() WHERE card_code = $2")
        .bind(&user.user_id)
        .bind(&req.code.trim())
        .execute(&mut *tx)
        .await
        .map_err(|e| AuthError::Database(e.to_string()))?;

    // 增加用户余额
    sqlx::query("UPDATE users SET balance = balance + $1 WHERE user_id = $2")
        .bind(card.1)
        .bind(&user.user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| AuthError::Database(e.to_string()))?;

    tx.commit().await.map_err(|e| AuthError::Database(e.to_string()))?;

    info!("用户 {} 使用充值卡充值 {} 积分", user.username, card.1);

    Ok(Json(RedeemResponse {
        success: true,
        message: "充值成功".to_string(),
        amount: Some(card.1),
    }))
}

/// 公开健康检查
async fn public_health_handler() -> Json<PublicHealthResponse> {
    Json(PublicHealthResponse {
        status: "healthy".to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

/// 公开健康检查响应
#[derive(Debug, Serialize)]
struct PublicHealthResponse {
    status: String,
    timestamp: String,
    version: String,
}

/// 统计数据响应
#[derive(Debug, Serialize)]
struct StatsResponse {
    total_users: i64,
    active_users: i64,
    total_requests: i64,
}

/// 公开统计数据
async fn public_stats_handler(
    State(state): State<AppState>,
) -> Result<Json<StatsResponse>, AuthError> {
    let total_users = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM users")
        .fetch_one(&state.db)
        .await
        .map_err(|e| AuthError::Database(e.to_string()))?;

    let active_users = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM users WHERE active = true")
        .fetch_one(&state.db)
        .await
        .map_err(|e| AuthError::Database(e.to_string()))?;

    let total_requests = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM request_logs")
        .fetch_one(&state.db)
        .await
        .map_err(|e| AuthError::Database(e.to_string()))?;

    Ok(Json(StatsResponse {
        total_users,
        active_users,
        total_requests,
    }))
}

/// 申请API密钥（公开接口，无需认证）
async fn apply_handler(
    State(state): State<AppState>,
    Json(req): Json<ApplyRequest>,
) -> Result<Json<ApplyResponse>, AuthError> {
    // 验证用户名格式
    if req.username.len() < 3 || req.username.len() > 50 {
        return Err(AuthError::InvalidToken("用户名长度必须在3-50个字符之间".to_string()));
    }

    if !req.username.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
        return Err(AuthError::InvalidToken("用户名只能包含字母、数字、下划线和连字符".to_string()));
    }

    // 验证邮箱格式
    if !req.email.contains('@') || !req.email.contains('.') {
        return Err(AuthError::InvalidToken("邮箱格式不正确".to_string()));
    }

    // 检查用户名是否已存在
    let exists = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM users WHERE username = $1"
    )
    .bind(&req.username)
    .fetch_one(&state.db)
    .await
    .map_err(|e| AuthError::Database(e.to_string()))?;

    if exists > 0 {
        return Ok(Json(ApplyResponse {
            status: "error".to_string(),
            message: "用户名已被占用".to_string(),
            api_key: None,
            user_id: None,
        }));
    }

    // 生成用户ID和API密钥
    let user_id = format!("user_{}", Uuid::new_v4());

    // 生成 Anthropic 格式 API Key (sk-ant-api03-XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX)
    // Anthropic API Key 格式: sk-ant-api03-<52个大写字母或数字>
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    const KEY_LEN: usize = 52;
    let api_key_suffix: String = (0..KEY_LEN)
        .map(|_| {
            let idx = rand::thread_rng().gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect();
    let anthropic_api_key = format!("sk-ant-api03-{}", api_key_suffix);

    // 短 Key 作为内部标识（用于查询等）
    let short_key: String = Uuid::new_v4().to_string()[..9].chars().map(|c| {
        match c {
            '0'..'9' => c,
            'a'..='z' => (c as u8 - b'a' + b'A') as char,
            _ => 'X',
        }
    }).collect();
    let short_key_internal = format!("sk-{}", &short_key[..9]);

    // 长格式 Key 备用（内部使用）
    let long_api_key = format!("sk_{}", Uuid::new_v4());

    let default_password = "password123"; // 默认密码，用户首次登录后应修改
    let password_hash = format!("{:x}", Sha256::digest(default_password.as_bytes()));

    // 创建新用户
    sqlx::query(
        "INSERT INTO users (user_id, username, api_token, tier, balance, token_version, active, password_hash, created_at_old, updated_at_old, created_at, updated_at)
         VALUES ($1, $2, $3, 'base', 100000, 1, true, $4, EXTRACT(EPOCH FROM NOW())::BIGINT, EXTRACT(EPOCH FROM NOW())::BIGINT, NOW(), NOW())"
    )
    .bind(&user_id)
    .bind(&req.username)
    .bind(&long_api_key)
    .bind(&password_hash)
    .execute(&state.db)
    .await
    .map_err(|e| AuthError::Database(e.to_string()))?;

    // 插入 Anthropic 格式 API Key（用户使用的 Key）
    sqlx::query(
        "INSERT INTO user_api_keys (api_key, user_id, is_active)
         VALUES ($1, $2, true)"
    )
    .bind(&anthropic_api_key)
    .bind(&user_id)
    .execute(&state.db)
    .await
    .map_err(|e| AuthError::Database(e.to_string()))?;

    info!("新用户注册: username={}, user_id={}, email={}, api_key={}", req.username, user_id, req.email, anthropic_api_key);

    Ok(Json(ApplyResponse {
        status: "success".to_string(),
        message: "申请成功！您的API密钥已生成".to_string(),
        api_key: Some(anthropic_api_key),
        user_id: Some(user_id),
    }))
}

/// 检查用户名是否可用
async fn check_username_handler(
    State(state): State<AppState>,
    Path(username): Path<String>,
) -> Result<Json<CheckAvailabilityResponse>, AuthError> {
    let exists = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM users WHERE username = $1"
    )
    .bind(&username)
    .fetch_one(&state.db)
    .await
    .map_err(|e| AuthError::Database(e.to_string()))?;

    Ok(Json(CheckAvailabilityResponse {
        available: exists == 0,
    }))
}

/// 测试认证状态
async fn auth_test_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<AuthTestResponse>, AuthError> {
    let auth_header = headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| AuthError::InvalidToken("缺少Authorization头".to_string()))?;

    let token = auth_header.strip_prefix("Bearer ")
        .ok_or_else(|| AuthError::InvalidToken("无效的Authorization格式".to_string()))?;

    let user_info = state.auth.verify_token(token).await?;

    Ok(Json(AuthTestResponse {
        status: "authenticated".to_string(),
        user: AuthenticatedUserResponse {
            user_id: user_info.user_id.clone(),
            username: user_info.username.clone(),
            tier: user_info.tier.clone(),
        },
        token_remaining_secs: 0,  // 可以从Claims获取
    }))
}

async fn token_query_handler(
    Extension(user): Extension<AuthenticatedUser>,
    Json(req): Json<TokenQueryRequest>,
) -> Result<Json<TokenQueryResponse>, AuthError> {
    let cost = calculate_cost(&req.model, req.input_tokens, req.output_tokens);

    Ok(Json(TokenQueryResponse {
        status: "ok".to_string(),
        cost,
        balance: user.balance,
        can_proceed: user.balance >= cost,
    }))
}

/// 获取用户任务序号
async fn user_task_sequence_handler(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
) -> Json<UserTaskSequenceResponse> {
    let task_sequence = sqlx::query_scalar::<_, i64>(
        "SELECT COALESCE(task_sequence, 0) FROM user_task_sequences WHERE user_id = $1"
    )
    .bind(&user.user_id)
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);

    let pool_id = sqlx::query_scalar::<_, Option<String>>(
        "SELECT pool_id FROM user_task_sequences WHERE user_id = $1"
    )
    .bind(&user.user_id)
    .fetch_one(&state.db)
    .await
    .unwrap_or(None);

    let last_updated = sqlx::query_scalar::<_, Option<i64>>(
        "SELECT last_updated FROM user_task_sequences WHERE user_id = $1"
    )
    .bind(&user.user_id)
    .fetch_one(&state.db)
    .await
    .unwrap_or(None);

    Json(UserTaskSequenceResponse {
        user_id: user.user_id.clone(),
        task_sequence,
        pool_id: pool_id.unwrap_or_else(|| "none".to_string()),
        last_updated: last_updated.unwrap_or(0),
    })
}

/// 列出所有资源池及其状态
async fn providers_handler(State(state): State<AppState>) -> Result<Json<PoolsListResponse>, AuthError> {
    let pools = state.pool_manager.list_pools().await;
    let system_status = state.pool_manager.get_system_status().await;

    let mut pool_statuses = vec![];

    for pool in &pools {
        // 获取该池的路由规则数量
        let rules_count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM pool_routing_rules WHERE pool_id = $1 AND enabled = true"
        )
        .bind(&pool.pool_id)
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);

        pool_statuses.push(PoolStatusResponse {
            pool_id: pool.pool_id.clone(),
            pool_name: pool.pool_name.clone(),
            description: pool.description.clone(),
            enabled: pool.enabled,
            routing_rules_count: rules_count,
            fallback_chain: pool.fallback_chain.clone(),
        });
    }

    Ok(Json(PoolsListResponse {
        pools: pool_statuses,
        total_pools: pools.len(),
        enabled_pools: system_status.enabled_pools,
        total_providers: system_status.total_providers,
        healthy_providers: system_status.healthy_providers,
    }))
}

async fn chat_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Extension(user): Extension<AuthenticatedUser>,
    Json(req): Json<ChatRequest>,
) -> Result<Json<ChatResponse>, AuthError> {
    let start_time = std::time::Instant::now();
    let request_id = Uuid::new_v4();

    // 限流检查
    let qps_limit = match user.tier.as_str() {
        "base" => 10,
        "pro" => 100,
        "max" => 200,
        _ => 10,
    };

    let mut conn = state.redis.get_async_connection().await
        .map_err(|e| AuthError::Redis(e.to_string()))?;

    let rate_key = format!("rate_limit:{}", user.user_id);
    let count: Option<usize> = redis::cmd("GET")
        .arg(&rate_key)
        .query_async(&mut conn)
        .await
        .map_err(|e| AuthError::Redis(e.to_string()))?;

    if let Some(c) = count {
        if c >= qps_limit as usize {
            return Err(AuthError::Forbidden);
        }
    }

    redis::cmd("SETEX")
        .arg(&rate_key)
        .arg(1)
        .arg(count.unwrap_or(0) + 1)
        .query_async::<_, ()>(&mut conn)
        .await
        .map_err(|e| AuthError::Redis(e.to_string()))?;

    // 验证模型 - 检查是否为有效的资源池别名
    let valid_pools = ["opus", "sonnet", "haiku", "codex"];
    if !valid_pools.contains(&req.model.as_str()) {
        return Err(AuthError::InvalidToken(format!("模型不存在: {}", req.model)));
    }

    // 计算tokens
    let estimated_input_tokens: u32 = req.messages.iter()
        .map(|m| m.content.len() as u32 / 4)
        .sum();
    let estimated_cost = calculate_cost(&req.model, estimated_input_tokens, req.max_tokens);

    if user.balance < estimated_cost {
        return Err(AuthError::Forbidden);
    }

    // 使用资源池管理器调用API（支持基于任务序号的路由和故障转移）
    let api_result = state.pool_manager.call_api(&user.user_id, &req, &req.model).await;

    let (response_text, actual_input_tokens, actual_output_tokens, pool_id, task_sequence, actual_provider, used_fallback) = match api_result {
        Ok(result) => {
            info!("API调用成功 - 用户: {}, 池: {}, 任务序号: {}, 提供商: {}, 模型: {}, 耗时: {}ms, 故障转移: {}",
                  user.user_id, result.pool_id, result.task_sequence, result.provider_id,
                  result.actual_model, result.response_time_ms, result.used_fallback);
            (result.content, result.input_tokens, result.output_tokens,
             result.pool_id, result.task_sequence, result.provider_id, result.used_fallback)
        }
        Err(e) => {
            error!("所有提供商调用失败: {}", e);
            return Err(AuthError::Internal(format!("API调用失败: {}", e)));
        }
    };

    // 使用实际的token计数或估算
    let input_tokens = if actual_input_tokens > 0 { actual_input_tokens } else { estimated_input_tokens };
    let output_tokens = if actual_output_tokens > 0 { actual_output_tokens } else { (response_text.len() as u32 / 4) + 1 };
    let actual_cost = calculate_cost(&req.model, input_tokens, output_tokens);

    // 扣费
    sqlx::query(
        "UPDATE users SET balance = balance - $1, updated_at_old = EXTRACT(EPOCH FROM NOW())::BIGINT, updated_at = NOW()
         WHERE user_id = $2"
    )
    .bind(actual_cost)
    .bind(&user.user_id)
    .execute(&state.db)
    .await
    .map_err(|e| AuthError::Database(e.to_string()))?;

    // 检查是否需要刷新token
    let auth_header = headers.get("Authorization")
        .and_then(|h| h.to_str().ok());
    let new_token = if let Some(hdr) = auth_header {
        let token = hdr.strip_prefix("Bearer ").unwrap_or("");
        state.auth.refresh_token(token).ok().flatten()
    } else {
        None
    };

    let elapsed = start_time.elapsed().as_millis();
    info!(request_id = %request_id, user_id = %user.user_id, model = %req.model,
          pool_id = %pool_id, task_seq = %task_sequence, provider = %actual_provider, fallback = %used_fallback,
          cost = actual_cost, elapsed_ms = elapsed, "Chat completion");

    Ok(Json(ChatResponse {
        id: request_id.to_string(),
        object: "chat.completion".to_string(),
        created: chrono::Utc::now().timestamp(),
        model: req.model,
        choices: vec![ChatChoice {
            index: 0,
            message: ChatMessage {
                role: "assistant".to_string(),
                content: response_text,
            },
            finish_reason: "stop".to_string(),
        }],
        usage: Usage {
            prompt_tokens: input_tokens,
            completion_tokens: output_tokens,
            total_tokens: input_tokens + output_tokens,
        },
        new_token,
    }))
}

/// ========== Anthropic Messages API Handler (Claude Code 专用) ==========
async fn anthropic_messages_handler(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Json(req): Json<AnthropicMessagesRequest>,
) -> Result<Json<AnthropicMessagesResponse>, AnthropicErrorResponse> {
    let start_time = std::time::Instant::now();
    let request_id = Uuid::new_v4();

    // 限流检查
    let qps_limit = match user.tier.as_str() {
        "base" => 10,
        "pro" => 100,
        "max" => 200,
        _ => 10,
    };

    let mut conn = state.redis.get_async_connection().await
        .map_err(|e| AnthropicErrorResponse::from(AuthError::Redis(e.to_string())))?;

    let rate_key = format!("rate_limit:{}", user.user_id);
    let count: Option<usize> = redis::cmd("GET")
        .arg(&rate_key)
        .query_async(&mut conn)
        .await
        .map_err(|e| AnthropicErrorResponse::from(AuthError::Redis(e.to_string())))?;

    if let Some(c) = count {
        if c >= qps_limit as usize {
            return Err(AnthropicErrorResponse::from(AuthError::Forbidden));
        }
    }

    redis::cmd("SETEX")
        .arg(&rate_key)
        .arg(1)
        .arg(count.unwrap_or(0) + 1)
        .query_async::<_, ()>(&mut conn)
        .await
        .map_err(|e| AnthropicErrorResponse::from(AuthError::Redis(e.to_string())))?;

    // 映射 Anthropic 模型名到内部池ID
    let pool_id = map_anthropic_model(&req.model);

    // 转换 Anthropic 消息格式到内部格式
    let chat_req = ChatRequest {
        model: pool_id.clone(),
        messages: req.messages.iter().map(anthropic_to_chat).collect(),
        stream: req.stream,
        max_tokens: req.max_tokens,
    };

    // 计算tokens
    let estimated_input_tokens: u32 = chat_req.messages.iter()
        .map(|m| m.content.len() as u32 / 4)
        .sum();
    let estimated_cost = calculate_cost(&pool_id, estimated_input_tokens, chat_req.max_tokens);

    if user.balance < estimated_cost {
        return Err(AnthropicErrorResponse::from(AuthError::Forbidden));
    }

    // 使用资源池管理器调用API
    let api_result = state.pool_manager.call_api(&user.user_id, &chat_req, &pool_id).await;

    let (response_text, actual_input_tokens, actual_output_tokens, _, _, actual_provider, _) = match api_result {
        Ok(result) => {
            info!("Anthropic API调用成功 - 用户: {}, 池: {}, 提供商: {}, 耗时: {}ms",
                  user.user_id, result.pool_id, result.provider_id, result.response_time_ms);
            (result.content, result.input_tokens, result.output_tokens,
             result.pool_id, result.task_sequence, result.provider_id, result.used_fallback)
        }
        Err(e) => {
            error!("Anthropic API调用失败: {}", e);
            return Err(AnthropicErrorResponse {
                error_type: "error".to_string(),
                error: AnthropicErrorDetail {
                    detail_type: "api_error".to_string(),
                    message: format!("API调用失败: {}", e),
                },
            });
        }
    };

    // 使用实际的token计数或估算
    let input_tokens = if actual_input_tokens > 0 { actual_input_tokens } else { estimated_input_tokens };
    let output_tokens = if actual_output_tokens > 0 { actual_output_tokens } else { (response_text.len() as u32 / 4) + 1 };
    let actual_cost = calculate_cost(&pool_id, input_tokens, output_tokens);

    // 扣费
    sqlx::query(
        "UPDATE users SET balance = balance - $1, updated_at_old = EXTRACT(EPOCH FROM NOW())::BIGINT, updated_at = NOW()
         WHERE user_id = $2"
    )
    .bind(actual_cost)
    .bind(&user.user_id)
    .execute(&state.db)
    .await
    .map_err(|e| AnthropicErrorResponse::from(AuthError::Database(e.to_string())))?;

    let elapsed = start_time.elapsed().as_millis();
    info!(request_id = %request_id, user_id = %user.user_id, model = %req.model,
          pool_id = %pool_id, provider = %actual_provider, cost = actual_cost, elapsed_ms = elapsed, "Anthropic messages");

    // 返回 Anthropic 格式响应
    Ok(Json(AnthropicMessagesResponse {
        id: request_id.to_string(),
        response_type: "message".to_string(),
        role: "assistant".to_string(),
        content: vec![
            AnthropicContentBlock {
                content_type: "text".to_string(),
                text: response_text,
            }
        ],
        model: req.model,
        stop_reason: "end_turn".to_string(),
        usage: AnthropicUsage {
            input_tokens,
            output_tokens,
        },
    }))
}

/// 撤销当前token
async fn logout_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, AuthError> {
    let auth_header = headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| AuthError::InvalidToken("缺少Authorization头".to_string()))?;

    let token = auth_header.strip_prefix("Bearer ")
        .ok_or_else(|| AuthError::InvalidToken("无效的Authorization格式".to_string()))?;

    // 解析token获取jti
    let claims = state.auth.parse_claims(token)
        .map_err(|e| AuthError::InvalidToken(e.to_string()))?;

    // 计算token哈希用于清除L1缓存
    let token_hash = hash_token_for_cache(token);

    // 加入黑名单并清除L1缓存
    state.auth.revoke_token_with_cache(&claims.jti, &token_hash).await
        .map_err(|e| AuthError::Redis(e.to_string()))?;

    Ok(Json(serde_json::json!({
        "status": "ok",
        "message": "Token已撤销"
    })))
}

/// 撤销用户所有token (管理员功能)
async fn revoke_user_handler(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    axum::extract::Path(user_id): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, AuthError> {
    // 只有用户自己或管理员可以撤销
    if user.user_id != user_id && user.tier != "admin" {
        return Err(AuthError::Forbidden);
    }

    // 获取当前token版本
    let token_version = sqlx::query_scalar::<_, i32>(
        "SELECT token_version FROM users WHERE user_id = $1"
    )
    .bind(&user_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| AuthError::Database(e.to_string()))?;

    // 撤销所有token
    state.auth.revoke_user_all(&user_id, token_version).await
        .map_err(|e| AuthError::Redis(e.to_string()))?;

    Ok(Json(serde_json::json!({
        "status": "ok",
        "message": format!("用户 {} 所有token已撤销", user_id)
    })))
}

// ========== 管理后台 API ==========

/// 管理员统计数据响应
#[derive(Debug, Serialize)]
struct AdminStatsResponse {
    total_users: i64,
    active_users: i64,
    total_balance: i64,
    today_requests: i64,
    today_cost: i64,
    total_requests: i64,
    api_keys_count: i64,
    recharge_cards_total: i64,
    recharge_cards_used: i64,
}

/// 管理员统计数据
async fn admin_stats_handler(
    State(state): State<AppState>,
) -> Result<Json<AdminStatsResponse>, AuthError> {
    // 总用户数
    let total_users = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM users")
        .fetch_one(&state.db)
        .await
        .map_err(|e| AuthError::Database(e.to_string()))?;

    // 活跃用户数
    let active_users = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM users WHERE active = true")
        .fetch_one(&state.db)
        .await
        .map_err(|e| AuthError::Database(e.to_string()))?;

    // 总余额
    let total_balance = sqlx::query_scalar::<_, i64>("SELECT COALESCE(CAST(SUM(balance) AS BIGINT), 0) FROM users")
        .fetch_one(&state.db)
        .await
        .map_err(|e| AuthError::Database(e.to_string()))?;

    // 今日请求数 (从今天00:00开始)
    let today_start = chrono::Utc::now()
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_utc()
        .timestamp();

    let today_requests = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM request_logs WHERE created_at >= $1"
    )
    .bind(today_start)
    .fetch_one(&state.db)
    .await
    .map_err(|e| AuthError::Database(e.to_string()))?;

    // 今日消费
    let today_cost = sqlx::query_scalar::<_, i64>(
        "SELECT COALESCE(CAST(SUM(cost) AS BIGINT), 0) FROM request_logs WHERE created_at >= $1"
    )
    .bind(today_start)
    .fetch_one(&state.db)
    .await
    .map_err(|e| AuthError::Database(e.to_string()))?;

    // 总请求数
    let total_requests = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM request_logs")
        .fetch_one(&state.db)
        .await
        .map_err(|e| AuthError::Database(e.to_string()))?;

    // API Keys 数量
    let api_keys_count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM user_api_keys WHERE is_active = true")
        .fetch_one(&state.db)
        .await
        .map_err(|e| AuthError::Database(e.to_string()))?;

    // 充值卡统计
    let recharge_cards_total = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM recharge_cards")
        .fetch_one(&state.db)
        .await
        .map_err(|e| AuthError::Database(e.to_string()))?;

    let recharge_cards_used = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM recharge_cards WHERE used = true")
        .fetch_one(&state.db)
        .await
        .map_err(|e| AuthError::Database(e.to_string()))?;

    Ok(Json(AdminStatsResponse {
        total_users,
        active_users,
        total_balance,
        today_requests,
        today_cost,
        total_requests,
        api_keys_count,
        recharge_cards_total,
        recharge_cards_used,
    }))
}

/// 用户列表响应
#[derive(Debug, Serialize)]
struct AdminUsersResponse {
    users: Vec<UserInfo>,
    total: i64,
    page: i64,
    page_size: i64,
}

#[derive(Debug, Serialize)]
struct UserInfo {
    user_id: String,
    username: String,
    email: Option<String>,
    tier: String,
    balance: i64,
    created_at: i64,
    updated_at: i64,
}

/// 获取用户列表
async fn admin_users_handler(
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<AdminUsersQuery>,
) -> Result<Json<AdminUsersResponse>, AuthError> {
    let page = params.page.unwrap_or(1).max(1);
    let page_size = params.page_size.unwrap_or(20).min(100);
    let offset = (page - 1) * page_size;

    // 获取总数
    let total = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM users")
        .fetch_one(&state.db)
        .await
        .map_err(|e| AuthError::Database(e.to_string()))?;

    // 获取用户列表
    let users = sqlx::query_as::<_, (String, String, Option<String>, String, i64, i64, i64)>(
        "SELECT user_id, username, email, tier, balance, created_at_old, updated_at_old
         FROM users
         ORDER BY created_at_old DESC
         LIMIT $1 OFFSET $2"
    )
    .bind(page_size)
    .bind(offset)
    .fetch_all(&state.db)
    .await
    .map_err(|e| AuthError::Database(e.to_string()))?
    .into_iter()
    .map(|r| UserInfo {
        user_id: r.0,
        username: r.1,
        email: r.2,
        tier: r.3,
        balance: r.4,
        created_at: r.5,
        updated_at: r.6,
    })
    .collect();

    Ok(Json(AdminUsersResponse {
        users,
        total,
        page,
        page_size,
    }))
}

#[derive(Debug, Deserialize)]
struct AdminUsersQuery {
    #[serde(default)]
    page: Option<i64>,
    #[serde(default)]
    page_size: Option<i64>,
    #[serde(default)]
    search: Option<String>,
}

/// 充值卡列表响应
#[derive(Debug, Serialize)]
struct AdminRechargeCardsResponse {
    cards: Vec<RechargeCardInfo>,
    total: i64,
    page: i64,
    page_size: i64,
}

#[derive(Debug, Serialize)]
struct RechargeCardInfo {
    card_id: i32,
    card_code: String,
    amount: i64,
    is_used: bool,
    used_by: Option<String>,
    created_at: String,
    used_at: Option<String>,
}

/// 获取充值卡列表
async fn admin_recharge_cards_list_handler(
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<AdminCardsQuery>,
) -> Result<Json<AdminRechargeCardsResponse>, AuthError> {
    let page = params.page.unwrap_or(1).max(1);
    let page_size = params.page_size.unwrap_or(20).min(100);
    let offset = (page - 1) * page_size;

    // 获取总数
    let total = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM recharge_cards")
        .fetch_one(&state.db)
        .await
        .map_err(|e| AuthError::Database(e.to_string()))?;

    // 获取充值卡列表
    let cards = sqlx::query_as::<_, (i32, String, i64, bool, Option<String>, String, Option<String>)>(
        "SELECT card_id, card_code, amount, used, used_by,
         TO_CHAR(created_at, 'YYYY-MM-DD HH24:MI:SS'),
         TO_CHAR(used_at, 'YYYY-MM-DD HH24:MI:SS')
         FROM recharge_cards
         ORDER BY created_at DESC
         LIMIT $1 OFFSET $2"
    )
    .bind(page_size)
    .bind(offset)
    .fetch_all(&state.db)
    .await
    .map_err(|e| AuthError::Database(e.to_string()))?
    .into_iter()
    .map(|r| RechargeCardInfo {
        card_id: r.0,
        card_code: r.1,
        amount: r.2,
        is_used: r.3,
        used_by: r.4,
        created_at: r.5,
        used_at: r.6,
    })
    .collect();

    Ok(Json(AdminRechargeCardsResponse {
        cards,
        total,
        page,
        page_size,
    }))
}

#[derive(Debug, Deserialize)]
struct AdminCardsQuery {
    #[serde(default)]
    page: Option<i64>,
    #[serde(default)]
    page_size: Option<i64>,
}

/// 创建充值卡请求
#[derive(Debug, Deserialize)]
struct CreateRechargeCardsRequest {
    amount: i64,
    count: i64,
    #[serde(default)]
    prefix: Option<String>,
}

/// 创建充值卡响应
#[derive(Debug, Serialize)]
struct CreateRechargeCardsResponse {
    success: bool,
    message: String,
    cards: Vec<String>,
}

/// 创建充值卡
async fn admin_recharge_cards_create_handler(
    State(state): State<AppState>,
    Json(req): Json<CreateRechargeCardsRequest>,
) -> Result<Json<CreateRechargeCardsResponse>, AuthError> {
    if req.amount <= 0 {
        return Ok(Json(CreateRechargeCardsResponse {
            success: false,
            message: "充值金额必须大于0".to_string(),
            cards: vec![],
        }));
    }

    if req.count <= 0 || req.count > 1000 {
        return Ok(Json(CreateRechargeCardsResponse {
            success: false,
            message: "一次最多生成1000张卡".to_string(),
            cards: vec![],
        }));
    }

    let prefix = req.prefix.unwrap_or_default();
    let mut created_cards = vec![];

    for _ in 0..req.count {
        let card_code = if prefix.is_empty() {
            format!("{:010}", rand::thread_rng().gen_range(1000000000i64..=9999999999i64))
        } else {
            format!("{}{:06}", prefix, rand::thread_rng().gen_range(100000..=999999))
        };

        // 插入充值卡
        let result = sqlx::query(
            "INSERT INTO recharge_cards (card_code, amount, used) VALUES ($1, $2, false)"
        )
        .bind(&card_code)
        .bind(req.amount)
        .execute(&state.db)
        .await;

        if result.is_ok() {
            created_cards.push(card_code);
        }
    }

    Ok(Json(CreateRechargeCardsResponse {
        success: true,
        message: format!("成功创建{}张充值卡", created_cards.len()),
        cards: created_cards,
    }))
}

/// 删除充值卡响应
#[derive(Debug, Serialize)]
struct DeleteRechargeCardResponse {
    success: bool,
    message: String,
}

/// 删除充值卡
async fn admin_recharge_cards_delete_handler(
    State(state): State<AppState>,
    axum::extract::Path(card_id): axum::extract::Path<i32>,
) -> Result<Json<DeleteRechargeCardResponse>, AuthError> {
    // 检查卡是否存在
    let exists = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM recharge_cards WHERE card_id = $1"
    )
    .bind(card_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| AuthError::Database(e.to_string()))?;

    if exists == 0 {
        return Ok(Json(DeleteRechargeCardResponse {
            success: false,
            message: "充值卡不存在".to_string(),
        }));
    }

    // 删除充值卡
    sqlx::query("DELETE FROM recharge_cards WHERE card_id = $1")
        .bind(card_id)
        .execute(&state.db)
        .await
        .map_err(|e| AuthError::Database(e.to_string()))?;

    Ok(Json(DeleteRechargeCardResponse {
        success: true,
        message: "充值卡已删除".to_string(),
    }))
}

/// 用户充值接口 (兼容前端)
async fn user_recharge_handler(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Json(req): Json<RedeemRequest>,
) -> Result<Json<RedeemResponse>, AuthError> {
    // 检查充值卡是否存在
    let card = sqlx::query_as::<_, (i32, i64, bool)>(
        "SELECT card_id, amount, used FROM recharge_cards WHERE card_code = $1"
    )
    .bind(&req.code.trim())
    .fetch_optional(&state.db)
    .await
    .map_err(|e| AuthError::Database(e.to_string()))?;

    let card = match card {
        Some(c) => c,
        None => {
            return Ok(Json(RedeemResponse {
                success: false,
                message: "充值卡不存在".to_string(),
                amount: None,
            }));
        }
    };

    if card.2 {
        return Ok(Json(RedeemResponse {
            success: false,
            message: "充值卡已被使用".to_string(),
            amount: None,
        }));
    }

    // 开启事务
    let mut tx = state.db.begin().await
        .map_err(|e| AuthError::Database(e.to_string()))?;

    // 标记充值卡为已使用
    sqlx::query("UPDATE recharge_cards SET used = true, used_by = $1, used_at = NOW() WHERE card_code = $2")
        .bind(&user.user_id)
        .bind(&req.code.trim())
        .execute(&mut *tx)
        .await
        .map_err(|e| AuthError::Database(e.to_string()))?;

    // 增加用户余额
    sqlx::query("UPDATE users SET balance = balance + $1, updated_at_old = EXTRACT(EPOCH FROM NOW())::BIGINT, updated_at = NOW() WHERE user_id = $2")
        .bind(card.1)
        .bind(&user.user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| AuthError::Database(e.to_string()))?;

    tx.commit().await.map_err(|e| AuthError::Database(e.to_string()))?;

    info!("用户 {} 使用充值卡充值 {} 积分", user.username, card.1);

    Ok(Json(RedeemResponse {
        success: true,
        message: "充值成功".to_string(),
        amount: Some(card.1),
    }))
}

// ========== 工具函数 ==========

fn calculate_cost(model: &str, input_tokens: u32, output_tokens: u32) -> i64 {
    // 价格单位: 每100万token的价格(分)
    // 为了避免整数除法导致小额成本被舍去，使用浮点数计算后四舍五入
    let (input_price, output_price) = match model {
        "opus" => (15.0, 75.0),
        "sonnet" => (3.0, 15.0),
        "haiku" => (0.0, 1.0),
        "codex" => (5.0, 20.0),
        _ => (1.0, 5.0),
    };

    // 使用浮点数计算，避免整数除法舍去小数
    let input_cost = ((input_tokens as f64 * input_price) / 1_000_000.0).round() as i64;
    let output_cost = ((output_tokens as f64 * output_price) / 1_000_000.0).round() as i64;

    let total_cost = input_cost + output_cost;

    // 只有当完全没有任何使用时才不收费
    // 对于小额计算结果为0的情况，仍然返回0（不计费）
    // 实际使用中，token数很少会这么少
    total_cost.max(0)
}

/// 计算token哈希 (与jwt.rs中hash_token逻辑一致)
fn hash_token_for_cache(token: &str) -> String {
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    format!("{:x}", hasher.finalize())[..32].to_string()
}

// ========== 数据库初始化 ==========

async fn init_database(pool: &PgPool) -> Result<(), sqlx::Error> {
    // 创建用户表
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS users (
            user_id VARCHAR(64) PRIMARY KEY,
            username VARCHAR(64) UNIQUE NOT NULL,
            api_token VARCHAR(128) UNIQUE,
            tier VARCHAR(32) NOT NULL DEFAULT 'base',
            balance BIGINT NOT NULL DEFAULT 0,
            token_version INTEGER NOT NULL DEFAULT 1,
            active BOOLEAN NOT NULL DEFAULT true,
            created_at BIGINT NOT NULL,
            updated_at BIGINT NOT NULL
        )
        "#
    )
    .execute(pool)
    .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_users_username ON users(username)")
        .execute(pool)
        .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_users_api_token ON users(api_token)")
        .execute(pool)
        .await?;

    // 创建请求日志表
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS request_logs (
            log_id BIGSERIAL PRIMARY KEY,
            user_id VARCHAR(64) NOT NULL,
            model VARCHAR(64) NOT NULL,
            prompt_tokens INTEGER NOT NULL,
            completion_tokens INTEGER NOT NULL,
            total_tokens INTEGER NOT NULL,
            cost BIGINT NOT NULL,
            status VARCHAR(32) NOT NULL,
            created_at BIGINT NOT NULL,
            error_message TEXT
        )
        "#
    )
    .execute(pool)
    .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_request_logs_user_id ON request_logs(user_id)")
        .execute(pool)
        .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_request_logs_created_at ON request_logs(created_at)")
        .execute(pool)
        .await?;

    // 创建或更新测试用户
    // 用户1: 测试用户 (pro等级)
    sqlx::query(
        r#"
        INSERT INTO users (user_id, username, api_token, tier, balance, token_version, active, created_at_old, updated_at_old, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, 1, true, EXTRACT(EPOCH FROM NOW())::BIGINT, EXTRACT(EPOCH FROM NOW())::BIGINT, NOW(), NOW())
        ON CONFLICT (username) DO UPDATE
        SET balance = EXCLUDED.balance,
            tier = EXCLUDED.tier,
            updated_at_old = EXTRACT(EPOCH FROM NOW())::BIGINT,
            updated_at = NOW()
        "#
    )
    .bind("user_test_001")
    .bind("test_user")
    .bind("sk_test_1234567890abcdef")
    .bind("pro")
    .bind(100_000_000i64)
    .execute(pool)
    .await?;

    // 用户2: base等级用户
    sqlx::query(
        r#"
        INSERT INTO users (user_id, username, tier, balance, token_version, active, created_at_old, updated_at_old, created_at, updated_at)
        VALUES ($1, $2, $3, $4, 1, true, EXTRACT(EPOCH FROM NOW())::BIGINT, EXTRACT(EPOCH FROM NOW())::BIGINT, NOW(), NOW())
        ON CONFLICT (username) DO UPDATE
        SET balance = EXCLUDED.balance,
            tier = EXCLUDED.tier,
            updated_at_old = EXTRACT(EPOCH FROM NOW())::BIGINT,
            updated_at = NOW()
        "#
    )
    .bind("user_test_002")
    .bind("base_user")
    .bind("base")
    .bind(10_000i64)
    .execute(pool)
    .await?;

    info!("数据库初始化完成");
    Ok(())
}

// ========== 密钥轮换任务 ==========

async fn key_rotation_task(auth: Arc<JwtAuthService>) {
    let mut interval = interval(Duration::from_secs(3600)); // 每小时检查一次

    loop {
        interval.tick().await;

        if auth.needs_rotation() {
            info!("开始执行密钥轮换...");
            if auth.rotate_keys() {
                info!("密钥轮换完成");
            }
        }
    }
}

// ========== Redis内存限制配置 ==========

/// 配置Redis内存限制为128MB
async fn configure_redis_memory(redis_url: &str) -> anyhow::Result<()> {
    let client = redis::Client::open(redis_url)?;
    let mut conn = client.get_async_connection().await?;

    // 设置maxmemory为128MB
    redis::cmd("CONFIG")
        .arg("SET")
        .arg("maxmemory")
        .arg("134217728")  // 128MB in bytes
        .query_async::<_, ()>(&mut conn)
        .await?;

    // 设置淘汰策略为allkeys-lru
    redis::cmd("CONFIG")
        .arg("SET")
        .arg("maxmemory-policy")
        .arg("allkeys-lru")
        .query_async::<_, ()>(&mut conn)
        .await?;

    info!("Redis内存限制已配置: 128MB, 淘汰策略: allkeys-lru");

    Ok(())
}

// ========== Main ==========

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 加载.env文件
    dotenv::dotenv().ok();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "api_gateway=info,tower_http=info,axum=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = Config::from_env();

    info!("========================================");
    info!("API网关启动中 - JWT认证版");
    info!("========================================");
    info!("版本: 3.0.0 (JWT + Redis + PostgreSQL)");
    info!("Token TTL: {}秒 (15分钟)", TOKEN_TTL_SECS);
    info!("密钥轮换窗口: {}小时", KEY_ROTATION_HOURS);
    info!("数据库: {}", config.database_url);
    info!("Redis: {}", config.redis_url);

    // 连接PostgreSQL
    let db_pool = PgPoolOptions::new()
        .max_connections(20)
        .connect(&config.database_url)
        .await
        .map_err(|e| anyhow::anyhow!("数据库连接失败: {}", e))?;

    info!("PostgreSQL 连接成功");

    // 初始化数据库
    init_database(&db_pool).await?;

    // 连接Redis
    let redis_client = RedisClient::open(config.redis_url.as_str())
        .map_err(|e| anyhow::anyhow!("Redis连接失败: {}", e))?;

    // 验证Redis连接
    let _ = redis_client.get_async_connection().await
        .map_err(|e| anyhow::anyhow!("Redis连接验证失败: {}", e))?;

    info!("Redis 连接成功");

    // 配置Redis内存限制为128MB
    if let Err(e) = configure_redis_memory(&config.redis_url).await {
        warn!("Redis内存配置失败: {}", e);
    }

    // 创建JWT认证服务
    let auth = Arc::new(
        JwtAuthService::new(db_pool.clone(), &config.redis_url).await?
    );

    info!("JWT认证服务初始化完成");
    info!("密钥版本: {}", auth.key_version());

    // 创建HTTP客户端
    let http_client = Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .unwrap();

    // 初始化多提供商管理器
    let pool_manager = Arc::new(
        provider::ModelPoolManager::new(&config.database_url, &config.redis_url)
            .await
            .map_err(|e| anyhow::anyhow!("提供商管理器初始化失败: {}", e))?
    );

    info!("模型资源池管理器初始化完成");
    let pools_count: usize = pool_manager.list_pools().await.len();
    info!("已加载资源池: {}", pools_count);

    let state = AppState {
        db: db_pool.clone(),
        redis: redis_client,
        auth: auth.clone(),
        config: config.clone(),
        http_client,
        pool_manager,
    };

    // 启动密钥轮换任务
    let auth_clone = auth.clone();
    tokio::spawn(async move {
        key_rotation_task(auth_clone).await;
    });

    let app = Router::new()
        // 公开端点
        .route("/health", get(health_handler))
        .route("/v1/models", get(models_handler))
        .route("/v1/providers", get(providers_handler))
        .route("/v1/user/task-sequence", get(user_task_sequence_handler))
        // 申请API（无需认证）
        .route("/api/apply", post(apply_handler))
        .route("/api/apply/check-availability/username/:username", get(check_username_handler))

        // 用户认证 API（兼容 client.html）
        .route("/api/user/login", post(user_login_handler))
        .route("/api/user/register", post(user_register_handler))
        .route("/api/user/balance", get(user_balance_handler))
        .route("/api/user/profile", post(user_profile_handler))
        .route("/api/user/change-password", post(user_change_password_handler))
        .route("/api/user/history", get(user_history_handler))

        // 公开 API
        .route("/api/health", get(public_health_handler))
        .route("/api/stats", get(public_stats_handler))

        // 充值 API
        .route("/api/recharge/redeem", post(recharge_redeem_handler))
        .route("/api/user/recharge", post(user_recharge_handler))

        // 管理后台 API (公开，前端自行验证管理员身份)
        .route("/api/admin/stats", get(admin_stats_handler))
        .route("/api/admin/users", get(admin_users_handler))
        .route("/api/admin/recharge-cards", get(admin_recharge_cards_list_handler))
        .route("/api/admin/recharge-cards/create", post(admin_recharge_cards_create_handler))
        .route("/api/admin/recharge-cards/:card_id", axum::routing::delete(admin_recharge_cards_delete_handler))

        // 认证端点
        .route("/v1/auth/login", post(login_handler))
        .route("/v1/auth/logout", post(logout_handler))
        .route("/v1/auth/test", get(auth_test_handler))
        .route("/v1/auth/revoke/:user_id", post(revoke_user_handler))

        // API端点 (需要认证)
        .route("/v1/token/query", post(token_query_handler))
        .route("/v1/chat/completions", post(chat_handler))

        .layer(CorsLayer::new().allow_origin(tower_http::cors::Any))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            jwt_auth_middleware,
        ))
        .with_state(state.clone())

        // Claude Code Anthropic API 端点 (使用单独的认证中间件)
        .merge(
            Router::new()
                .route("/v1/messages", post(anthropic_messages_handler))
                .layer(CorsLayer::new().allow_origin(tower_http::cors::Any))
                .layer(middleware::from_fn_with_state(
                    state.clone(),
                    anthropic_auth_middleware,
                ))
                .with_state(state)
        );

    let listener = tokio::net::TcpListener::bind(config.server_addr).await?;
    info!("========================================");
    info!("API网关已启动: http://{}", config.server_addr);
    info!("健康检查: http://{}/health", config.server_addr);
    info!("测试用户: test_user / password123");
    info!("========================================");

    axum::serve(listener, app).await?;

    Ok(())
}
