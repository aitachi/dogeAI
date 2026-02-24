// ========== 缓存数据模型模块 ==========
use chrono::Utc;
use serde::{Deserialize, Serialize};

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
            total_hits as f64 / total_requests
        }
    }
}
