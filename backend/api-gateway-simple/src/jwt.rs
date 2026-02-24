// ========== JWT认证模块 ==========
// 实现: JWT验证、密钥轮换、Token撤销、本地缓存

use chrono::{DateTime, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation, Algorithm};
use redis::aio::ConnectionManager;
use serde::{Deserialize, Serialize};
use sha2::Digest;
use std::sync::Arc;
use std::time::Duration;
use moka::future::Cache;
use secrecy::{ExposeSecret, Secret};
use sqlx::PgPool;
use tracing::{info, warn, debug};
use rand::Rng;

// ========== 配置常量 ==========

/// Token TTL: 15分钟 (900秒)
/// 满足要求: token ttl <= 15min
pub const TOKEN_TTL_SECS: i64 = 900;

/// 密钥轮换窗口: 24小时
/// 满足要求: 密钥轮换窗口 <= 24h
pub const KEY_ROTATION_HOURS: i64 = 24;

/// JWT验证延迟目标: <5ms
pub const MAX_VERIFY_LATENCY_MS: u64 = 5;

/// Redis黑名单TTL: 与Token TTL相同
pub const BLACKLIST_TTL_SECS: usize = 900;

/// 本地缓存TTL: 1分钟
pub const LOCAL_CACHE_TTL_SECS: u64 = 60;

/// 本地缓存最大容量
pub const LOCAL_CACHE_CAPACITY: u64 = 10000;

// ========== JWT Claims 结构 ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    // 标准声明
    pub sub: String,        // subject: user_id
    pub exp: i64,           // expiry time (timestamp)
    pub iat: i64,           // issued at
    pub iss: String,        // issuer
    pub nbf: i64,           // not before

    // 自定义声明
    pub user_id: String,
    pub username: String,
    pub tier: String,       // Base/Pro/Max/AMax/Enterprise
    pub scopes: Vec<String>,

    // Token版本管理 (用于撤销)
    pub token_version: i32,
    pub jti: String,        // JWT ID (唯一标识)
}

impl Claims {
    /// 检查token是否过期
    pub fn is_expired(&self) -> bool {
        Utc::now().timestamp() > self.exp
    }

    /// 检查token是否在有效期内
    pub fn is_valid_time(&self) -> bool {
        let now = Utc::now().timestamp();
        now >= self.nbf && now <= self.exp
    }

    /// 获取剩余有效时间(秒)
    pub fn remaining_secs(&self) -> i64 {
        (self.exp - Utc::now().timestamp()).max(0)
    }
}

// ========== 密钥管理 ==========

/// 密钥轮换管理器
pub struct KeyManager {
    /// 当前活跃密钥 (primary key)
    current_key: Secret<String>,
    /// 上一个密钥 (grace period期间仍可验证)
    previous_key: Option<Secret<String>>,
    /// 密钥版本号
    key_version: i32,
    /// 最后轮换时间
    last_rotation: DateTime<Utc>,
}

impl KeyManager {
    /// 从环境变量加载密钥
    pub fn from_env() -> anyhow::Result<Self> {
        let secret = std::env::var("JWT_SECRET")
            .or_else(|_| std::env::var("JWT_SECRET_FILE").map(|path| {
                std::fs::read_to_string(path.trim()).unwrap_or_default()
            }))
            .unwrap_or_else(|_| {
                warn!("使用随机生成的JWT密钥，生产环境必须设置JWT_SECRET环境变量");
                Self::generate_secret()
            });

        Ok(Self {
            current_key: Secret::new(secret.clone()),
            previous_key: None,
            key_version: 1,
            last_rotation: Utc::now(),
        })
    }

