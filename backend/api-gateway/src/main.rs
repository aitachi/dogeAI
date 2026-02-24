mod models;
mod middleware;
mod services;
mod handlers;
mod config;

use axum::{
    routing::{get, post},
    Router,
};
use config::Config;
use middleware::AppState;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tracing::{Level, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. 加载配置
    let config = Config::load();

    // 2. 初始化日志
    init_logging(&config);

    info!("🚀 API网关启动中...");
    info!("📋 配置加载成功");
    info!("🌐 监听地址: {}:{}", config.server.host, config.server.port);

    // 3. 初始化数据库
    info!("🗄️  正在连接数据库...");
    let db = services::DbPool::new(&config.database.url).await?;
    info!("✅ 数据库连接成功");

    // 4. 初始化Redis
    info!("🔴 正在连接Redis...");
    let redis = services::RedisPool::new(&config.cache.redis_url).await?;
    info!("✅ Redis连接成功");

    // 4.5 初始化上游客户端
    info!("🌐 正在初始化上游API客户端...");
    let upstream_url = config.upstream.url.clone();
    let upstream_key = config.upstream.api_key.clone().unwrap_or_default();
    let upstream_client = Arc::new(services::UpstreamClient::new(upstream_url, upstream_key)?);
    info!("✅ 上游客户端初始化成功");

    // 5. 初始化服务
    let billing = Arc::new(services::BillingService::new(db.clone()));
    let model_service = Arc::new(services::ModelService::new());
    let token_cache = Arc::new(services::TokenCache::new(
        redis.clone(),
        config.cache.local_ttl_secs,
    ));
    let rate_limiter = Arc::new(services::RateLimiter::new(redis.clone()));

    // 6. 构建应用状态
    let state = AppState {
        db: Arc::new(db),
        billing: billing.clone(),
        redis: Arc::new(redis.clone()),
        token_cache: token_cache.clone(),
        rate_limiter: rate_limiter.clone(),
        model_service: model_service.clone(),
        upstream: upstream_client.clone(),
    };

    // 7. 构建路由
    let app = Router::new()
        // 健康检查（公开，无需认证）
        .route("/health", get(handlers::health_handler))

        // API v1 路由（OpenAI 兼容格式）
        .route("/v1/token/query", post(handlers::token_query_handler))
        .route("/v1/chat/completions", post(handlers::chat_handler))
        .route("/v1/models", get(handlers::models_handler))

        // Anthropic API 路由（Claude Code 原生格式）
        .route("/v1/messages", post(handlers::messages_handler))

        // CORS 层
        .layer(CorsLayer::permissive())

        // 认证中间件（除健康检查外的所有路由）
        .route_layer(axum::middleware::from_fn_with_state(
            state.clone(),
            middleware::auth_middleware,
        ))
        .with_state(state);

    // 8. 启动服务器
    let bind_address = format!("{}:{}", config.server.host, config.server.port);
    let listener = tokio::net::TcpListener::bind(&bind_address).await?;

    info!("🎉 API网关已启动!");
    info!("📍 监听地址: http://{}", bind_address);
    info!("🏥 健康检查: http://{}/health", bind_address);
    info!("📊 Anthropic API: http://{}/v1/messages", bind_address);

    axum::serve(listener, app).await?;

    Ok(())
}

/// 初始化日志系统
fn init_logging(config: &Config) {
    let log_level = match config.log.level.to_lowercase().as_str() {
        "trace" => Level::TRACE,
        "debug" => Level::DEBUG,
        "info" => Level::INFO,
        "warn" => Level::WARN,
        "error" => Level::ERROR,
        _ => Level::INFO,
    };

    if config.log.format == "json" {
        // JSON格式日志（生产环境）
        tracing_subscriber::registry()
            .with(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(log_level.to_string()))
            )
            .with(tracing_subscriber::fmt::layer().json())
            .init();
    } else {
        // 文本格式日志（开发环境）
        tracing_subscriber::registry()
            .with(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(log_level.to_string()))
            )
            .with(
                tracing_subscriber::fmt::layer()
                    .with_target(true)
                    .with_thread_ids(true)
            )
            .init();
    }
}
