use crate::models::{ApiError, RateLimitResult, UserTier};
use redis::AsyncCommands;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

/// Redis连接池（简化版本，使用 redis::aio::MultiplexedConnection）
#[derive(Clone)]
pub struct RedisPool {
    client: Arc<redis::Client>,
}

impl RedisPool {
    /// 创建新的Redis连接池
    pub async fn new(url: &str) -> Result<Self, ApiError> {
        let client = redis::Client::open(url).map_err(|e| ApiError::Redis(e.to_string()))?;
        // 测试连接
        let _conn = client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| ApiError::Redis(e.to_string()))?;

        Ok(Self {
            client: Arc::new(client),
        })
    }

    /// 获取客户端
    pub fn get_client(&self) -> Arc<redis::Client> {
        Arc::clone(&self.client)
    }

    /// 获取连接
    pub async fn get_connection(&self) -> Result<redis::aio::MultiplexedConnection, ApiError> {
        self.client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| ApiError::Redis(e.to_string()))
    }
}

/// 限流器
#[derive(Clone)]
pub struct RateLimiter {
    redis: RedisPool,
}

impl RateLimiter {
    pub fn new(redis: RedisPool) -> Self {
        Self { redis }
    }

    /// 检查用户是否允许请求（用户级限流）
    pub async fn allow_user(&self, user_id: i64, tier: UserTier) -> Result<RateLimitResult, ApiError> {
        let qps_limit = tier.qps_limit();
        let key = format!("rate:user:{}", user_id);

        let mut conn = self.redis.get_connection().await?;

        // 使用Redis INCR和EXPIRE实现滑动窗口
        let current: usize = conn.incr(&key, 1).await.map_err(|e| ApiError::Redis(e.to_string()))?;

        // 如果是第一次请求，设置TTL为1秒
        if current == 1 {
            let _: () = conn
                .expire(&key, 1)
                .await
                .map_err(|e| ApiError::Redis(e.to_string()))?;
        }

        if current <= qps_limit {
            Ok(RateLimitResult::Allowed)
        } else {
            // 计算重试时间
            let ttl: i64 = conn
                .ttl(&key)
                .await
                .map_err(|e| ApiError::Redis(e.to_string()))?;

            let retry_after = if ttl > 0 { ttl as u64 } else { 1 };
            Ok(RateLimitResult::Limited { retry_after })
        }
    }

    /// IP级限流
    pub async fn allow_ip(&self, ip: &str, max_qps: usize) -> Result<RateLimitResult, ApiError> {
        let key = format!("rate:ip:{}", ip);

        let mut conn = self.redis.get_connection().await?;

        let current: usize = conn.incr(&key, 1).await.map_err(|e| ApiError::Redis(e.to_string()))?;

        if current == 1 {
            let _: () = conn
                .expire(&key, 1)
                .await
                .map_err(|e| ApiError::Redis(e.to_string()))?;
        }

        if current <= max_qps {
            Ok(RateLimitResult::Allowed)
        } else {
            let ttl: i64 = conn
                .ttl(&key)
                .await
                .map_err(|e| ApiError::Redis(e.to_string()))?;

            let retry_after = if ttl > 0 { ttl as u64 } else { 1 };
            Ok(RateLimitResult::Limited { retry_after })
        }
    }

    /// 全局限流（使用原子计数器）
    pub async fn check_global_limit(
        &self,
        max_qps: usize,
    ) -> Result<RateLimitResult, ApiError> {
        let key = "rate:global";

        let mut conn = self.redis.get_connection().await?;

        let current: usize = conn.incr(key, 1).await.map_err(|e| ApiError::Redis(e.to_string()))?;

        if current == 1 {
            let _: () = conn
                .expire(key, 1)
                .await
                .map_err(|e| ApiError::Redis(e.to_string()))?;
        }

        if current <= max_qps {
            Ok(RateLimitResult::Allowed)
        } else {
            let ttl: i64 = conn.ttl(key).await.map_err(|e| ApiError::Redis(e.to_string()))?;

            let retry_after = if ttl > 0 { ttl as u64 } else { 1 };
            Ok(RateLimitResult::Limited { retry_after })
        }
    }
}

/// Token缓存（使用moka本地缓存 + Redis）
#[derive(Clone)]
pub struct TokenCache {
    local: Arc<moka::future::Cache<String, i64>>,
    redis: RedisPool,
    local_ttl: Duration,
}

impl TokenCache {
    pub fn new(redis: RedisPool, local_ttl_secs: u64) -> Self {
        let cache = moka::future::CacheBuilder::new(10_000)
            .time_to_live(Duration::from_secs(local_ttl_secs))
            .build();

        Self {
            local: Arc::new(cache),
            redis,
            local_ttl: Duration::from_secs(local_ttl_secs),
        }
    }

    /// 获取Token对应的用户ID
    pub async fn get_user_id(&self, token: &str) -> Result<Option<i64>, ApiError> {
        // 1. 先查本地缓存
        if let Some(user_id) = self.local.get(&token.to_string()).await {
            return Ok(Some(user_id));
        }

        // 2. 查Redis
        let key = format!("token:{}", token);
        let mut conn = self.redis.get_connection().await?;

        let user_id: Option<i64> = conn
            .get(&key)
            .await
            .map_err(|e| ApiError::Redis(e.to_string()))?;

        if let Some(uid) = user_id {
            // 3. 写入本地缓存
            self.local.insert(token.to_string(), uid).await;
            Ok(Some(uid))
        } else {
            Ok(None)
        }
    }

    /// 设置Token缓存
    pub async fn set_token(&self, token: &str, user_id: i64, ttl: Duration) -> Result<(), ApiError> {
        // 1. 写Redis
        let key = format!("token:{}", token);
        let ttl_secs = ttl.as_secs() as usize;

        let mut conn = self.redis.get_connection().await?;
        conn.set_ex::<_, _, ()>(&key, user_id, ttl_secs as u64)
            .await
            .map_err(|e| ApiError::Redis(e.to_string()))?;

        // 2. 写本地缓存
        self.local.insert(token.to_string(), user_id).await;

        Ok(())
    }

    /// 使Token失效
    pub async fn invalidate(&self, token: &str) -> Result<(), ApiError> {
        // 1. 删除本地缓存
        self.local.invalidate(&token.to_string()).await;

        // 2. 删除Redis缓存
        let key = format!("token:{}", token);
        let mut conn = self.redis.get_connection().await?;
        conn.del::<_, ()>(&key).await.map_err(|e| ApiError::Redis(e.to_string()))?;

        Ok(())
    }

    /// 批量预热本地缓存
    pub async fn warm_up(&self, tokens: Vec<(String, i64)>) -> Result<(), ApiError> {
        for (token, user_id) in tokens {
            self.local.insert(token, user_id).await;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limiter() {
        // 需要Redis实例
        // let redis = RedisPool::new("redis://127.0.0.1:6379").await.unwrap();
        // let limiter = RateLimiter::new(redis);
        // let result = limiter.allow_user(1, UserTier::Pro).await.unwrap();
        // assert_eq!(result, RateLimitResult::Allowed);
    }
}
