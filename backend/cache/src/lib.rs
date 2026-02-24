//! 两级缓存系统 (L1: LRU + L2: Redis)
//!
//! ## 架构
//! - L1: 本地内存 LRU 缓存，占用 < 200MB
//! - L2: Redis 缓存，maxmemory=256MB, eviction=volatile-lru
//! - TTL: 1-5 分钟可配置
//!
//! ## 性能目标
//! - L1 命中率 ≥ 70%
//! - L2 命中率 ≥ 90%

use atomic::Atomic;
use chrono::Utc;
use lru::LruCache;
use redis::aio::ConnectionManager;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tokio::sync::Mutex;

/// 缓存条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry<T> {
    pub value: T,
    pub cached_at: i64,
    pub ttl_secs: u64,
}

impl<T: Clone> CacheEntry<T> {
    pub fn new(value: T, ttl_secs: u64) -> Self {
        Self {
            value,
            cached_at: Utc::now().timestamp(),
            ttl_secs,
        }
    }

    pub fn is_expired(&self) -> bool {
        let age = Utc::now().timestamp() - self.cached_at;
        age as u64 > self.ttl_secs
    }

    pub fn remaining_ttl(&self) -> u64 {
        let age = (Utc::now().timestamp() - self.cached_at) as u64;
        if age >= self.ttl_secs {
            0
        } else {
            self.ttl_secs - age
        }
    }
}

/// 缓存统计
#[derive(Debug, Clone, Serialize)]
pub struct CacheStats {
    pub l1_hits: u64,
    pub l1_misses: u64,
    pub l2_hits: u64,
    pub l2_misses: u64,
    pub l1_size: usize,
    pub l1_capacity: usize,
    pub l2_keys_count: u64,
    pub l2_evicted_keys: u64,
    pub redis_memory_used: u64,
    pub redis_memory_max: u64,
}

impl CacheStats {
    pub fn l1_hit_rate(&self) -> f64 {
        let total = self.l1_hits + self.l1_misses;
        if total == 0 {
            0.0
        } else {
            self.l1_hits as f64 / total as f64
        }
    }

    pub fn l2_hit_rate(&self) -> f64 {
        let total = self.l2_hits + self.l2_misses;
        if total == 0 {
            0.0
        } else {
            self.l2_hits as f64 / total as f64
        }
    }

    pub fn overall_hit_rate(&self) -> f64 {
        let total_hits = self.l1_hits + self.l2_hits;
        let total_requests = total_hits + self.l2_misses;
        if total_requests == 0 {
            0.0
        } else {
            total_hits as f64 / total_requests as f64
        }
    }
}

/// 指标收集器 (使用原子操作)
#[derive(Default)]
pub struct Metrics {
    pub l1_hits: Atomic<u64>,
    pub l1_misses: Atomic<u64>,
    pub l2_hits: Atomic<u64>,
    pub l2_misses: Atomic<u64>,
}

