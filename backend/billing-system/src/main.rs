mod config;
mod models;
mod database;
mod cache;
mod auth;
mod billing;
mod proxy;
mod key_manager;
mod recharge;
mod ratelimit;
mod scheduler;
mod api;

use axum::{
    routing::{get, post},
    Router,
};
use tower::ServiceBuilder;
use tower_http::{cors::CorsLayer, trace::TraceLayer, services::ServeDir};
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use api::AppState;
use config::AppConfig;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let config = AppConfig::load()?;

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "billing_gateway=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting Aitachi Billing Gateway...");

    // 初始化数据库
    let db = Arc::new(database::Database::new(&config.database_url).await?);
    tracing::info!("Database connected");

    // 初始化缓存
    let cache = Arc::new(cache::Cache::new(&config.redis_url).await?);
    tracing::info!("Redis connected");

    // 初始化Key管理器
    let key_manager = Arc::new(key_manager::KeyManager::new(db.clone(), cache.clone()));
    key_manager.load_keys().await?;
    tracing::info!("Key manager initialized with {} keys", key_manager.get_all_keys_status().await.len());

    // 初始化模型池
    let model_pool = Arc::new(proxy::ModelPool::new(cache.clone(), key_manager.clone()));
    // 启动健康检查任务
    model_pool.clone().start_health_checker();
    tracing::info!("Model pool initialized");

    // 初始化计费服务
    let billing = Arc::new(billing::BillingService::new(db.clone(), cache.clone()).await?);
    tracing::info!("Billing service initialized");

    // 初始化充值服务
    let recharge = Arc::new(recharge::RechargeService::new(db.clone(), cache.clone()));
    tracing::info!("Recharge service initialized");

    // 初始化Token限流器
    let rate_limiter = Arc::new(ratelimit::TokenRateLimiter::new());
    tracing::info!("Token rate limiter initialized");

    // 初始化定时任务
    let scheduler = Arc::new(scheduler::Scheduler::new(db.clone(), cache.clone()));
    scheduler.start().await;
    tracing::info!("Scheduler tasks started");

    // 构建应用状态
    let state = AppState {
        db: db.clone(),
        cache: cache.clone(),
        billing: billing.clone(),
        model_pool: model_pool.clone(),
        key_manager: key_manager.clone(),
        recharge: recharge.clone(),
        rate_limiter: rate_limiter.clone(),
        scheduler: scheduler.clone(),
    };

    // 构建路由
    let app = Router::new()
        // Web管理面板
        .route("/", get(api::web_panel))
        .route("/favicon.ico", get(api::favicon))
        .nest_service("/static", ServeDir::new("static"))
        // 健康检查
        .route("/health", get(api::health_check))
        .route("/api/health/model", get(api::model_health))
        .route("/api/health/model/:model", get(api::check_model_health))
        .route("/api/admin/models/stats", get(api::model_stats))
        // 用户相关
        .route("/api/user/register", post(api::register_user))
        .route("/api/user/login", post(api::password_login))
        .route("/api/user/email-login", post(api::email_login))
        .route("/api/user/balance", get(api::get_balance))
        .route("/api/user/profile", get(api::get_profile))
        .route("/api/user/profile", post(api::update_profile))
        .route("/api/user/change-password", post(api::change_password))
        .route("/api/user/history", get(api::get_billing_history))
        // 聊天接口
        .route("/v1/chat/completions", post(api::chat_completion))
        .route("/v1/chat/completions/stream", post(api::chat_completion_stream))
        // 充值码接口
        .route("/api/recharge/redeem", post(api::redeem_code))
        .route("/api/recharge/create", post(api::create_recharge_batch))
        .route("/api/recharge/code/:code", get(api::get_code_stats))
        .route("/api/recharge/batch/:batch_id", get(api::get_batch_stats))
        // API申请相关
        .route("/api/apply", post(api::apply_api_key))
        .route("/api/apply/check-availability/username/:username", get(api::check_username_availability))
        // API统计
        .route("/api/stats", get(api::get_api_stats))
        // 管理端接口
        .route("/api/admin/overview", get(api::admin_overview))
        .route("/api/admin/users", get(api::admin_get_users))
        .route("/api/admin/billing", get(api::admin_get_billing))
        .route("/api/admin/recharge/stats", get(api::admin_recharge_stats))
        .route("/api/admin/recharge/batches", get(api::admin_recharge_batches))
        .route("/api/admin/reconciliation", get(api::admin_get_reconciliation))
        .route("/api/admin/scheduler/pending", get(api::admin_scheduler_pending))
        .route("/api/admin/keys/status", get(api::get_keys_status))
        // 管理员认证
        .route("/api/admin/login", post(api::admin_login))
        .route("/api/admin/session", get(api::verify_admin_session))
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CorsLayer::permissive()),
        )
        .with_state(state);

    let addr = format!("{}:{}", config.host, config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("Server listening on {}", addr);

    axum::serve(listener, app).await?;

    Ok(())
}