    /// 生成随机密钥
    pub fn generate_secret() -> String {
        const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
        let mut rng = rand::thread_rng();
        (0..64)
            .map(|_| {
                let idx = rng.gen_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect()
    }

    /// 获取当前编码密钥
    pub fn get_current_key(&self) -> &[u8] {
        self.current_key.expose_secret().as_bytes()
    }

    /// 获取当前解码密钥
    pub fn get_decoding_key(&self) -> DecodingKey {
        DecodingKey::from_secret(self.current_key.expose_secret().as_bytes())
    }

    /// 获取上一个解码密钥 (用于grace period验证)
    pub fn get_previous_decoding_key(&self) -> Option<DecodingKey> {
        self.previous_key.as_ref().map(|key| {
            DecodingKey::from_secret(key.expose_secret().as_bytes())
        })
    }

    /// 执行密钥轮换
    pub fn rotate(&mut self) -> anyhow::Result<()> {
        let new_secret = Self::generate_secret();
        let old_key = self.current_key.expose_secret().clone();

        self.previous_key = Some(Secret::new(old_key));
        self.current_key = Secret::new(new_secret);
        self.key_version += 1;
        self.last_rotation = Utc::now();

        info!("JWT密钥已轮换: 新版本={}", self.key_version);

        Ok(())
    }

    /// 检查是否需要轮换
    pub fn should_rotate(&self) -> bool {
        let elapsed = Utc::now().signed_duration_since(self.last_rotation);
        elapsed.num_hours() >= KEY_ROTATION_HOURS
    }

    /// 获取当前密钥版本
    pub fn version(&self) -> i32 {
        self.key_version
    }
}

// ========== Token黑名单 ==========

/// Token黑名单 (使用Redis)
#[derive(Clone)]
pub struct TokenBlacklist {
    redis: ConnectionManager,
}

impl TokenBlacklist {
    pub fn new(redis: ConnectionManager) -> Self {
        Self { redis }
    }

    /// 添加token到黑名单
    pub async fn add(&self, jti: &str, ttl: usize) -> anyhow::Result<()> {
        let mut conn = self.redis.clone();
        let key = format!("blacklist:{}", jti);

        redis::cmd("SETEX")
            .arg(&key)
            .arg(ttl)
            .arg("1")
            .query_async::<_, ()>(&mut conn)
            .await?;

        debug!("Token已加入黑名单: jti={}, ttl={}s", jti, ttl);

        Ok(())
    }

    /// 检查token是否在黑名单中
    pub async fn contains(&self, jti: &str) -> bool {
        let mut conn = self.redis.clone();
        let key = format!("blacklist:{}", jti);

        let result: Option<String> = redis::cmd("GET")
            .arg(&key)
            .query_async(&mut conn)
            .await
            .unwrap_or(None);

        result.is_some()
    }

    /// 撤销用户的所有token (通过增加token版本)
    pub async fn revoke_user_tokens(&self, user_id: &str, version: i32) -> anyhow::Result<()> {
        let mut conn = self.redis.clone();
        let key = format!("user_version:{}", user_id);

        // 记录用户的token版本，验证时检查
        redis::cmd("SETEX")
            .arg(&key)
            .arg(BLACKLIST_TTL_SECS)
            .arg(version)
            .query_async::<_, ()>(&mut conn)
            .await?;

        debug!("用户Token已撤销: user_id={}, version={}", user_id, version);

        Ok(())
    }

    /// 获取用户的当前token版本
    pub async fn get_user_version(&self, user_id: &str) -> Option<i32> {
        let mut conn = self.redis.clone();
        let key = format!("user_version:{}", user_id);

        let result: Option<String> = redis::cmd("GET")
            .arg(&key)
            .query_async(&mut conn)
            .await
            .unwrap_or(None);

        result.and_then(|s| s.parse().ok())
    }
}

// ========== 认证错误 ==========

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("Token无效: {0}")]
    InvalidToken(String),

    #[error("Token已过期")]
    TokenExpired,

    #[error("Token已被撤销")]
    TokenRevoked,

    #[error("用户不存在: {0}")]
    UserNotFound(String),

    #[error("用户已被禁用")]
    UserSuspended,

    #[error("权限不足")]
    Forbidden,

    #[error("数据库错误: {0}")]
    Database(String),

    #[error("Redis错误: {0}")]
    Redis(String),

