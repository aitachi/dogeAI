// ========== 配置模块 ==========
use std::net::SocketAddr;
use std::time::Duration;

/// 缓存策略配置
#[derive(Clone, Debug)]
pub struct CacheStrategy {
    /// Token验证缓存 TTL (秒)
    pub token_cache_ttl: u64,
    /// 用户信息缓存 TTL (秒)
    pub user_info_cache_ttl: u64,
    /// 模型池配置缓存 TTL (秒)
    pub model_pool_cache_ttl: u64,
    /// 提供商健康检查缓存 TTL (秒)
    pub provider_health_cache_ttl: u64,
}

impl Default for CacheStrategy {
    fn default() -> Self {
        Self {
            // Token缓存: 60秒 (短期缓存，频繁更新)
            token_cache_ttl: 60,
            // 用户信息缓存: 1小时 (中期缓存)
            user_info_cache_ttl: 3600,
            // 模型池配置: 24小时 (长期缓存，配置不常变)
            model_pool_cache_ttl: 86400,
            // 提供商健康状态: 5分钟 (较短期缓存)
            provider_health_cache_ttl: 300,
        }
    }
}

impl CacheStrategy {
    /// 从环境变量加载缓存策略
    pub fn from_env() -> Self {
        Self {
            token_cache_ttl: std::env::var("CACHE_TOKEN_TTL")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(60),
            user_info_cache_ttl: std::env::var("CACHE_USER_INFO_TTL")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(3600),
            model_pool_cache_ttl: std::env::var("CACHE_MODEL_POOL_TTL")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(86400),
            provider_health_cache_ttl: std::env::var("CACHE_PROVIDER_HEALTH_TTL")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(300),
        }
    }

    /// 获取Token缓存Duration
    pub fn token_cache_duration(&self) -> Duration {
        Duration::from_secs(self.token_cache_ttl)
    }

    /// 获取用户信息缓存Duration
    pub fn user_info_cache_duration(&self) -> Duration {
        Duration::from_secs(self.user_info_cache_ttl)
    }

    /// 获取模型池缓存Duration
    pub fn model_pool_cache_duration(&self) -> Duration {
        Duration::from_secs(self.model_pool_cache_ttl)
    }
}

/// JWT 配置
#[derive(Clone, Debug)]
pub struct JwtConfig {
    /// JWT 密钥
    pub secret: String,
    /// Token TTL (秒)
    pub token_ttl_secs: i64,
    /// 密钥轮换窗口 (小时)
    pub key_rotation_hours: i64,
    /// 本地缓存容量
    pub local_cache_capacity: u64,
}

impl Default for JwtConfig {
    fn default() -> Self {
        Self {
            secret: "dev-secret-key-change-in-production-32bytes-min".to_string(),
            token_ttl_secs: 900,      // 15分钟
            key_rotation_hours: 24,   // 24小时
            local_cache_capacity: 10000,
        }
    }
}

impl JwtConfig {
    pub fn from_env() -> Self {
        let secret = std::env::var("JWT_SECRET")
            .or_else(|_| std::env::var("JWT_SECRET_FILE").map(|path| {
                std::fs::read_to_string(path.trim()).unwrap_or_default()
            }))
            .unwrap_or_else(|| {
                tracing::warn!("使用默认JWT密钥，生产环境必须设置JWT_SECRET环境变量");
                "dev-secret-key-change-in-production-32bytes-min".to_string()
            });

        Self {
            secret,
            token_ttl_secs: std::env::var("JWT_TOKEN_TTL")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(900), // 默认15分钟
            key_rotation_hours: std::env::var("JWT_KEY_ROTATION_HOURS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(24), // 默认24小时
            local_cache_capacity: std::env::var("JWT_CACHE_CAPACITY")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(10000),
        }
    }
}

/// 数据库配置
#[derive(Clone, Debug)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub acquire_timeout_secs: u64,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: "postgresql://postgres:postgres@localhost/api_gateway".to_string(),
            max_connections: 10,
            min_connections: 1,
            acquire_timeout_secs: 30,
        }
    }
}

