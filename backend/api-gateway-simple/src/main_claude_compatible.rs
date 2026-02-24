// ========== API网关 - Claude Code 完全兼容版 ==========
// 功能: JWT验证、密钥轮换、Token撤销、本地缓存
// 多提供商: GLM智谱AI、千问、DeepSeek自动故障转移
// Claude Code: 完全兼容 Anthropic Messages API

mod jwt;
mod provider;
mod anthropic;  // 新增 Anthropic API 专用模块

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

// 导入 anthropic 模块
use anthropic::{
    anthropic_messages_handler,
    AnthropicMessagesRequest,
    AnthropicErrorResponse,
};

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
    new_token: Option<String>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AnthropicMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct AnthropicMessagesRequestLegacy {
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
struct AnthropicMessagesResponseLegacy {
    id: String,
    #[serde(rename = "type")]
    response_type: String,
    role: String,
    content: Vec<AnthropicContentBlock>,
    model: String,
    stop_reason: String,
    usage: AnthropicUsage,
}

// ========== 辅助函数 ==========

pub fn calculate_cost(model: &str, input_tokens: u32, output_tokens: u32) -> i64 {
    let (input_price, output_price) = match model {
        "opus" => (15.0, 75.0),
        "sonnet" => (3.0, 15.0),
        "haiku" => (0.0, 1.0),
        "codex" => (5.0, 20.0),
        _ => (1.0, 5.0),
    };

    let input_cost = ((input_tokens as f64 * input_price) / 1_000_000.0).round() as i64;
    let output_cost = ((output_tokens as f64 * output_price) / 1_000_000.0).round() as i64;
    input_cost + output_cost
}

// 将 Anthropic 消息转换为内部 ChatMessage
fn anthropic_to_chat(msg: &AnthropicMessage) -> ChatMessage {
    ChatMessage {
        role: msg.role.clone(),
        content: msg.content.clone(),
    }
}

// 将 Anthropic 模型名映射到内部池ID
pub fn map_anthropic_model(model: &str) -> String {
    // 移除可能的后缀，如 [1m], [200k] 等（Claude Code 有时会添加这些）
    let base_model = if let Some(bracket_pos) = model.find('[') {
        &model[..bracket_pos]
    } else {
        model
    };

    match base_model {
        // ========== Claude 4 系列（最新） ==========
        "claude-opus-4-6" | "claude-opus-4-6-20250214" => "opus".to_string(),
        "claude-opus-4-5-20250929" => "opus".to_string(),
        "claude-opus-4-5-20250514" => "opus".to_string(),
        "claude-opus-4-20250514" => "opus".to_string(),
        "claude-opus-4" => "opus".to_string(),

        "claude-sonnet-4-5-20250929" => "sonnet".to_string(),
        "claude-sonnet-4-5-20250914" => "sonnet".to_string(),
        "claude-sonnet-4-5-20250514" => "sonnet".to_string(),
        "claude-sonnet-4-20250514" => "sonnet".to_string(),
        "claude-sonnet-4" => "sonnet".to_string(),

        "claude-haiku-4-5-20251001" => "haiku".to_string(),
        "claude-haiku-4-20250514" => "haiku".to_string(),
        "claude-haiku-4" => "haiku".to_string(),

        // ========== Claude 3 系列 ==========
        "claude-3-opus-20240229" => "opus".to_string(),
        "claude-3-5-sonnet-20241022" => "sonnet".to_string(),
        "claude-3-5-sonnet-20240620" => "sonnet".to_string(),
        "claude-3-5-haiku-20241022" => "haiku".to_string(),
        "claude-3-haiku-20240307" => "haiku".to_string(),

        // ========== 兼容短名称 ==========
        "opus" | "claude-opus" => "opus".to_string(),
        "sonnet" | "claude-sonnet" => "sonnet".to_string(),
        "haiku" | "claude-haiku" => "haiku".to_string(),
        "codex" | "claude-codex" => "codex".to_string(),

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

// ========== Claude Code 专用认证中间件 ==========

/// Claude Code 兼容认证中间件
/// 支持: x-api-key (Anthropic 标准) 和 Authorization Bearer (OpenAI 格式)
async fn claude_auth_middleware(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<axum::response::Response, AnthropicErrorResponse> {
    // 跳过公开端点
    let path = req.uri().path();
    let public_paths = [
        "/health",
        "/v1/models",
        "/api/apply",
        "/api/apply/check-availability",
        "/api/user/login",
        "/api/user/register",
        "/api/health",
        "/api/stats",
        "/api/admin/",
    ];
    let is_public = public_paths.iter().any(|p| path.starts_with(p)) || path.starts_with("/v1/auth/") || path.starts_with("/api/admin/");
    if is_public {
        return Ok(next.run(req).await);
    }

    // 1. 优先使用 x-api-key (Claude Code / Anthropic 标准)
    let token = if let Some(api_key) = req.headers()
        .get("x-api-key")
        .and_then(|h| h.to_str().ok())
    {
        api_key.to_string()
    }
    // 2. 回退到 Authorization Bearer (兼容 OpenAI 客户端)
    else if let Some(auth_header) = req.headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
    {
        if !auth_header.starts_with("Bearer ") {
            return Err(AnthropicErrorResponse {
                error_type: "error".to_string(),
                error: anthropic::AnthropicErrorDetail {
                    detail_type: "invalid_request_error".to_string(),
                    message: "无效的Authorization格式，应为 'Bearer <token>'".to_string(),
                },
            });
        }
        auth_header[7..].to_string()
    }
    else {
        return Err(AnthropicErrorResponse {
            error_type: "error".to_string(),
            error: anthropic::AnthropicErrorDetail {
                detail_type: "authentication_error".to_string(),
                message: "缺少认证信息 (需要 x-api-key 或 Authorization 头)".to_string(),
            },
        });
    };

    // 验证 Token (支持 JWT 和 API Key)
    let user_info = match state.auth.verify_token(&token).await {
        Ok(user) => user,
        Err(err) => {
            return Err(AnthropicErrorResponse {
                error_type: "error".to_string(),
                error: anthropic::AnthropicErrorDetail {
                    detail_type: match err {
                        AuthError::InvalidToken(_) => "authentication_error",
                        AuthError::TokenExpired => "authentication_error",
                        AuthError::TokenRevoked => "authentication_error",
                        AuthError::UserNotFound(_) => "authentication_error",
                        AuthError::Forbidden => "permission_error",
                        _ => "api_error",
                    }.to_string(),
                    message: err.to_string(),
                },
            });
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

    req.extensions_mut().insert(auth_user.clone());
    req.extensions_mut().insert(user_info);

    Ok(next.run(req).await)
}

// ========== Handler ==========

async fn health_handler(State(state): State<AppState>) -> Result<Json<HealthResponse>, AuthError> {
    let db_status = sqlx::query("SELECT 1")
        .fetch_one(&state.db)
        .await
        .map(|_| "connected".to_string())
        .unwrap_or_else(|_| "disconnected".to_string());

    let redis_status = state.redis
        .get_async_connection()
        .await
        .map(|_: redis::aio::Connection| "connected".to_string())
        .unwrap_or_else(|_| "disconnected".to_string());

    let cache_stats = state.auth.cache_stats();

    Ok(Json(HealthResponse {
        status: "healthy".to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        version: "4.0.0-claude-code".to_string(),
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
    let pools = state.pool_manager.list_pools().await;
    let pool_ids: Vec<String> = pools.iter().map(|p| p.pool_id.clone()).collect();
    let mut models = vec![];

    // ========== Opus 系列 ==========
    if pool_ids.contains(&"opus".to_string()) {
        models.extend(vec![
            ModelInfo { id: "claude-opus-4-6".to_string(), created_at: "2026-02-04T00:00:00Z".to_string(), display_name: "Claude Opus 4.6".to_string(), model_type: "model".to_string() },
            ModelInfo { id: "claude-opus-4-5-20250929".to_string(), created_at: "2025-09-29T00:00:00Z".to_string(), display_name: "Claude Opus 4.5".to_string(), model_type: "model".to_string() },
            ModelInfo { id: "claude-opus-4".to_string(), created_at: "2025-05-14T00:00:00Z".to_string(), display_name: "Claude Opus 4".to_string(), model_type: "model".to_string() },
            ModelInfo { id: "claude-opus-4-20250514".to_string(), created_at: "2025-05-14T00:00:00Z".to_string(), display_name: "Claude Opus 4 (20250514)".to_string(), model_type: "model".to_string() },
            ModelInfo { id: "claude-3-opus-20240229".to_string(), created_at: "2024-02-29T00:00:00Z".to_string(), display_name: "Claude 3 Opus".to_string(), model_type: "model".to_string() },
        ]);
    }

    // ========== Sonnet 系列 ==========
    if pool_ids.contains(&"sonnet".to_string()) {
        models.extend(vec![
            ModelInfo { id: "claude-sonnet-4-5-20250929".to_string(), created_at: "2025-09-29T00:00:00Z".to_string(), display_name: "Claude Sonnet 4.5".to_string(), model_type: "model".to_string() },
            ModelInfo { id: "claude-sonnet-4".to_string(), created_at: "2025-05-14T00:00:00Z".to_string(), display_name: "Claude Sonnet 4".to_string(), model_type: "model".to_string() },
            ModelInfo { id: "claude-sonnet-4-20250514".to_string(), created_at: "2025-05-14T00:00:00Z".to_string(), display_name: "Claude Sonnet 4 (20250514)".to_string(), model_type: "model".to_string() },
            ModelInfo { id: "claude-3-5-sonnet-20241022".to_string(), created_at: "2024-10-22T00:00:00Z".to_string(), display_name: "Claude 3.5 Sonnet (Latest)".to_string(), model_type: "model".to_string() },
            ModelInfo { id: "claude-3-5-sonnet-20240620".to_string(), created_at: "2024-06-20T00:00:00Z".to_string(), display_name: "Claude 3.5 Sonnet".to_string(), model_type: "model".to_string() },
        ]);
    }

    // ========== Haiku 系列 ==========
    if pool_ids.contains(&"haiku".to_string()) {
        models.extend(vec![
            ModelInfo { id: "claude-haiku-4-5-20251001".to_string(), created_at: "2025-10-01T00:00:00Z".to_string(), display_name: "Claude Haiku 4.5".to_string(), model_type: "model".to_string() },
            ModelInfo { id: "claude-haiku-4-20250514".to_string(), created_at: "2025-05-14T00:00:00Z".to_string(), display_name: "Claude Haiku 4".to_string(), model_type: "model".to_string() },
            ModelInfo { id: "claude-3-5-haiku-20241022".to_string(), created_at: "2024-10-22T00:00:00Z".to_string(), display_name: "Claude 3.5 Haiku".to_string(), model_type: "model".to_string() },
            ModelInfo { id: "claude-3-haiku-20240307".to_string(), created_at: "2024-03-07T00:00:00Z".to_string(), display_name: "Claude 3 Haiku".to_string(), model_type: "model".to_string() },
        ]);
    }

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

/// 旧的 Anthropic Messages API Handler (兼容旧代码，已废弃)
/// 请使用 anthropic::anthropic_messages_handler 代替
async fn anthropic_messages_handler_legacy(
    State(state): State<AppState>,
    headers: HeaderMap,
    Extension(user): Extension<AuthenticatedUser>,
    Json(req): AnthropicMessagesRequestLegacy,
) -> Result<Json<AnthropicMessagesResponseLegacy>, AuthError> {
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

    let pool_id = map_anthropic_model(&req.model);
    let chat_req = ChatRequest {
        model: pool_id.clone(),
        messages: req.messages.iter().map(anthropic_to_chat).collect(),
        stream: req.stream,
        max_tokens: req.max_tokens,
    };

    let estimated_input_tokens: u32 = chat_req.messages.iter()
        .map(|m| m.content.len() as u32 / 4)
        .sum();
    let estimated_cost = calculate_cost(&pool_id, estimated_input_tokens, chat_req.max_tokens);

    if user.balance < estimated_cost {
        return Err(AuthError::Forbidden);
    }

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
            return Err(AuthError::Internal(format!("API调用失败: {}", e)));
        }
    };

    let input_tokens = if actual_input_tokens > 0 { actual_input_tokens } else { estimated_input_tokens };
    let output_tokens = if actual_output_tokens > 0 { actual_output_tokens } else { (response_text.len() as u32 / 4) + 1 };
    let actual_cost = calculate_cost(&pool_id, input_tokens, output_tokens);

    sqlx::query(
        "UPDATE users SET balance = balance - $1, updated_at_old = EXTRACT(EPOCH FROM NOW())::BIGINT, updated_at = NOW()
         WHERE user_id = $2"
    )
    .bind(actual_cost)
    .bind(&user.user_id)
    .execute(&state.db)
    .await
    .map_err(|e| AuthError::Database(e.to_string()))?;

    let elapsed = start_time.elapsed().as_millis();
    info!(request_id = %request_id, user_id = %user.user_id, model = %req.model,
          pool_id = %pool_id, provider = %actual_provider, cost = actual_cost, elapsed_ms = elapsed, "Anthropic messages");

    Ok(Json(AnthropicMessagesResponseLegacy {
        id: request_id.to_string(),
        response_type: "message".to_string(),
        role: "assistant".to_string(),
        content: vec![AnthropicContentBlock {
            content_type: "text".to_string(),
            text: response_text,
        }],
        model: req.model,
        stop_reason: "end_turn".to_string(),
        usage: AnthropicUsage {
            input_tokens,
            output_tokens,
        },
    }))
}

// 其他 handler 保持不变...
// (省略其他 handler 以节省空间，实际使用时需要保留原有的所有 handler)

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
            password_hash TEXT,
            email VARCHAR(256),
            created_at_old BIGINT NOT NULL,
            updated_at_old BIGINT NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#
    )
    .execute(pool)
    .await?;

    // 创建用户API密钥表
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS user_api_keys (
            id BIGSERIAL PRIMARY KEY,
            api_key VARCHAR(128) UNIQUE NOT NULL,
            user_id VARCHAR(64) NOT NULL REFERENCES users(user_id),
            is_active BOOLEAN NOT NULL DEFAULT true,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#
    )
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

    // 创建充值卡表
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS recharge_cards (
            card_id SERIAL PRIMARY KEY,
            card_code VARCHAR(20) UNIQUE NOT NULL,
            amount BIGINT NOT NULL,
            used BOOLEAN NOT NULL DEFAULT false,
            used_by VARCHAR(64),
            used_at TIMESTAMPTZ,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#
    )
    .execute(pool)
    .await?;

    info!("数据库初始化完成");
    Ok(())
}

// ========== 密钥轮换任务 ==========

async fn key_rotation_task(auth: Arc<JwtAuthService>) {
    let mut interval = interval(Duration::from_secs(3600));
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

// ========== Main ==========

#[tokio::main]
async fn main() -> anyhow::Result<()> {
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
    info!("API网关启动中 - Claude Code 兼容版");
    info!("========================================");
    info!("版本: 4.0.0 (Claude Code Compatible)");
    info!("数据库: {}", config.database_url);
    info!("Redis: {}", config.redis_url);

    // 连接PostgreSQL
    let db_pool = PgPoolOptions::new()
        .max_connections(20)
        .connect(&config.database_url)
        .await
        .map_err(|e| anyhow::anyhow!("数据库连接失败: {}", e))?;

    info!("PostgreSQL 连接成功");

    init_database(&db_pool).await?;

    // 连接Redis
    let redis_client = RedisClient::open(config.redis_url.as_str())
        .map_err(|e| anyhow::anyhow!("Redis连接失败: {}", e))?;

    let _ = redis_client.get_async_connection().await
        .map_err(|e| anyhow::anyhow!("Redis连接验证失败: {}", e))?;

    info!("Redis 连接成功");

    // 创建JWT认证服务
    let auth = Arc::new(JwtAuthService::new(db_pool.clone(), &config.redis_url).await?);
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
        // ========== Claude Code 专用端点 ==========
        .route("/v1/messages", post(anthropic_messages_handler))  // 使用新的 anthropic 模块
        .route("/v1/models", get(models_handler))

        // ========== 兼容 OpenAI 格式 ==========
        .route("/v1/chat/completions", post(crate::chat_handler))

        // ========== 健康检查 ==========
        .route("/health", get(health_handler))

        .layer(CorsLayer::new().allow_origin(tower_http::cors::Any))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            claude_auth_middleware,  // 使用 Claude Code 兼容的认证中间件
        ))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(config.server_addr).await?;
    info!("========================================");
    info!("API网关已启动: http://{}", config.server_addr);
    info!("健康检查: http://{}/health", config.server_addr);
    info!("Claude Code 端点: http://{}/v1/messages", config.server_addr);
    info!("========================================");

    axum::serve(listener, app).await?;

    Ok(())
}

// ========== chat_handler 占位函数（需要从原 main.rs 复制完整实现）==========
async fn chat_handler(
    State(_state): State<AppState>,
    _headers: HeaderMap,
    Extension(_user): Extension<AuthenticatedUser>,
    Json(_req): Json<ChatRequest>,
) -> Result<Json<ChatResponse>, AuthError> {
    // 这里需要从原 main.rs 复制完整的 chat_handler 实现
    Err(AuthError::Internal("需要实现完整逻辑".to_string()))
}