    #[error("内部错误: {0}")]
    Internal(String),
}

impl From<jsonwebtoken::errors::Error> for AuthError {
    fn from(err: jsonwebtoken::errors::Error) -> Self {
        match err.kind() {
            jsonwebtoken::errors::ErrorKind::ExpiredSignature => AuthError::TokenExpired,
            _ => AuthError::InvalidToken(err.to_string()),
        }
    }
}

impl From<sqlx::error::Error> for AuthError {
    fn from(err: sqlx::error::Error) -> Self {
        AuthError::Database(err.to_string())
    }
}

impl From<redis::RedisError> for AuthError {
    fn from(err: redis::RedisError) -> Self {
        AuthError::Redis(err.to_string())
    }
}

impl From<anyhow::Error> for AuthError {
    fn from(err: anyhow::Error) -> Self {
        AuthError::Internal(err.to_string())
    }
}

// 实现 axum IntoResponse 以便可以在 handler 中直接返回
impl axum::response::IntoResponse for AuthError {
    fn into_response(self) -> axum::response::Response {
        let (status, error_type, message) = match self {
            AuthError::InvalidToken(msg) => (axum::http::StatusCode::UNAUTHORIZED, "authentication_error", msg),
            AuthError::TokenExpired => (axum::http::StatusCode::UNAUTHORIZED, "authentication_error", "Token已过期".to_string()),
            AuthError::TokenRevoked => (axum::http::StatusCode::UNAUTHORIZED, "authentication_error", "Token已被撤销".to_string()),
            AuthError::UserNotFound(msg) => (axum::http::StatusCode::NOT_FOUND, "not_found", msg),
            AuthError::UserSuspended => (axum::http::StatusCode::FORBIDDEN, "forbidden", "用户已被禁用".to_string()),
            AuthError::Forbidden => (axum::http::StatusCode::FORBIDDEN, "forbidden", "权限不足".to_string()),
            AuthError::Database(msg) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "internal_error", msg),
            AuthError::Redis(msg) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "internal_error", msg),
            AuthError::Internal(msg) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "internal_error", msg),
        };

        let body = axum::Json(serde_json::json!({
            "error_type": error_type,
            "message": message
        }));

        (status, body).into_response()
    }
}

// ========== 用户信息 ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub user_id: String,
    pub username: String,
    pub tier: String,
    pub token_version: i32,
    pub scopes: Vec<String>,
}

// ========== JWT认证服务 ==========

/// JWT认证服务
pub struct JwtAuthService {
    /// PostgreSQL连接池
    db: PgPool,

    /// Redis连接管理器
    redis: ConnectionManager,

    /// 密钥管理器
    key_manager: Arc<std::sync::RwLock<KeyManager>>,

    /// L1: 本地内存缓存 (moka)
    /// 键: token哈希, 值: UserInfo
    local_cache: Cache<String, UserInfo>,

    /// Token黑名单
    blacklist: TokenBlacklist,

    /// JWT算法
    algorithm: Algorithm,
}

impl JwtAuthService {
    /// 创建新的认证服务
    pub async fn new(
        db: PgPool,
        redis_url: &str,
    ) -> anyhow::Result<Self> {
        // 连接Redis
        let client = redis::Client::open(redis_url)?;
        let conn = ConnectionManager::new(client).await?;

        // 创建密钥管理器
        let key_manager = Arc::new(std::sync::RwLock::new(
            KeyManager::from_env()?
        ));

        // 创建本地缓存
        let local_cache = Cache::builder()
            .max_capacity(LOCAL_CACHE_CAPACITY)
            .time_to_live(Duration::from_secs(LOCAL_CACHE_TTL_SECS))
            .build();

        let blacklist = TokenBlacklist::new(conn.clone());

        info!("JWT认证服务初始化完成");

        Ok(Self {
            db,
            redis: conn,
            key_manager,
            local_cache,
            blacklist,
            algorithm: Algorithm::HS256,
        })
    }

