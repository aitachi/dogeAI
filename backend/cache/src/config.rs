// ========== 缓存配置模块 ==========
use serde::{Deserialize, Serialize};

/// 两级缓存配置
#[derive(Debug, Clone, Serialize, Deserialize)]
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