impl DatabaseConfig {
    pub fn from_env() -> Self {
        Self {
            url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgresql://postgres:postgres@localhost/api_gateway".to_string()),
            max_connections: std::env::var("DB_MAX_CONNECTIONS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(10),
            min_connections: std::env::var("DB_MIN_CONNECTIONS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(1),
            acquire_timeout_secs: std::env::var("DB_ACQUIRE_TIMEOUT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(30),
        }
    }
}

/// Redis 配置
#[derive(Clone, Debug)]
pub struct RedisConfig {
    pub url: String,
    pub pool_size: u32,
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            url: "redis://127.0.0.1:6379".to_string(),
            pool_size: 10,
        }
    }
}

impl RedisConfig {
    pub fn from_env() -> Self {
        Self {
            url: std::env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string()),
            pool_size: std::env::var("REDIS_POOL_SIZE")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(10),
        }
    }
}

/// 上游 API 配置
#[derive(Clone, Debug)]
pub struct UpstreamConfig {
    pub url: String,
    pub api_key: String,
    pub timeout_secs: u64,
    pub max_retries: u32,
}

impl Default for UpstreamConfig {
    fn default() -> Self {
        Self {
            url: "https://api.anthropic.com".to_string(),
            api_key: String::new(),
            timeout_secs: 30,
            max_retries: 3,
        }
    }
}

impl UpstreamConfig {
    pub fn from_env() -> Self {
        Self {
            url: std::env::var("UPSTREAM_API_URL")
                .unwrap_or_else(|_| "https://api.anthropic.com".to_string()),
            api_key: std::env::var("UPSTREAM_API_KEY")
                .unwrap_or_default(),
            timeout_secs: std::env::var("UPSTREAM_TIMEOUT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(30),
            max_retries: std::env::var("UPSTREAM_MAX_RETRIES")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(3),
        }
    }
}

/// 服务器配置
#[derive(Clone, Debug)]
pub struct ServerConfig {
    pub addr: SocketAddr,
    pub workers: Option<usize>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            addr: SocketAddr::from(([0, 0, 0, 0], 8081)),
            workers: None,
        }
    }
}

impl ServerConfig {
    pub fn from_env() -> Self {
        Self {
            addr: std::env::var("SERVER_ADDR")
                .unwrap_or_else(|_| "0.0.0.0:8081".to_string())
                .parse()
                .unwrap_or_else(|_| SocketAddr::from(([0, 0, 0, 0], 8081))),
            workers: std::env::var("SERVER_WORKERS")
                .ok()
                .and_then(|s| s.parse().ok()),
        }
    }
}

/// 统一配置结构
#[derive(Clone, Debug)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub jwt: JwtConfig,
    pub upstream: UpstreamConfig,
    pub cache: CacheStrategy,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            database: DatabaseConfig::default(),
            redis: RedisConfig::default(),
            jwt: JwtConfig::default(),
            upstream: UpstreamConfig::default(),
            cache: CacheStrategy::default(),
        }
    }
}

impl Config {
    /// 从环境变量加载配置
    pub fn from_env() -> Self {
        Self {
            server: ServerConfig::from_env(),
            database: DatabaseConfig::from_env(),
            redis: RedisConfig::from_env(),
            jwt: JwtConfig::from_env(),
            upstream: UpstreamConfig::from_env(),
            cache: CacheStrategy::from_env(),
        }
    }

    /// 验证配置是否有效
    pub fn validate(&self) -> Result<(), String> {
        if self.jwt.secret.len() < 32 {
            return Err("JWT_SECRET 长度必须至少32字节".to_string());
        }

        if self.database.url.is_empty() {
            return Err("DATABASE_URL 不能为空".to_string());
        }

        if self.redis.url.is_empty() {
            return Err("REDIS_URL 不能为空".to_string());
        }

        Ok(())
    }
}

// ========== 向后兼容的旧配置结构 ==========

#[deprecated(note = "请使用新的 Config 结构")]
#[derive(Clone)]
pub struct OldConfig {
    pub database_url: String,
    pub redis_url: String,
    pub server_addr: SocketAddr,
    pub jwt_secret: String,
    pub upstream_api_url: String,
    pub upstream_api_key: String,
}

impl From<Config> for OldConfig {
    fn from(config: Config) -> Self {
        Self {
            database_url: config.database.url,
            redis_url: config.redis.url,
            server_addr: config.server.addr,
            jwt_secret: config.jwt.secret,
            upstream_api_url: config.upstream.url,
            upstream_api_key: config.upstream.api_key,
        }
    }
}

impl OldConfig {
    pub fn from_env() -> Self {
        Config::from_env().into()
    }
}