    /// 从Redis连接创建服务
    pub async fn from_parts(
        db: PgPool,
        redis: ConnectionManager,
    ) -> anyhow::Result<Self> {
        let key_manager = Arc::new(std::sync::RwLock::new(
            KeyManager::from_env()?
        ));

        let local_cache = Cache::builder()
            .max_capacity(LOCAL_CACHE_CAPACITY)
            .time_to_live(Duration::from_secs(LOCAL_CACHE_TTL_SECS))
            .build();

        let blacklist = TokenBlacklist::new(redis.clone());

        Ok(Self {
            db,
            redis,
            key_manager,
            local_cache,
            blacklist,
            algorithm: Algorithm::HS256,
        })
    }

    /// 生成JWT Token
    pub fn create_token(
        &self,
        user_id: &str,
        username: &str,
        tier: &str,
        scopes: Vec<String>,
        token_version: i32,
    ) -> anyhow::Result<String> {
        let now = Utc::now();
        let jti = uuid::Uuid::new_v4().to_string();

        let claims = Claims {
            sub: user_id.to_string(),
            exp: now.timestamp() + TOKEN_TTL_SECS,
            iat: now.timestamp(),
            iss: "dogeai-gateway".to_string(),
            nbf: now.timestamp(),
            user_id: user_id.to_string(),
            username: username.to_string(),
            tier: tier.to_string(),
            scopes,
            token_version,
            jti: jti.clone(),
        };

        let key_manager = self.key_manager.read().unwrap();
        let encoding_key = EncodingKey::from_secret(key_manager.get_current_key());

        let token = encode(&Header::default(), &claims, &encoding_key)?;

        debug!("Token已生成: user_id={}, jti={}", user_id, jti);

        Ok(token)
    }

    /// 验证JWT Token
    pub async fn verify_token(&self, token: &str) -> anyhow::Result<AuthenticatedUser> {
        let start = std::time::Instant::now();

        // 计算token哈希用于本地缓存
        let token_hash = format!("{:x}", sha2::Sha256::digest(token.as_bytes()));

        // L1: 检查本地缓存
        if let Some(user_info) = self.local_cache.get(&token_hash).await {
            debug!("Token验证命中本地缓存: latency={}ms", start.elapsed().as_millis());
            return Ok(AuthenticatedUser {
                user_id: user_info.user_id.clone(),
                username: user_info.username.clone(),
                tier: user_info.tier.clone(),
                token_version: user_info.token_version,
                balance: 0,  // 从缓存获取时没有balance
                scopes: user_info.scopes.clone(),
            });
        }

        // 解码token - 必须在await之前完成以避免Send问题
        let decoding_key = {
            let key_manager = self.key_manager.read().unwrap();
            key_manager.get_decoding_key()
        };

        let token_data = decode::<Claims>(
            token,
            &decoding_key,
            &Validation::new(self.algorithm),
        )?;

        let claims = token_data.claims;

        // 检查是否过期
        if claims.is_expired() {
            return Err(AuthError::TokenExpired.into());
        }

        // 检查黑名单 (jti)
        if self.blacklist.contains(&claims.jti).await {
            return Err(AuthError::TokenRevoked.into());
        }

        // 检查用户token版本
        if let Some(stored_version) = self.blacklist.get_user_version(&claims.user_id).await {
            if stored_version > claims.token_version {
                return Err(AuthError::TokenRevoked.into());
            }
        }

        // 从数据库获取用户信息 (包括balance)
        let balance: Option<i64> = sqlx::query_scalar("SELECT balance FROM users WHERE id = $1")
            .bind(&claims.user_id)
            .fetch_optional(&self.db)
            .await
            .unwrap_or(None);

        let balance = balance.unwrap_or(0);

        // 缓存验证结果
        let user_info = UserInfo {
            user_id: claims.user_id.clone(),
            username: claims.username.clone(),
            tier: claims.tier.clone(),
            token_version: claims.token_version,
            scopes: claims.scopes.clone(),
        };

        self.local_cache.insert(token_hash, user_info.clone()).await;

        let elapsed = start.elapsed();
        if elapsed.as_millis() > MAX_VERIFY_LATENCY_MS as u128 {
            warn!("Token验证延迟过高: {}ms", elapsed.as_millis());
        } else {
            debug!("Token验证完成: latency={}ms", elapsed.as_millis());
        }

        Ok(AuthenticatedUser {
            user_id: user_info.user_id,
            username: user_info.username,
            tier: user_info.tier,
            token_version: user_info.token_version,
            balance,
            scopes: user_info.scopes,
        })
    }

