//! Token速度限制器
//!
//! 根据需求文档:
//! - Opus: 30 tokens/s
//! - Sonnet: 25 tokens/s
//! - Haiku: 20 tokens/s
//! - Codex: 20 tokens/s
//! - Gemini: 20 tokens/s

use crate::models::{AppError, Result};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::RwLock as StdRwLock;
use std::time::{Duration, Instant};

/// Token速度限制配置
#[derive(Debug, Clone, Copy)]
pub struct RateLimitConfig {
    pub tokens_per_second: u32,
    pub burst_size: u32,
}

/// Token速度限制器
pub struct TokenRateLimiter {
    configs: Arc<StdRwLock<HashMap<String, RateLimitConfig>>>,
    last_checked: Arc<StdRwLock<HashMap<String, Instant>>>,
    token_buckets: Arc<StdRwLock<HashMap<String, (u32, Instant)>>>, // (tokens, last_update)
}

impl TokenRateLimiter {
    pub fn new() -> Self {
        let mut configs = HashMap::new();

        // Opus/GLM-4-Plus: 30 tokens/s
        configs.insert("opus".to_string(), RateLimitConfig {
            tokens_per_second: 30,
            burst_size: 60,
        });
        configs.insert("glm-4-plus".to_string(), RateLimitConfig {
            tokens_per_second: 30,
            burst_size: 60,
        });

        // Sonnet/GLM-4-FlashX: 25 tokens/s
        configs.insert("sonnet".to_string(), RateLimitConfig {
            tokens_per_second: 25,
            burst_size: 50,
        });
        configs.insert("glm-4-flashx".to_string(), RateLimitConfig {
            tokens_per_second: 25,
            burst_size: 50,
        });

        // Haiku/GLM-4-Flash: 20 tokens/s
        configs.insert("haiku".to_string(), RateLimitConfig {
            tokens_per_second: 20,
            burst_size: 40,
        });
        configs.insert("glm-4-flash".to_string(), RateLimitConfig {
            tokens_per_second: 20,
            burst_size: 40,
        });

        // Codex/qwen-coder: 20 tokens/s
        configs.insert("codex".to_string(), RateLimitConfig {
            tokens_per_second: 20,
            burst_size: 40,
        });
        configs.insert("qwen-coder".to_string(), RateLimitConfig {
            tokens_per_second: 20,
            burst_size: 40,
        });

        // Gemini/qwen-turbo: 20 tokens/s
        configs.insert("gemini".to_string(), RateLimitConfig {
            tokens_per_second: 20,
            burst_size: 40,
        });
        configs.insert("qwen-turbo".to_string(), RateLimitConfig {
            tokens_per_second: 20,
            burst_size: 40,
        });

        Self {
            configs: Arc::new(StdRwLock::new(configs)),
            last_checked: Arc::new(StdRwLock::new(HashMap::new())),
            token_buckets: Arc::new(StdRwLock::new(HashMap::new())),
        }
    }

    /// 检查是否可以发送指定数量的token
    pub fn check_tokens(&self, model: &str, token_count: usize) -> Result<()> {
        let model_key = self.get_model_key(model);

        // 检查令牌桶
        {
            let buckets = self.token_buckets.read().map_err(|e| {
                AppError::InternalError(anyhow::anyhow!("RwLock poisoned: {}", e))
            })?;

            if let Some((tokens, _last_update)) = buckets.get(&model_key) {
                if *tokens as usize >= token_count {
                    return Ok(());
                }
            }
        }

        // 检查速率限制
        let config = self.get_config(&model_key)
            .ok_or_else(|| AppError::InternalError(anyhow::anyhow!("Model not found: {}", model_key)))?;

        // 计算应该有多少token
        let now = Instant::now();
        let last = {
            let last_checked = self.last_checked.read().map_err(|e| {
                AppError::InternalError(anyhow::anyhow!("RwLock poisoned: {}", e))
            })?;
            last_checked.get(&model_key).copied().unwrap_or(now)
        };

        let elapsed = now.duration_since(last).as_secs_f64();
        let new_tokens = (elapsed * config.tokens_per_second as f64) as u32;

        // 更新令牌桶
        {
            let mut buckets = self.token_buckets.write().map_err(|e| {
                AppError::InternalError(anyhow::anyhow!("RwLock poisoned: {}", e))
            })?;
            let mut last_checked = self.last_checked.write().map_err(|e| {
                AppError::InternalError(anyhow::anyhow!("RwLock poisoned: {}", e))
            })?;

            // 如果是首次使用，初始令牌为burst_size
            let default_bucket = &(config.burst_size, now);
            let (current_tokens, _) = buckets.get(&model_key).unwrap_or(default_bucket);
            let available = (*current_tokens + new_tokens).min(config.burst_size);

            if available < token_count as u32 {
                return Err(AppError::RateLimitExceeded);
            }

            buckets.insert(model_key.clone(), (available - token_count as u32, now));
            last_checked.insert(model_key.clone(), now);
        }

        Ok(())
    }

