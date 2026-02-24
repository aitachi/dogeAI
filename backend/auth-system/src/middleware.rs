//! Axum中间件集成

use crate::error::{AuthError, AuthResult};
use crate::models::{permission_from_user_info, Permission};
use crate::service::AuthService;
use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
    Extension,
};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tracing::{debug, warn};

/// 已认证的用户信息
#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    pub user_id: i64,
    pub username: String,
    pub email: String,
    pub tier: String,
    pub permission: Permission,
}

impl AuthenticatedUser {
    pub fn can_use_model(&self, model: &str) -> bool {
        self.permission.can_use_model(model)
    }

    pub fn has_scope(&self, scope: &str) -> bool {
        self.permission.has_scope(scope)
    }
}

/// 认证中间件
pub async fn auth_middleware(
    State(auth): State<Arc<AuthService>>,
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // 提取Authorization头
    let auth_header = req
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    debug!("认证中间件: 验证请求");

    // 验证Token
    let user_info = auth
        .verify_token(auth_header)
        .await
        .map_err(|e| {
            warn!("Token验证失败: {}", e);
            StatusCode::UNAUTHORIZED
        })?;

    // 创建权限对象
    let permission = permission_from_user_info(&user_info);

    // 创建认证用户对象
    let authenticated_user = AuthenticatedUser {
        user_id: user_info.user_id,
        username: user_info.username.clone(),
        email: user_info.email.clone(),
        tier: user_info.tier.clone(),
        permission,
    };

    // 将用户信息添加到请求扩展
    req.extensions_mut().insert(authenticated_user.clone());
    req.extensions_mut().insert(user_info);

    debug!("认证成功: user_id={}, tier={}", authenticated_user.user_id, authenticated_user.tier);

    Ok(next.run(req).await)
}

/// 可选认证中间件 (允许未认证的请求通过)
pub async fn optional_auth_middleware(
    State(auth): State<Arc<AuthService>>,
    mut req: Request,
    next: Next,
) -> Response {
    // 尝试提取Authorization头
    if let Some(auth_header) = req
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
    {
        // 尝试验证Token
        if let Ok(user_info) = auth.verify_token(auth_header).await {
            let permission = permission_from_user_info(&user_info);

            let authenticated_user = AuthenticatedUser {
                user_id: user_info.user_id,
                username: user_info.username.clone(),
                email: user_info.email.clone(),
                tier: user_info.tier.clone(),
                permission,
            };

            let user_id = authenticated_user.user_id;
            req.extensions_mut().insert(authenticated_user);
            req.extensions_mut().insert(user_info);

            debug!("可选认证成功: user_id={}", user_id);
        } else {
            debug!("可选认证失败, 继续为匿名请求");
        }
    }

    next.run(req).await
}

/// 模型权限检查中间件工厂
pub fn require_model_permission(model: &'static str) -> impl Fn(Request, Next) -> Pin<Box<dyn Future<Output = Result<Response, StatusCode>> + Send>> + Clone {
    move |req: Request, next: Next| {
        let model = model.to_string();
        Box::pin(async move {
            // 从扩展中获取认证用户
            let authenticated_user = req
                .extensions()
                .get::<AuthenticatedUser>()
                .ok_or(StatusCode::UNAUTHORIZED)?;

            if !authenticated_user.can_use_model(&model) {
                warn!(
                    "模型权限不足: user_id={}, model={}",
                    authenticated_user.user_id, model
                );
                return Err(StatusCode::FORBIDDEN);
            }

            debug!("模型权限检查通过: user_id={}, model={}", authenticated_user.user_id, model);

            Ok(next.run(req).await)
        })
    }
}

/// 权限范围检查中间件工厂
pub fn require_scope(required_scope: &'static str) -> impl Fn(Request, Next) -> Pin<Box<dyn Future<Output = Result<Response, StatusCode>> + Send>> + Clone {
    move |req: Request, next: Next| {
        let scope = required_scope.to_string();
        Box::pin(async move {
            // 从扩展中获取认证用户
            let authenticated_user = req
                .extensions()
                .get::<AuthenticatedUser>()
                .ok_or(StatusCode::UNAUTHORIZED)?;

            if !authenticated_user.has_scope(&scope) {
                warn!(
                    "权限范围不足: user_id={}, required_scope={}",
                    authenticated_user.user_id, scope
                );
                return Err(StatusCode::FORBIDDEN);
            }

            debug!("权限范围检查通过: user_id={}, scope={}", authenticated_user.user_id, scope);

            Ok(next.run(req).await)
        })
    }
}

