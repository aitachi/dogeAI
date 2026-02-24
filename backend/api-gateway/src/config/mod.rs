use serde::{Deserialize, Serialize};
use std::path::Path;

/// 服务器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub port: u16,
    pub timeout_secs: u64,
    pub host: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            port: 8080,
            timeout_secs: 30,
            host: "0.0.0.0".to_string(),
        }
    }
}

/// 上游服务配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpstreamConfig {
    pub url: String,
    pub timeout_secs: u64,
    pub max_connections: usize,
    pub api_key: Option<String>,
}

impl Default for UpstreamConfig {
    fn default() -> Self {
        Self {
            url: "https://api.anthropic.com".to_string(),
            timeout_secs: 150,
            max_connections: 100,
            api_key: None,
        }
    }
}

/// 限流配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    pub user_qps_base: usize,
    pub user_qps_pro: usize,
    pub user_qps_max: usize,
    pub global_qps: usize,
    pub ip_qps: usize,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            user_qps_base: 10,
            user_qps_pro: 100,
            user_qps_max: 200,
            global_qps: 5000,
            ip_qps: 500,
        }
    }
}

/// 缓存配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    pub redis_url: String,
    pub ttl_secs: u64,
    pub local_ttl_secs: u64,
    pub max_connections: usize,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            redis_url: "redis://127.0.0.1:6379".to_string(),
            ttl_secs: 86400,
            local_ttl_secs: 3600,
            max_connections: 50,
        }
    }
}

/// 数据库配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub idle_connections: u32,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: "postgresql://postgres:password@localhost/api_gateway".to_string(),
            max_connections: 30,
            idle_connections: 10,
        }
    }
}

/// 日志配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogConfig {
    pub level: String,
    pub format: String,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            format: "json".to_string(),
        }
    }
}

/// 应用配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub upstream: UpstreamConfig,
    pub rate_limit: RateLimitConfig,
    pub cache: CacheConfig,
    pub database: DatabaseConfig,
    pub log: LogConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            upstream: UpstreamConfig::default(),
            rate_limit: RateLimitConfig::default(),
            cache: CacheConfig::default(),
            database: DatabaseConfig::default(),
            log: LogConfig::default(),
        }
    }
}

impl Config {
    /// 从TOML文件加载配置
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    /// 加载配置（优先从文件，否则使用默认配置）
    pub fn load() -> Self {
        // 尝试从多个位置加载配置文件
        let paths = [
            "./config.toml",
            "/etc/api-gateway/config.toml",
            "/root/api-gateway/config.toml",
        ];

        for path in &paths {
            if std::path::Path::new(path).exists() {
                if let Ok(config) = Self::from_file(path) {
                    tracing::info!("配置已从文件加载: {}", path);
                    return config;
                }
            }
        }

        tracing::warn!("未找到配置文件，使用默认配置");
        Self::default()
    }

    /// 保存配置到文件
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn std::error::Error>> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}