    /// 获取模型的token速度限制
    pub fn get_rate_limit(&self, model: &str) -> Option<u32> {
        self.get_config(model).map(|c| c.tokens_per_second)
    }

    /// 获取下一次可用token的等待时间（毫秒）
    pub fn wait_time_millis(&self, model: &str) -> u64 {
        let config = match self.get_config(model) {
            Some(c) => c,
            None => return 0,
        };

        // 简单计算：假设令牌桶为空时的等待时间
        let tokens_needed = 1u32;
        (tokens_needed * 1000 / config.tokens_per_second) as u64
    }

    /// 标准化模型名称
    fn get_model_key(&self, model: &str) -> String {
        model.to_lowercase().replace("glm-4-plus", "opus")
                                   .replace("glm-4-flashx", "sonnet")
                                   .replace("glm-4-flash", "haiku")
    }

    /// 获取模型配置
    fn get_config(&self, model: &str) -> Option<RateLimitConfig> {
        let model_key = self.get_model_key(model);

        let configs = self.configs.read().ok()?;
        configs.get(&model_key).copied()
    }

    /// 预留token槽位（阻塞直到可用）
    pub async fn reserve_tokens(&self, model: &str, token_count: usize) -> Result<()> {
        loop {
            match self.check_tokens(model, token_count) {
                Ok(_) => return Ok(()),
                Err(AppError::RateLimitExceeded) => {
                    // 等待一段时间后重试
                    let wait_ms = self.wait_time_millis(model);
                    if wait_ms > 0 {
                        tokio::time::sleep(Duration::from_millis(wait_ms)).await;
                    } else {
                        tokio::time::sleep(Duration::from_millis(10)).await;
                    }
                }
                Err(e) => return Err(e),
            }
        }
    }
}

impl Clone for TokenRateLimiter {
    fn clone(&self) -> Self {
        Self {
            configs: self.configs.clone(),
            last_checked: self.last_checked.clone(),
            token_buckets: self.token_buckets.clone(),
        }
    }
}

// ==================== 单元测试 ====================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiter_creation() {
        let limiter = TokenRateLimiter::new();
        assert_eq!(limiter.get_rate_limit("opus"), Some(30));
        assert_eq!(limiter.get_rate_limit("sonnet"), Some(25));
        assert_eq!(limiter.get_rate_limit("haiku"), Some(20));
        assert_eq!(limiter.get_rate_limit("codex"), Some(20));
        assert_eq!(limiter.get_rate_limit("gemini"), Some(20));
    }

    #[test]
    fn test_check_tokens_within_limit() {
        let limiter = TokenRateLimiter::new();
        // 首次检查应该在限制内
        assert!(limiter.check_tokens("opus", 10).is_ok());
        // 再次检查也应该通过（令牌桶会有余量）
        assert!(limiter.check_tokens("opus", 5).is_ok());
    }

    #[test]
    fn test_model_alias_resolution() {
        let limiter = TokenRateLimiter::new();
        assert_eq!(limiter.get_rate_limit("glm-4-plus"), Some(30));
        assert_eq!(limiter.get_rate_limit("glm-4-flashx"), Some(25));
        assert_eq!(limiter.get_rate_limit("glm-4-flash"), Some(20));
    }

    #[test]
    fn test_codex_model() {
        let limiter = TokenRateLimiter::new();
        assert_eq!(limiter.get_rate_limit("codex"), Some(20));
        assert_eq!(limiter.get_rate_limit("qwen-coder"), Some(20));
    }

    #[test]
    fn test_gemini_model() {
        let limiter = TokenRateLimiter::new();
        assert_eq!(limiter.get_rate_limit("gemini"), Some(20));
        assert_eq!(limiter.get_rate_limit("qwen-turbo"), Some(20));
    }

    #[test]
    fn test_unknown_model_returns_none() {
        let limiter = TokenRateLimiter::new();
        assert_eq!(limiter.get_rate_limit("unknown"), None);
    }
}
