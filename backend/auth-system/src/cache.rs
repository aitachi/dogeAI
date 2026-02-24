//! 多层缓存实现 (L1: moka + L2: Redis)

use crate::error::{AuthError, AuthResult};
use crate::models::UserInfo;
use deadpool_redis::{redis::AsyncCommands, Manager, Pool};
use moka::future::Cache;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, trace};

/// Redis连接池配置
#[derive(Debug, Clone)]
pub struct RedisConfig {
    pub url: String,
    pub pool_max_size: Option<usize>,
    pub pool_min_idle: Option<usize>,
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            url: "redis://127.0.0.1:6379".to_string(),
            pool_max_size: Some(10),
            pool_min_idle: Some(2),
        }
    }
}

/// 多层缓存管理器
#[derive(Clone)]
pub struct CacheManager {
    /// L1: 本地内存缓存
    local_cache: Arc<Cache<String, CachedUserInfo>>,

    /// L2: Redis连接池
    redis_pool: Arc<Pool>,
}

/// 缓存的用户信息 (包含过期时间)
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CachedUserInfo {
    user_info: UserInfo,
    cached_at: i64,
}

impl CachedUserInfo {
    fn new(user_info: UserInfo) -> Self {
        Self {
            user_info,
            cached_at: chrono::Utc::now().timestamp(),
        }
    }

    fn age_secs(&self) -> i64 {
        chrono::Utc::now().timestamp() - self.cached_at
    }
}

impl CacheManager {
    /// 创建新的缓存管理器
    pub fn new(redis_config: RedisConfig, local_capacity: u64, local_ttl_secs: u64) -> Self {
        // 创建Redis连接池 (处理 Manager::new 返回 Result)
        let manager = Manager::new(redis_config.url.clone()).expect("Invalid Redis URL");
        let pool = Pool::builder(manager)
            .max_size(redis_config.pool_max_size.unwrap_or(10))
            .build()
            .unwrap();

        // 创建本地缓存
        let local_cache = Cache::builder()
            .max_capacity(local_capacity)
            .time_to_live(Duration::from_secs(local_ttl_secs))
            .build();

        debug!(
            "缓存管理器初始化完成: L1容量={}, L1 TTL={}s, L2={}",
            local_capacity, local_ttl_secs, redis_config.url
        );

        Self {
            local_cache: Arc::new(local_cache),
            redis_pool: Arc::new(pool),
        }
    }

    /// 从缓存获取用户信息
    pub async fn get_user(&self, token: &str) -> AuthResult<Option<UserInfo>> {
        // 1. 尝试从L1本地缓存获取
        if let Some(cached) = self.local_cache.get(token).await {
            trace!("L1缓存命中: token前缀={}", &token[..8.min(token.len())]);
            return Ok(Some(cached.user_info));
        }

        // 2. 尝试从L2 Redis获取
        let redis_key = format!("auth:token:{}", token);
        let mut conn = self.redis_pool.get().await.map_err(|e| AuthError::InternalError(e.into()))?;

        let cached_json: Option<String> = conn.get(&redis_key).await?;

        if let Some(json) = cached_json {
            trace!("L2缓存命中: token前缀={}", &token[..8.min(token.len())]);

            // 反序列化
            let cached: CachedUserInfo = serde_json::from_str(&json)
                .map_err(|e| AuthError::InternalError(e.into()))?;

            // 回写到L1缓存
            self.local_cache.insert(token.to_string(), cached.clone()).await;

            return Ok(Some(cached.user_info));
        }

        trace!("缓存未命中: token前缀={}", &token[..8.min(token.len())]);
        Ok(None)
    }

    /// 缓存用户信息 (写入L1和L2)
    pub async fn put_user(&self, token: &str, user_info: &UserInfo, redis_ttl_secs: usize) -> AuthResult<()> {
        let cached = CachedUserInfo::new(user_info.clone());

        // 写入L1
        self.local_cache.insert(token.to_string(), cached.clone()).await;

        // 写入L2
        let redis_key = format!("auth:token:{}", token);
        let json = serde_json::to_string(&cached)
            .map_err(|e| AuthError::InternalError(e.into()))?;

        let mut conn = self.redis_pool.get().await.map_err(|e| AuthError::InternalError(e.into()))?;

        // SETEX: 设置带过期时间的键
        redis::cmd("SETEX")
            .arg(&redis_key)
            .arg(redis_ttl_secs)
            .arg(&json)
            .query_async::<_, ()>(&mut *conn)
            .await?;

        trace!("用户信息已缓存: user_id={}, token前缀={}", user_info.user_id, &token[..8.min(token.len())]);

        Ok(())
    }

    /// 使Token失效 (从L1和L2删除)
    pub async fn invalidate_token(&self, token: &str) -> AuthResult<()> {
        // 从L1删除
        self.local_cache.invalidate(token).await;

        // 从L2删除
        let redis_key = format!("auth:token:{}", token);
        let mut conn = self.redis_pool.get().await.map_err(|e| AuthError::InternalError(e.into()))?;

        redis::cmd("DEL")
            .arg(&redis_key)
            .query_async::<_, ()>(&mut *conn)
            .await?;

        debug!("Token缓存已清除: token前缀={}", &token[..8.min(token.len())]);

        Ok(())
    }

