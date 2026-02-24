use crate::models::{AppError, Result};
use redis::Client as RedisClient;

#[derive(Clone)]
pub struct Cache {
    pub(crate) client: RedisClient,
}

impl Cache {
    pub fn get_client(&self) -> &RedisClient {
        &self.client
    }

    pub async fn new(redis_url: &str) -> Result<Self> {
        let client = RedisClient::open(redis_url)?;
        Ok(Cache { client })
    }

    // ==================== 余额操作 ====================

    pub async fn get_balance(&self, user_id: i64) -> Result<i64> {
        let key = format!("balance:{}", user_id);
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let balance: Option<i64> = redis::cmd("GET")
            .arg(&key)
            .query_async(&mut conn)
            .await?;
        balance.ok_or(AppError::UserNotFound)
    }

    pub async fn set_balance(&self, user_id: i64, balance: i64) -> Result<()> {
        let key = format!("balance:{}", user_id);
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        redis::cmd("SETEX")
            .arg(&key)
            .arg(3600)
            .arg(balance)
            .query_async::<_, ()>(&mut conn)
            .await?;
        Ok(())
    }

    pub async fn deduct_balance(&self, user_id: i64, amount: i64) -> Result<i64> {
        let key = format!("balance:{}", user_id);
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let new_balance: i64 = redis::cmd("DECRBY")
            .arg(&key)
            .arg(amount)
            .query_async(&mut conn)
            .await?;
        redis::cmd("EXPIRE")
            .arg(&key)
            .arg(3600)
            .query_async::<_, ()>(&mut conn)
            .await?;
        Ok(new_balance)
    }

    pub async fn add_balance(&self, user_id: i64, amount: i64) -> Result<i64> {
        let key = format!("balance:{}", user_id);
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let new_balance: i64 = redis::cmd("INCRBY")
            .arg(&key)
            .arg(amount)
            .query_async(&mut conn)
            .await?;
        redis::cmd("EXPIRE")
            .arg(&key)
            .arg(3600)
            .query_async::<_, ()>(&mut conn)
            .await?;
        Ok(new_balance)
    }

    // ==================== 限流操作 ====================

    pub async fn check_rate_limit(&self, user_id: i64, limit: usize) -> Result<bool> {
        let key = format!("rate:user:{}", user_id);
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let count: usize = redis::cmd("INCR")
            .arg(&key)
            .query_async(&mut conn)
            .await?;

        if count == 1 {
            redis::cmd("EXPIRE")
                .arg(&key)
                .arg(1)
                .query_async::<_, ()>(&mut conn)
                .await?;
        }

        Ok(count <= limit)
    }

    pub async fn is_concurrent_limited(&self, user_id: i64) -> Result<bool> {
        let key = format!("limit_concurrent:{}", user_id);
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let exists: bool = redis::cmd("EXISTS")
            .arg(&key)
            .query_async(&mut conn)
            .await?;
        Ok(exists)
    }

    pub async fn set_concurrent_limit(&self, user_id: i64) -> Result<()> {
        let key = format!("limit_concurrent:{}", user_id);
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        redis::cmd("SETEX")
            .arg(&key)
            .arg(300)
            .arg(1)
            .query_async::<_, ()>(&mut conn)
            .await?;
        Ok(())
    }
}