    /// 验证Token并返回完整信息
    pub async fn verify_token_full(&self, token: &str) -> anyhow::Result<(String, Claims, i32)> {
        let key_manager = self.key_manager.read().unwrap();
        let token_data = decode::<Claims>(
            token,
            &key_manager.get_decoding_key(),
            &Validation::new(self.algorithm),
        )?;

        let claims = token_data.claims;
        let token_version = claims.token_version;

        Ok((
            token.to_string(),
            claims,
            token_version,
        ))
    }

    /// 撤销单个token
    pub async fn revoke_token(&self, jti: &str) -> anyhow::Result<()> {
        self.blacklist.add(jti, BLACKLIST_TTL_SECS).await
    }

    /// 撤销用户的所有token
    pub async fn revoke_user_tokens(&self, user_id: &str) -> anyhow::Result<()> {
        // 获取用户当前的token版本并+1
        let version: Option<i32> = sqlx::query_scalar(
            "SELECT token_version FROM users WHERE id = $1"
        )
        .bind(user_id)
        .fetch_optional(&self.db)
        .await?
        .map(|v: i64| v as i32);

        let current_version = version.unwrap_or(0);
        let new_version = current_version + 1;

        // 更新数据库中的版本
        sqlx::query(
            "UPDATE users SET token_version = $2 WHERE id = $1"
        )
        .bind(user_id)
        .bind(new_version)
        .execute(&self.db)
        .await?;

        // 在Redis中记录版本变化
        self.blacklist.revoke_user_tokens(user_id, new_version).await?;

        // 清除本地缓存
        self.local_cache.invalidate_all();

        info!("用户所有Token已撤销: user_id={}, new_version={}", user_id, new_version);

        Ok(())
    }

    /// 检查token是否被撤销
    pub async fn is_token_revoked(&self, jti: &str) -> bool {
        self.blacklist.contains(jti).await
    }

    /// 获取用户信息
    pub async fn get_user_info(&self, user_id: &str) -> anyhow::Result<Option<UserInfo>> {
        let row = sqlx::query_as::<_, (String, String, String, i32)>(
            "SELECT id, username, tier, token_version FROM users WHERE id = $1"
        )
        .bind(user_id)
        .fetch_optional(&self.db)
        .await?;

        Ok(row.map(|(id, username, tier, token_version)| UserInfo {
            user_id: id,
            username,
            tier,
            token_version,
            scopes: vec![],
        }))
    }

    /// 验证用户凭据
    pub async fn verify_credentials(
        &self,
        account: &str,
        password: &str,
    ) -> anyhow::Result<Option<UserInfo>> {
        use sha2::{Digest, Sha256};
        let password_hash = format!("{:x}", Sha256::digest(password.as_bytes()));

        let row = sqlx::query_as::<_, (String, String, String, i32)>(
            "SELECT id, username, tier, token_version FROM users WHERE (username = $1 OR email = $1) AND password_hash = $2"
        )
        .bind(account)
        .bind(password_hash)
        .fetch_optional(&self.db)
        .await?;

        Ok(row.map(|(id, username, tier, token_version)| UserInfo {
            user_id: id,
            username,
            tier,
            token_version,
            scopes: vec![],
        }))
    }

