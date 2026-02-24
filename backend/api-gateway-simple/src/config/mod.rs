// ========== 配置模块 ==========
use std::net::SocketAddr;

#[derive(Clone)]
pub struct Config {
    pub database_url: String,
    pub redis_url: String,
    pub server_addr: SocketAddr,
    pub jwt_secret: String,
    pub upstream_api_url: String,
    pub upstream_api_key: String,
}

impl Config {
    pub fn from_env() -> Self {
        let jwt_secret = std::env::var("JWT_SECRET")
            .or_else(|_| std::env::var("JWT_SECRET_FILE")
                .map(|path| std::fs::read_to_string(path.trim())
                    .unwrap_or_default()
                    .trim().to_string()))
            .unwrap_or_else(|_| {
                tracing::warn!("使用默认JWT密钥，生产环境必须设置JWT_SECRET环境变量");
                "dev-secret-key-change-in-production-32bytes-min".to_string()
            });

        Config {
            database_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgresql://postgres:postgres@localhost/api_gateway".to_string()),
            redis_url: std::env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string()),
            server_addr: std::env::var("SERVER_ADDR")
                .unwrap_or_else(|_| "0.0.0.0:8081".to_string())
                .parse()
                .unwrap_or_else(|_| SocketAddr::from(([0, 0, 0, 0], 8081))),
            jwt_secret,
            upstream_api_url: std::env::var("UPSTREAM_API_URL")
                .unwrap_or_else(|_| "https://api.anthropic.com".to_string()),
            upstream_api_key: std::env::var("UPSTREAM_API_KEY")
                .unwrap_or_else(|_| "".to_string()),
        }
    }
}
