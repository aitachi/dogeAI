use crate::models::{ApiError, UserContext};
use axum::{
    extract::{Request, State},
    http::HeaderMap,
    middleware::Next,
    response::Response,
};
use std::sync::Arc;

/// 应用状态
#[derive(Clone)]
pub struct AppState {
    pub db: Arc<crate::services::DbPool>,
    pub billing: Arc<crate::services::BillingService>,
    pub redis: Arc<crate::services::RedisPool>,
    pub token_cache: Arc<crate::services::TokenCache>,
    pub rate_limiter: Arc<crate::services::RateLimiter>,
    pub model_service: Arc<crate::services::ModelService>,
    pub upstream: Arc<crate::services::UpstreamClient>,
}

/// Token验证中间件
pub async fn auth_middleware(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<Response, ApiError> {
    // 1. 提取Authorization头
    let auth_header = req
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| ApiError::Unauthorized)?;

    // 2. 验证Bearer格式
    if !auth_header.starts_with("Bearer ") {
        return Err(ApiError::InvalidToken(
            "无效的Authorization格式".to_string(),
        ));
    }

    let token = &auth_header[7..];

    // 3. 先查缓存（moka + Redis）
    let user_id_opt = state.token_cache.get_user_id(token).await?;

    let user_id = if let Some(uid) = user_id_opt {
        uid
    } else {
        // 4. 缓存未命中，查数据库
        let uid = state
            .billing
            .validate_token(token)
            .await?
            .ok_or_else(|| ApiError::InvalidToken("Token不存在或已失效".to_string()))?;

        // 5. 更新缓存
        state
            .token_cache
            .set_token(token, uid, std::time::Duration::from_secs(3600))
            .await?;

        uid
    };

    // 6. 获取用户套餐等级
    let tier = state.billing.get_user_tier(user_id).await?;

    // 7. 将用户信息存入请求扩展
    let user_context = UserContext {
        user_id,
        token: token.to_string(),
        tier,
    };
    req.extensions_mut().insert(user_context);

    // 8. 继续处理请求
    Ok(next.run(req).await)
}

/// 从请求中提取用户上下文
pub fn extract_user_context(req: &Request) -> Result<UserContext, ApiError> {
    req.extensions()
        .get::<UserContext>()
        .cloned()
        .ok_or_else(|| ApiError::Unauthorized)
}

/// 从请求中提取用户ID
pub fn extract_user_id(req: &Request) -> Result<i64, ApiError> {
    Ok(extract_user_context(req)?.user_id)
}

/// 提取真实IP（考虑代理）
pub fn extract_real_ip(headers: &HeaderMap) -> Option<String> {
    // 尝试从各种代理头中获取真实IP
    headers
        .get("x-forwarded-for")
        .or_else(|| headers.get("x-real-ip"))
        .or_else(|| headers.get("cf-connecting-ip"))
        .and_then(|h| h.to_str().ok())
        .map(|s| {
            // X-Forwarded-For 可能包含多个IP，取第一个
            s.split(',')
                .next()
                .unwrap_or("")
                .trim()
                .to_string()
        })
}