/// 用户等级要求中间件工厂
pub fn require_tier(min_tier: &'static str) -> impl Fn(Request, Next) -> Pin<Box<dyn Future<Output = Result<Response, StatusCode>> + Send>> + Clone {
    move |req: Request, next: Next| {
        let tier = min_tier.to_string();
        Box::pin(async move {
            // 从扩展中获取认证用户
            let authenticated_user = req
                .extensions()
                .get::<AuthenticatedUser>()
                .ok_or(StatusCode::UNAUTHORIZED)?;

            // 定义用户等级层级
            let tier_hierarchy = vec!["Base", "Pro", "Max", "AMax", "Enterprise"];

            let user_tier_level = tier_hierarchy
                .iter()
                .position(|t| t == &authenticated_user.tier)
                .unwrap_or(0);

            let required_tier_level = tier_hierarchy
                .iter()
                .position(|t| t == &tier)
                .unwrap_or(0);

            if user_tier_level < required_tier_level {
                warn!(
                    "用户等级不足: user_id={}, user_tier={}, required_tier={}",
                    authenticated_user.user_id, authenticated_user.tier, tier
                );
                return Err(StatusCode::FORBIDDEN);
            }

            debug!("用户等级检查通过: user_id={}, tier={}", authenticated_user.user_id, authenticated_user.tier);

            Ok(next.run(req).await)
        })
    }
}

/// Token续期中间件
pub async fn token_refresh_middleware(
    State(auth): State<Arc<AuthService>>,
    req: Request,
    next: Next,
) -> Response {
    // 提取Authorization头
    let auth_header = match req
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
    {
        Some(h) => h,
        None => return next.run(req).await,
    };

    // 检查是否需要续期
    let needs_refresh = match auth.should_refresh(auth_header) {
        Ok(needs) => needs,
        Err(_) => return next.run(req).await,
    };

    if !needs_refresh {
        return next.run(req).await;
    }

    // 尝试续期
    match auth.verify_token(auth_header).await {
        Ok(user_info) => {
            match auth.refresh_token(auth_header, &user_info) {
                Ok(new_token) => {
                    debug!("Token已自动续期");

                    // 在响应头中返回新Token
                    let mut response = next.run(req).await;

                    response.headers_mut().insert(
                        "X-New-Token",
                        new_token.parse().unwrap(),
                    );

                    response.headers_mut().insert(
                        "X-Token-Refreshed",
                        "true".parse().unwrap(),
                    );

                    response
                }
                Err(_) => next.run(req).await,
            }
        }
        Err(_) => next.run(req).await,
    }
}

// Axum提取器实现
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use async_trait::async_trait;

#[async_trait]
impl<S> FromRequestParts<S> for AuthenticatedUser
where
    S: Send + Sync,
{
    type Rejection = StatusCode;

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<AuthenticatedUser>()
            .cloned()
            .ok_or(StatusCode::UNAUTHORIZED)
    }
}

/// 使用示例:
/// ```rust,no_run
/// use axum::{Router, routing::{get, post}, Json};
/// use auth_system::middleware::{AuthMiddleware, AuthenticatedUser};
/// use auth_system::models::UserInfo;
///
/// async fn protected_handler(
///     user: AuthenticatedUser,
/// ) -> Result<Json<&'static str>, StatusCode> {
///     // user.user_id, user.tier 可直接使用
///     // 已经过认证和权限检查
///     Ok(Json("Hello, authenticated user!"))
/// }
///
/// async fn chat_handler(
///     user: AuthenticatedUser,
///     Json(payload): Json<ChatRequest>,
/// ) -> Result<Json<ChatResponse>, StatusCode> {
///     // 检查模型权限
///     if !user.can_use_model(&payload.model) {
///         return Err(StatusCode::FORBIDDEN);
///     }
///
///     // 处理聊天请求
///     Ok(Json(response))
/// }
/// ```

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_authenticated_user() {
        // 测试代码
    }
}