impl Metrics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn inc_l1_hit(&self) {
        self.l1_hits.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_l1_miss(&self) {
        self.l1_misses.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_l2_hit(&self) {
        self.l2_hits.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_l2_miss(&self) {
        self.l2_misses.fetch_add(1, Ordering::Relaxed);
    }

    pub fn l1_hits(&self) -> u64 {
        self.l1_hits.load(Ordering::Relaxed)
    }

    pub fn l1_misses(&self) -> u64 {
        self.l1_misses.load(Ordering::Relaxed)
    }

    pub fn l2_hits(&self) -> u64 {
        self.l2_hits.load(Ordering::Relaxed)
    }

    pub fn l2_misses(&self) -> u64 {
        self.l2_misses.load(Ordering::Relaxed)
    }
}

/// 两级缓存配置
#[derive(Debug, Clone)]
pub struct TwoLevelCacheConfig {
    /// L1 缓存容量 (条目数)
    pub l1_capacity: usize,
    /// L1 默认 TTL (秒)
    pub l1_default_ttl: u64,
    /// L2 默认 TTL (秒)
    pub l2_default_ttl: u64,
    /// Redis 连接 URL
    pub redis_url: String,
    /// Redis key 前缀
    pub key_prefix: String,
    /// 是否启用指标收集
    pub enable_metrics: bool,
}

impl Default for TwoLevelCacheConfig {
    fn default() -> Self {
        Self {
            l1_capacity: 10000,
            l1_default_ttl: 60,   // 1 分钟
            l2_default_ttl: 300,  // 5 分钟
            redis_url: "redis://127.0.0.1:6379".to_string(),
            key_prefix: "cache2:".to_string(),
            enable_metrics: true,
        }
    }
}

/// 两级缓存管理器
pub struct TwoLevelCache<T>
where
    T: Clone + Serialize + for<'de> Deserialize<'de> + Send + Sync + 'static,
{
    l1_cache: Arc<Mutex<LruCache<String, CacheEntry<T>>>>,
    redis: Arc<ConnectionManager>,
    config: TwoLevelCacheConfig,
    metrics: Arc<Metrics>,
}

impl<T> TwoLevelCache<T>
where
    T: Clone + Serialize + for<'de> Deserialize<'de> + Send + Sync + 'static,
{
    /// 创建新的两级缓存
    pub async fn new(config: TwoLevelCacheConfig) -> Result<Self, CacheError> {
        // 连接 Redis
        let client = redis::Client::open(config.redis_url.clone())?;
        let redis = ConnectionManager::new(client).await?;

        // 配置 Redis
        let mut conn = redis.clone();
        // 设置 maxmemory=256MB
        let _: String = redis::cmd("CONFIG")
            .arg("SET")
            .arg("maxmemory")
            .arg("256mb")
            .query_async(&mut conn)
            .await?;
        // 设置淘汰策略为 volatile-lru
        let _: String = redis::cmd("CONFIG")
            .arg("SET")
            .arg("maxmemory-policy")
            .arg("volatile-lru")
            .query_async(&mut conn)
            .await?;

        let metrics = Arc::new(Metrics::new());

        let l1_cache = Arc::new(Mutex::new(LruCache::new(NonZeroUsize::new(
            config.l1_capacity,
        ).unwrap())));

        tracing::info!(
            "两级缓存初始化完成: L1容量={}, L1 TTL={}s, L2 TTL={}s",
            config.l1_capacity, config.l1_default_ttl, config.l2_default_ttl
        );

        Ok(Self {
            l1_cache,
            redis: Arc::new(redis),
            config,
            metrics,
        })
    }

    /// 获取缓存值
    pub async fn get(&self, key: &str) -> Result<Option<T>, CacheError> {
        // 1. 尝试 L1
        {
            let mut l1 = self.l1_cache.lock().await;
            if let Some(entry) = l1.get_mut(key) {
                if !entry.is_expired() {
                    if self.config.enable_metrics {
                        self.metrics.inc_l1_hit();
                    }
                    return Ok(Some(entry.value.clone()));
                }
                // 过期则移除
                l1.pop(key);
            }
        }
        if self.config.enable_metrics {
            self.metrics.inc_l1_miss();
        }

        // 2. 尝试 L2 (Redis)
        let redis_key = format!("{}{}", self.config.key_prefix, key);
        let mut conn = self.redis.as_ref().clone();

        let result: Option<String> = redis::cmd("GET")
            .arg(&redis_key)
            .query_async(&mut conn)
            .await?;

        if let Some(json) = result {
            if self.config.enable_metrics {
                self.metrics.inc_l2_hit();
            }

            let entry: CacheEntry<T> = serde_json::from_str(&json)?;

            if !entry.is_expired() {
                // 回写 L1
                let mut l1 = self.l1_cache.lock().await;
                l1.put(key.to_string(), entry.clone());
                return Ok(Some(entry.value));
            }
        }

        if self.config.enable_metrics {
            self.metrics.inc_l2_miss();
        }
        Ok(None)
    }

    /// 设置缓存值
    pub async fn set(&self, key: &str, value: T, ttl: Option<u64>) -> Result<(), CacheError> {
        let l1_ttl = ttl.unwrap_or(self.config.l1_default_ttl);
        let l2_ttl = ttl.unwrap_or(self.config.l2_default_ttl) as usize;

        let entry = CacheEntry::new(value.clone(), l1_ttl);
        let json = serde_json::to_string(&entry)?;

        // 写入 L1
        {
            let mut l1 = self.l1_cache.lock().await;
            l1.put(key.to_string(), entry);
        }

        // 写入 L2 (Redis)
        let redis_key = format!("{}{}", self.config.key_prefix, key);
        let mut conn = self.redis.as_ref().clone();

        redis::cmd("SETEX")
            .arg(&redis_key)
            .arg(l2_ttl)
            .arg(&json)
            .query_async::<()>(&mut conn)
            .await?;

        Ok(())
    }

    /// 删除缓存值
    pub async fn delete(&self, key: &str) -> Result<(), CacheError> {
        // 从 L1 删除
        {
            let mut l1 = self.l1_cache.lock().await;
            l1.pop(key);
        }

        // 从 L2 删除
        let redis_key = format!("{}{}", self.config.key_prefix, key);
        let mut conn = self.redis.as_ref().clone();
        redis::cmd("DEL")
            .arg(&redis_key)
            .query_async::<()>(&mut conn)
            .await?;

        Ok(())
    }

    /// 清空所有缓存
    pub async fn clear(&self) -> Result<(), CacheError> {
        // 清空 L1
        {
            let mut l1 = self.l1_cache.lock().await;
            l1.clear();
        }

        // 清空 L2 (按前缀)
        let pattern = format!("{}*", self.config.key_prefix);
        let mut conn = self.redis.as_ref().clone();

        let keys: Vec<String> = redis::cmd("KEYS")
            .arg(&pattern)
            .query_async(&mut conn)
            .await?;

        if !keys.is_empty() {
            redis::cmd("DEL")
                .arg(&keys)
                .query_async::<()>(&mut conn)
                .await?;
        }

        Ok(())
    }

    /// 获取缓存统计
    pub async fn stats(&self) -> Result<CacheStats, CacheError> {
        let l1 = self.l1_cache.lock().await;

        let mut conn = self.redis.as_ref().clone();

        // 获取 Redis 统计
        let info_str: String = redis::cmd("INFO")
            .arg("stats")
            .query_async(&mut conn)
            .await?;

        let l2_evicted_keys = parse_redis_info(&info_str, "evicted_keys");

        let memory_info: String = redis::cmd("INFO")
            .arg("memory")
            .query_async(&mut conn)
            .await?;

        let redis_memory_used = parse_redis_info(&memory_info, "used_memory") as u64;
        let redis_memory_max = parse_redis_info(&memory_info, "maxmemory") as u64;

        let keys_count: usize = redis::cmd("DBSIZE")
            .query_async(&mut conn)
            .await?;

        Ok(CacheStats {
            l1_hits: self.metrics.l1_hits(),
            l1_misses: self.metrics.l1_misses(),
            l2_hits: self.metrics.l2_hits(),
            l2_misses: self.metrics.l2_misses(),
            l1_size: l1.len(),
            l1_capacity: self.config.l1_capacity,
            l2_keys_count: keys_count as u64,
            l2_evicted_keys,
            redis_memory_used,
            redis_memory_max,
        })
    }

    /// 预热缓存 (批量设置)
    pub async fn warm_up(&self, entries: HashMap<String, T>) -> Result<(), CacheError>
    where
        T: Clone,
    {
        for (key, value) in entries {
            self.set(&key, value, None).await?;
        }
        Ok(())
    }

    /// 检查键是否存在
    pub async fn exists(&self, key: &str) -> Result<bool, CacheError> {
        Ok(self.get(key).await?.is_some())
    }

    /// 更新 TTL (touch)
    pub async fn touch(&self, key: &str, ttl: Option<u64>) -> Result<(), CacheError> {
        if let Some(value) = self.get(key).await? {
            self.set(key, value, ttl).await?;
        }
        Ok(())
    }

    /// 批量获取
    pub async fn get_multi(&self, keys: &[String]) -> Result<Vec<Option<T>>, CacheError> {
        let mut results = Vec::with_capacity(keys.len());
        for key in keys {
            results.push(self.get(key).await?);
        }
        Ok(results)
    }

    /// 批量设置
    pub async fn set_multi(&self, entries: HashMap<String, T>, _ttl: Option<u64>) -> Result<(), CacheError> {
        self.warm_up(entries).await
    }
}

/// 解析 Redis INFO 输出
fn parse_redis_info(info: &str, key: &str) -> u64 {
    info.lines()
        .find_map(|line| {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() == 2 && parts[0] == key {
                parts[1].parse().ok()
            } else {
                None
            }
        })
        .unwrap_or(0)
}

/// 缓存错误
#[derive(Debug, thiserror::Error)]
pub enum CacheError {
    #[error("Redis error: {0}")]
    Redis(#[from] redis::RedisError),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cache_entry_expiration() {
        let entry = CacheEntry::new("value".to_string(), 1); // 1秒 TTL
        assert!(!entry.is_expired());
        // 注意: 这个测试依赖实际时间，可能需要 mock
    }
}
