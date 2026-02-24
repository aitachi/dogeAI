//! # Token认证系统
//!
//! 高性能JWT认证系统, 支持多层缓存、权限管理、Token撤销等功能。
//!
//! ## 功能特性
//!
//! - **JWT Token验证**: 基于HS256算法的JWT Token生成和验证
//! - **多层缓存**: L1(moka本地) + L2(Redis)双层缓存, 验证延迟<0.5ms
//! - **权限管理**: 基于用户等级和权限范围的细粒度权限控制
//! - **Token撤销**: 支持用户维度和单个Token的撤销机制
//! - **Token续期**: 自动检测并续期即将过期的Token
//! - **Axum集成**: 提供开箱即用的认证中间件和提取器
//!
//! ## 快速开始
//!
//! ```rust,no_run
//! use auth_system::{AuthService, CacheManager, AuthConfig};
//! use auth_system::cache::RedisConfig;
//! use sqlx::PgPool;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // 1. 加载配置
//!     let config = AuthConfig::from_env()?;
//!
//!     // 2. 创建数据库连接
//!     let db = PgPool::connect(&std::env::var("DATABASE_URL")?).await?;
//!
//!     // 3. 创建缓存管理器
//!     let redis_config = RedisConfig::default();
//!     let cache = CacheManager::new(redis_config, 1000, 3600);
//!
//!     // 4. 创建认证服务
//!     let auth_service = AuthService::new(db, cache, config);
//!
//!     // 5. 生成Token
//!     let token = auth_service.login(12345, auth_system::models::TokenType::Short).await?;
//!
//!     // 6. 验证Token
//!     let user_info = auth_service.verify_token(&token).await?;
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Axum集成示例
//!
//! ```rust,no_run
//! use axum::{Router, routing::get};
//! use auth_system::middleware::{auth_middleware, AuthenticatedUser};
//! use std::sync::Arc;
//!
//! async fn protected_handler(user: AuthenticatedUser) -> String {
//!     format!("Hello, {}!", user.username)
//! }
//!
//! #[tokio::main]
//! async fn main() {
//!     let app = Router::new()
//!         .route("/protected", get(protected_handler))
//!         .route_layer(axum::middleware::from_fn_with_state(
//!             Arc::new(auth_service),
//!             auth_middleware,
//!         ));
//! }
//! ```

pub mod cache;
pub mod config;
pub mod error;
pub mod middleware;
pub mod models;
pub mod service;

// 重新导出常用类型
pub use error::{AuthError, AuthResult};
pub use middleware::AuthenticatedUser;
pub use models::{Claims, Permission, TokenType, UserInfo};
pub use service::AuthService;

/// 当前版本
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// 默认Token有效期 (24小时)
pub const DEFAULT_TOKEN_TTL_SECS: i64 = 86400;

/// 默认续期窗口 (12小时)
pub const DEFAULT_REFRESH_WINDOW_SECS: i64 = 43200;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert_eq!(VERSION, "2.0.0");
    }

    #[test]
    fn test_constants() {
        assert_eq!(DEFAULT_TOKEN_TTL_SECS, 86400);
        assert_eq!(DEFAULT_REFRESH_WINDOW_SECS, 43200);
    }
}