    /// 使用户的所有Token失效 (按前缀)
    pub async fn invalidate_user_all(&self, user_id: i64) -> AuthResult<()> {
        // 清除L1中该用户的所有缓存
        // 注意: moka v0.12不支持前缀删除, 清除所有缓存作为替代
        self.local_cache.invalidate_all();

        // Redis清除 (使用SCAN + DEL)
        let pattern = format!("auth:user:{}*", user_id);
        let mut conn = self.redis_pool.get().await.map_err(|e| AuthError::InternalError(e.into()))?;

        // 使用SCAN查找所有匹配的键
        let keys: Vec<String> = redis::cmd("KEYS")
            .arg(&pattern)
            .query_async(&mut *conn)
            .await?;

        if !keys.is_empty() {
            redis::cmd("DEL")
                .arg(&keys)
                .query_async::<_, ()>(&mut *conn)
                .await?;

            debug!("用户{}的所有缓存已清除, 共{}个键", user_id, keys.len());
        }

        Ok(())
    }

    /// 检查Token是否在黑名单中
    pub async fn is_blacklisted(&self, token: &str) -> AuthResult<bool> {
        // 检查Token黑名单
        let token_key = format!("auth:blacklist:token:{}", token);
        let mut conn = self.redis_pool.get().await.map_err(|e| AuthError::InternalError(e.into()))?;

        let exists: usize = redis::cmd("EXISTS")
            .arg(&token_key)
            .query_async(&mut *conn)
            .await?;

        Ok(exists > 0)
    }

    /// 将Token加入黑名单
    pub async fn add_to_blacklist(&self, token: &str, ttl: usize) -> AuthResult<()> {
        let key = format!("auth:blacklist:token:{}", token);
        let mut conn = self.redis_pool.get().await.map_err(|e| AuthError::InternalError(e.into()))?;

        redis::cmd("SETEX")
            .arg(&key)
            .arg(ttl)
            .arg("1")
            .query_async::<_, ()>(&mut *conn)
            .await?;

        debug!("Token已加入黑名单: 前缀={}, TTL={}s", &token[..8.min(token.len())], ttl);

        Ok(())
    }

    /// 将用户的Token版本号加入黑名单 (优化方案)
    pub async fn add_user_version_to_blacklist(&self, user_id: i64, token_version: i32, ttl: usize) -> AuthResult<()> {
        let key = format!("auth:blacklist:user:{}:v{}", user_id, token_version);
        let mut conn = self.redis_pool.get().await.map_err(|e| AuthError::InternalError(e.into()))?;

        redis::cmd("SETEX")
            .arg(&key)
            .arg(ttl)
            .arg("1")
            .query_async::<_, ()>(&mut *conn)
            .await?;

        debug!("用户Token版本已加入黑名单: user_id={}, version={}", user_id, token_version);

        Ok(())
    }

    /// 检查用户Token版本是否被撤销
    pub async fn is_user_version_revoked(&self, user_id: i64, token_version: i32) -> AuthResult<bool> {
        let key = format!("auth:blacklist:user:{}:v{}", user_id, token_version);
        let mut conn = self.redis_pool.get().await.map_err(|e| AuthError::InternalError(e.into()))?;

        let exists: usize = redis::cmd("EXISTS")
            .arg(&key)
            .query_async(&mut *conn)
            .await?;

        Ok(exists > 0)
    }

    /// 撤销单个Token (通过jti)
    pub async fn revoke_jti(&self, jti: &str, ttl: usize) -> AuthResult<()> {
        let key = format!("auth:blacklist:jti:{}", jti);
        let mut conn = self.redis_pool.get().await.map_err(|e| AuthError::InternalError(e.into()))?;

        redis::cmd("SETEX")
            .arg(&key)
            .arg(ttl)
            .arg("1")
            .query_async::<_, ()>(&mut *conn)
            .await?;

        debug!("JTI已撤销: jti={}", jti);

        Ok(())
    }

    /// 检查JTI是否被撤销
    pub async fn is_jti_revoked(&self, jti: &str) -> AuthResult<bool> {
        let key = format!("auth:blacklist:jti:{}", jti);
        let mut conn = self.redis_pool.get().await.map_err(|e| AuthError::InternalError(e.into()))?;

        let exists: usize = redis::cmd("EXISTS")
            .arg(&key)
            .query_async(&mut *conn)
            .await?;

        Ok(exists > 0)
    }

    /// 获取缓存统计信息
    pub async fn stats(&self) -> CacheStats {
        CacheStats {
            l1_size: self.local_cache.entry_count(),
            l1_capacity: self.local_cache.weighted_size(),
        }
    }
}

/// 缓存统计信息
#[derive(Debug, Serialize)]
pub struct CacheStats {
    pub l1_size: u64,
    pub l1_capacity: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cache_manager() {
        // 需要Redis服务才能运行
        let redis_config = RedisConfig {
            url: "redis://127.0.0.1:6379".to_string(),
            pool_max_size: Some(2),
            pool_min_idle: Some(1),
        };

        let cache = CacheManager::new(redis_config, 100, 60);

        let user_info = UserInfo {
            user_id: 123,
            username: "test".to_string(),
            email: "test@example.com".to_string(),
            tier: "Pro".to_string(),
            scopes: vec!["read".to_string(), "write".to_string()],
            balance: 1000,
            token_version: 1,
            status: "active".to_string(),
        };

        let token = "test_token_123";

        // 测试缓存写入
        cache.put_user(token, &user_info, 60).await.unwrap();

        // 测试缓存读取
        let cached = cache.get_user(token).await.unwrap();
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().user_id, 123);

        // 测试缓存删除
        cache.invalidate_token(token).await.unwrap();
        let cached = cache.get_user(token).await.unwrap();
        assert!(cached.is_none());
    }
}
