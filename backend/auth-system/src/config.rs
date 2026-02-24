//! 认证系统配置

use serde::Deserialize;
use std::time::Duration;

#[derive(Debug, Deserialize, Clone)]
pub struct AuthConfig {
    pub jwt_secret: String,
    pub issuer: String,
    pub token_ttl_secs: Option<i64>,
    pub refresh_window_secs: Option<i64>,
    pub cache: Option<CacheConfig>,
    pub blacklist: Option<BlacklistConfig>,
    pub rate_limit: Option<RateLimitConfig>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CacheConfig {
    #[serde(default = "default_local_ttl_secs")]
    pub local_ttl_secs: u64,

    #[serde(default = "default_local_capacity")]
    pub local_capacity: u64,

    #[serde(default = "default_redis_ttl_secs")]
    pub redis_ttl_secs: usize,
}

#[derive(Debug, Deserialize, Clone)]
pub struct BlacklistConfig {
    #[serde(default = "default_blacklist_enabled")]
    pub enabled: bool,

    #[serde(default = "default_check_first")]
    pub check_first: bool,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RateLimitConfig {
    #[serde(default = "default_base_qps")]
    pub base_qps: usize,

    #[serde(default = "default_pro_qps")]
    pub pro_qps: usize,

    #[serde(default = "default_max_qps")]
    pub max_qps: usize,

    #[serde(default = "default_amax_qps")]
    pub amax_qps: usize,
}

// 默认值函数
fn default_local_ttl_secs() -> u64 { 3600 }
fn default_local_capacity() -> u64 { 1000 }
fn default_redis_ttl_secs() -> usize { 86400 }
fn default_blacklist_enabled() -> bool { true }
fn default_check_first() -> bool { true }
fn default_base_qps() -> usize { 10 }
fn default_pro_qps() -> usize { 100 }
fn default_max_qps() -> usize { 200 }
fn default_amax_qps() -> usize { 300 }

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            local_ttl_secs: default_local_ttl_secs(),
            local_capacity: default_local_capacity(),
            redis_ttl_secs: default_redis_ttl_secs(),
        }
    }
}

impl Default for BlacklistConfig {
    fn default() -> Self {
        Self {
            enabled: default_blacklist_enabled(),
            check_first: default_check_first(),
        }
    }
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            base_qps: default_base_qps(),
            pro_qps: default_pro_qps(),
            max_qps: default_max_qps(),
            amax_qps: default_amax_qps(),
        }
    }
}

impl AuthConfig {
    /// 从环境变量加载配置
    pub fn from_env() -> Result<Self, config::ConfigError> {
        let mut settings = config::Config::builder();

        // 设置默认值
        // token_ttl_secs: 900秒 (15分钟) - 满足安全要求 token ttl ≤ 15min
        // refresh_window_secs: 600秒 (10分钟) - 在过期前10分钟内允许刷新
        settings = settings
            .set_default("issuer", "aitachi-auth")?
            .set_default("token_ttl_secs", 900)?
            .set_default("refresh_window_secs", 600)?
            .set_default("cache.local_ttl_secs", 3600)?
            .set_default("cache.local_capacity", 1000)?
            .set_default("cache.redis_ttl_secs", 86400)?
            .set_default("blacklist.enabled", true)?
            .set_default("blacklist.check_first", true)?
            .set_default("rate_limit.base_qps", 10)?
            .set_default("rate_limit.pro_qps", 100)?
            .set_default("rate_limit.max_qps", 200)?
            .set_default("rate_limit.amax_qps", 300)?;

        // 从环境变量覆盖
        if let Ok(secret) = std::env::var("JWT_SECRET") {
            settings = settings.set_override("jwt_secret", secret)?;
        } else {
            // 开发环境使用默认密钥
            settings = settings.set_override("jwt_secret", "dev-secret-key-change-in-production")?;
        }

        if let Ok(issuer) = std::env::var("JWT_ISSUER") {
            settings = settings.set_override("issuer", issuer)?;
        }

        if let Ok(ttl) = std::env::var("TOKEN_TTL_SECS") {
            if let Ok(ttl_val) = ttl.parse::<i64>() {
                settings = settings.set_override("token_ttl_secs", ttl_val)?;
            }
        }

        settings.build()?.try_deserialize()
    }

    /// 从TOML文件加载配置
    pub fn from_file(path: &str) -> Result<Self, config::ConfigError> {
        let settings = config::Config::builder()
            .add_source(config::File::with_name(path))
            .build()?;

        // 处理环境变量替换 (如 ${JWT_SECRET})
        let mut config: AuthConfig = settings.try_deserialize()?;

        // 替换密钥中的环境变量引用
        if config.jwt_secret.starts_with("${") && config.jwt_secret.ends_with("}") {
            let env_var = &config.jwt_secret[2..config.jwt_secret.len()-1];
            if let Ok(value) = std::env::var(env_var) {
                config.jwt_secret = value;
            }
        }

        Ok(config)
    }

    pub fn token_ttl(&self) -> Duration {
        Duration::from_secs(self.token_ttl_secs.unwrap_or(900) as u64)
    }

    pub fn refresh_window(&self) -> Duration {
        Duration::from_secs(self.refresh_window_secs.unwrap_or(600) as u64)
    }

    pub fn cache(&self) -> CacheConfig {
        self.cache.clone().unwrap_or_default()
    }

    pub fn blacklist(&self) -> BlacklistConfig {
        self.blacklist.clone().unwrap_or_default()
    }

    pub fn rate_limit(&self) -> RateLimitConfig {
        self.rate_limit.clone().unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_from_env() {
        std::env::set_var("JWT_SECRET", "test-secret");
        let config = AuthConfig::from_env().unwrap();
        assert_eq!(config.jwt_secret, "test-secret");
        std::env::remove_var("JWT_SECRET");
    }

    #[test]
    fn test_defaults() {
        let config = AuthConfig::from_env().unwrap();
        assert_eq!(config.issuer, "aitachi-auth");
        assert_eq!(config.token_ttl_secs, Some(86400));
    }
}
