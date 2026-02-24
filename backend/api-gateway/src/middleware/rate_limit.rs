use crate::middleware::auth::{extract_user_context, extract_real_ip};
use crate::models::{ApiError, RateLimitResult};
use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde_json::json;

/// 应用状态类型（简化版，实际应该从auth模块导入）
#[derive(Clone)]
pub struct AppState {
    pub rate_limiter: std::sync::Arc<crate::services::RateLimiter>,
}

/// 限流中间件
pub async fn rate_limit_middleware(
    State(state): State<super::auth::AppState>,
    req: Request,
    next: Next,
) -> Result<Response, Response> {
    // 1. 提取用户上下文（如果已认证）
    let user_context = req.extensions().get::<crate::models::UserContext>();

    // 2. 用户级限流
    if let Some(ctx) = user_context {
        let result = state
            .rate_limiter
            .allow_user(ctx.user_id, ctx.tier)
            .await
            .unwrap_or_else(|_| RateLimitResult::Limited { retry_after: 1 });

        if let RateLimitResult::Limited { retry_after } = result {
            return Ok(rate_limit_response(ctx.tier.qps_limit(), ctx.tier.qps_limit() + 1, retry_after));
        }
    }

    // 3. IP级限流（可选，防止滥用）
    if let Some(ip) = extract_real_ip(req.headers()) {
        let result = state
            .rate_limiter
            .allow_ip(&ip, 500)
            .await
            .unwrap_or_else(|_| RateLimitResult::Limited { retry_after: 1 });

        if let RateLimitResult::Limited { retry_after } = result {
            return Ok(rate_limit_response(500, 501, retry_after));
        }
    }

    // 4. 全局限流（防止系统过载）
    let global_result = state
        .rate_limiter
        .check_global_limit(5000)
        .await
        .unwrap_or_else(|_| RateLimitResult::Limited { retry_after: 1 });

    if let RateLimitResult::Limited { retry_after } = global_result {
        return Ok(rate_limit_response(5000, 5001, retry_after));
    }

    // 5. 通过限流，继续处理
    Ok(next.run(req).await)
}

/// 构造限流响应
fn rate_limit_response(limit: usize, used: usize, retry_after: u64) -> Response {
    let body = json!({
        "error": "rate_limit_exceeded",
        "message": "请求过于频繁，请稍后重试",
        "retry_after": retry_after,
        "limit": limit,
        "used": used
    });

    let mut response = axum::Json(body).into_response();
    *response.status_mut() = StatusCode::TOO_MANY_REQUESTS;

    // 添加限流相关响应头
    let headers = response.headers_mut();
    headers.insert("X-RateLimit-Limit", limit.to_string().parse().unwrap());
    headers.insert("X-RateLimit-Used", used.to_string().parse().unwrap());
    headers.insert("X-RateLimit-Remaining", "0".parse().unwrap());
    headers.insert("X-RateLimit-Reset", (chrono::Utc::now().timestamp() + retry_after as i64).to_string().parse().unwrap());
    headers.insert("Retry-After", retry_after.to_string().parse().unwrap());

    response
}

