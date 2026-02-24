//! Axum服务器集成示例

use auth_system::{
    cache::RedisConfig,
    middleware::{auth_middleware, require_model_permission, AuthenticatedUser},
    models::{TokenType, UserInfo},
    service::AuthService,
    AuthConfig,
};
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::{net::SocketAddr, sync::Arc};
use tower_http::trace::TraceLayer;
use tracing::{info, Level};
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    info!("启动认证系统示例服务器...");

    // 加载配置
    let config = AuthConfig::from_env()?;

    // 连接数据库
    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "postgresql://localhost/auth_db".to_string());
    let db = PgPool::connect(&db_url).await?;
    info!("数据库连接成功: {}", db_url);

    // 创建缓存管理器
    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
    let redis_config = RedisConfig {
        url: redis_url.clone(),
        pool_max_size: Some(10),
        pool_min_idle: Some(2),
    };
    let cache = auth_system::cache::CacheManager::new(redis_config, 1000, 3600);
    info!("Redis连接成功: {}", redis_url);

    // 创建认证服务
    let auth_service = Arc::new(AuthService::new(db, cache, config));

    // 构建路由
    let app = Router::new()
        // 公开路由
        .route("/health", get(health_check))
        .route("/login", post(login))
        .route("/logout", post(logout))
        // 受保护路由
        .route("/profile", get(get_profile))
        .route("/chat", post(chat))
        .route("/admin", get(admin_panel))
        // 认证中间件 (应用于所有后续路由)
        .route_layer(axum::middleware::from_fn_with_state(
            auth_service.clone(),
            auth_middleware,
        ))
        .with_state(auth_service.clone());

    // 启动服务器
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    info!("服务器启动成功: http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

/// 健康检查 (无需认证)
async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "ok",
        "version": auth_system::VERSION,
    }))
}

/// 用户登录
async fn login(
    State(auth): State<Arc<AuthService>>,
    Json(payload): Json<LoginRequest>,
) -> Result<impl IntoResponse, AuthError> {
    // 在实际应用中, 这里应该验证用户名和密码
    // 这里为了演示, 直接使用user_id生成Token

    let token = auth
        .login(payload.user_id, TokenType::Short)
        .await?;

    Ok(Json(LoginResponse {
        access_token: token,
        token_type: "Bearer".to_string(),
        expires_in: 86400,
    }))
}

/// 用户登出
async fn logout(
    State(auth): State<Arc<AuthService>>,
    user: AuthenticatedUser,
) -> impl IntoResponse {
    // 在实际应用中, 这里应该撤销Token
    info!("用户登出: user_id={}", user.user_id);

    Json(serde_json::json!({
        "message": "登出成功",
    }))
}

/// 获取用户资料 (需要认证)
async fn get_profile(user: AuthenticatedUser) -> impl IntoResponse {
    Json(serde_json::json!({
        "user_id": user.user_id,
        "username": user.username,
        "email": user.email,
        "tier": user.tier,
    }))
}

/// 聊天接口 (需要模型权限)
async fn chat(
    user: AuthenticatedUser,
    State(auth): State<Arc<AuthService>>,
    Json(payload): Json<ChatRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    // 检查模型权限
    if !user.can_use_model(&payload.model) {
        return Err(StatusCode::FORBIDDEN);
    }

    info!(
        "聊天请求: user_id={}, model={}",
        user.user_id, payload.model
    );

    // 处理聊天逻辑...
    Ok(Json(ChatResponse {
        message: format!("你好, {}! 你正在使用 {} 模型", user.username, payload.model),
        model: payload.model,
    }))
}

/// 管理面板 (需要admin权限)
async fn admin_panel(
    user: AuthenticatedUser,
) -> Result<impl IntoResponse, StatusCode> {
    // 检查admin权限
    if !user.has_scope("admin") {
        return Err(StatusCode::FORBIDDEN);
    }

    Ok(Json(serde_json::json!({
        "message": "欢迎来到管理面板",
        "admin_user": user.username,
    })))
}

/// 使用模型权限检查中间件的替代实现
/// 在Router中使用:
/// .route_layer(axum::middleware::from_fn(
///     require_model_permission("opus")
/// ))
async fn chat_with_middleware(
    user: AuthenticatedUser,
    Json(payload): Json<ChatRequest>,
) -> impl IntoResponse {
    // 到达这里说明已经通过了模型权限检查
    info!("聊天请求 (已通过权限检查): user_id={}, model={}", user.user_id, payload.model);

    Json(ChatResponse {
        message: format!("处理聊天: {}", payload.message),
        model: payload.model,
    })
}

// ===== 请求/响应类型 =====

#[derive(Debug, Deserialize)]
struct LoginRequest {
    user_id: i64,
    // username: String,
    // password: String,
}

#[derive(Debug, Serialize)]
struct LoginResponse {
    access_token: String,
    token_type: String,
    expires_in: i64,
}

#[derive(Debug, Deserialize)]
struct ChatRequest {
    model: String,
    message: String,
}

#[derive(Debug, Serialize)]
struct ChatResponse {
    message: String,
    model: String,
}