    /// 获取缓存统计
    pub async fn get_cache_stats(&self) -> CacheStats {
        let size = self.local_cache.entry_count();
        CacheStats {
            local_size: size,
            local_hits: 0,
            local_misses: 0,
            redis_size: 0,
            hit_count: 0,
            miss_count: 0,
            hit_rate: 0.0,
            size,
        }
    }

    /// 获取缓存统计 (同步版本)
    pub fn cache_stats(&self) -> CacheStats {
        let size = self.local_cache.entry_count();
        CacheStats {
            local_size: size,
            local_hits: 0,
            local_misses: 0,
            redis_size: 0,
            hit_count: 0,
            miss_count: 0,
            hit_rate: 0.0,
            size,
        }
    }

    /// 获取当前密钥版本
    pub fn key_version(&self) -> i32 {
        let key_manager = self.key_manager.read().unwrap();
        key_manager.version()
    }

    /// 解析Token (不验证)
    pub fn parse_claims(&self, token: &str) -> anyhow::Result<Claims> {
        let key_manager = self.key_manager.read().unwrap();
        let token_data = decode::<Claims>(
            token,
            &key_manager.get_decoding_key(),
            &Validation::new(self.algorithm),
        )?;
        Ok(token_data.claims)
    }

    /// 刷新Token
    pub fn refresh_token(&self, token: &str) -> anyhow::Result<Option<String>> {
        let claims = self.parse_claims(token)?;

        // 检查是否需要刷新
        let remaining = claims.remaining_secs();
        if remaining > TOKEN_TTL_SECS / 2 {
            // 还有足够时间，不需要刷新
            return Ok(None);
        }

        // 生成新token
        Ok(Some(self.create_token(
            &claims.user_id,
            &claims.username,
            &claims.tier,
            claims.scopes,
            claims.token_version,
        )?))
    }

    /// 撤销token并清除缓存
    pub async fn revoke_token_with_cache(&self, jti: &str, token_hash: &str) -> anyhow::Result<()> {
        self.blacklist.add(jti, BLACKLIST_TTL_SECS).await?;
        // 清除本地缓存中对应的条目
        let _ = self.local_cache.invalidate(token_hash).await;
        Ok(())
    }

    /// 撤销用户所有token (使用新版本)
    pub async fn revoke_user_all(&self, user_id: &str, old_version: i32) -> anyhow::Result<()> {
        let new_version = old_version + 1;

        // 更新数据库中的版本
        sqlx::query(
            "UPDATE users SET token_version = $2 WHERE id = $1"
        )
        .bind(user_id)
        .bind(new_version)
        .execute(&self.db)
        .await?;

        // 在Redis中记录版本变化
        self.blacklist.revoke_user_tokens(user_id, new_version).await?;

        // 清除本地缓存
        self.local_cache.invalidate_all();

        info!("用户所有Token已撤销: user_id={}, new_version={}", user_id, new_version);

        Ok(())
    }

    /// 检查是否需要密钥轮换
    pub async fn needs_rotation(&self) -> bool {
        let key_manager = self.key_manager.read().unwrap();
        key_manager.should_rotate()
    }

    /// 执行密钥轮换
    pub async fn rotate_keys(&self) -> anyhow::Result<()> {
        let mut key_manager = self.key_manager.write().unwrap();
        key_manager.rotate()?;
        Ok(())
    }

    /// 密钥轮换 (同步版本别名)
    pub fn needs_rotation_sync(&self) -> bool {
        let key_manager = self.key_manager.read().unwrap();
        key_manager.should_rotate()
    }
}

// ========== AuthenticatedUser (用于提取器) ==========

#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    pub user_id: String,
    pub username: String,
    pub tier: String,
    pub token_version: i32,
    pub balance: i64,
    pub scopes: Vec<String>,
}

// ========== 缓存统计 ==========

#[derive(Debug, Clone)]
pub struct CacheStats {
    pub local_size: u64,
    pub local_hits: u64,
    pub local_misses: u64,
    pub redis_size: u64,
    pub hit_count: u64,
    pub miss_count: u64,
    pub hit_rate: f64,
    pub size: u64,
}
