// ========== JWT认证模块 ==========
// 实现: JWT验证、密钥轮换、Token撤销、本地缓存

use chrono::{DateTime, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation, Algorithm};
use redis::aio::ConnectionManager;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use moka::future::Cache;
use secrecy::{ExposeSecret, Secret};
use sqlx::PgPool;
use tracing::{info, warn, debug};

// ========== 配置常量 ==========

/// Token TTL: 15分钟 (900秒)
/// 满足要求: token ttl ≤ 15min
pub const TOKEN_TTL_SECS: i64 = 900;

/// 密钥轮换窗口: 24小时
/// 满足要求: 密钥轮换窗口 ≤ 24h
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
    current_key: String,
    /// 上一个密钥 (grace period期间仍可验证)
    previous_key: Option<String>,
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
            current_key: secret,
            previous_key: None,
            key_version: 1,
            last_rotation: Utc::now(),
        })
    }

    /// 生成随机密钥 (256位)
    fn generate_secret() -> String {
        use rand::Rng;
        const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
        let mut rng = rand::thread_rng();
        (0..64)
            .map(|_| {
                let idx = rng.gen_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect()
    }

    /// 从配置文件加载密钥 (受限文件管理)
    pub fn from_file(path: &str) -> anyhow::Result<Self> {
        let secret = std::fs::read_to_string(path)?;
        let secret = secret.trim().to_string();

        if secret.len() < 32 {
            anyhow::bail!("JWT密钥长度必须至少32字符");
        }

        Ok(Self {
            current_key: secret,
            previous_key: None,
            key_version: 1,
            last_rotation: Utc::now(),
        })
    }

    /// 密钥轮换 (每24小时调用一次)
    pub fn rotate(&mut self) -> bool {
        let now = Utc::now();
        let hours_since_rotation = (now - self.last_rotation).num_hours();

        if hours_since_rotation >= KEY_ROTATION_HOURS {
            info!("执行JWT密钥轮换，当前版本: {}", self.key_version);

            // 保存旧密钥作为previous_key
            self.previous_key = Some(self.current_key.clone());

            // 生成新密钥
            let new_secret = Self::generate_secret();
            self.current_key = new_secret;

            self.key_version += 1;
            self.last_rotation = now;

            info!("密钥轮换完成，新版本: {}", self.key_version);
            return true;
        }

        false
    }

    /// 检查是否需要轮换
    pub fn needs_rotation(&self) -> bool {
        (Utc::now() - self.last_rotation).num_hours() >= KEY_ROTATION_HOURS
    }

    /// 获取当前编码密钥
    pub fn encoding_key(&self) -> EncodingKey {
        EncodingKey::from_secret(self.current_key.as_bytes())
    }

    /// 获取解码密钥 (尝试当前和上一个密钥)
    pub fn decoding_keys(&self) -> Vec<DecodingKey> {
        let mut keys = vec![
            DecodingKey::from_secret(self.current_key.as_bytes())
        ];

        if let Some(ref prev) = self.previous_key {
            keys.push(DecodingKey::from_secret(prev.as_bytes()));
        }

        keys
    }

    /// 获取密钥版本
    pub fn version(&self) -> i32 {
        self.key_version
    }
}

// ========== 黑名单管理 ==========

/// Token黑名单 (Redis实现)
/// 用于强制下线token
pub struct TokenBlacklist {
    redis: ConnectionManager,
}

impl TokenBlacklist {
    pub fn new(redis: ConnectionManager) -> Self {
        Self { redis }
    }

    /// 将token加入黑名单
    pub async fn add(&self, jti: &str, ttl_secs: usize) -> anyhow::Result<()> {
        let key = format!("auth:blacklist:jti:{}", jti);
        let mut conn = self.redis.clone();

        redis::cmd("SETEX")
            .arg(&key)
            .arg(ttl_secs)
            .arg("1")
            .query_async::<_, ()>(&mut conn)
            .await?;

        debug!("Token {} 已加入黑名单，TTL: {}秒", jti, ttl_secs);
        Ok(())
    }

    /// 批量将用户所有token加入黑名单
    pub async fn revoke_user_all(
        &self,
        user_id: &str,
        token_version: i32,
        ttl_secs: usize,
    ) -> anyhow::Result<()> {
        let key = format!("auth:blacklist:user:{}", user_id);
        let mut conn = self.redis.clone();

        redis::cmd("SETEX")
            .arg(&key)
            .arg(ttl_secs)
            .arg(token_version.to_string())
            .query_async::<_, ()>(&mut conn)
            .await?;

        info!("用户 {} 所有token已撤销，版本 > {}", user_id, token_version);
        Ok(())
    }

    /// 检查token是否在黑名单中
    pub async fn is_revoked(&self, jti: &str) -> anyhow::Result<bool> {
        let key = format!("auth:blacklist:jti:{}", jti);
        let mut conn = self.redis.clone();

        let exists: usize = redis::cmd("EXISTS")
            .arg(&key)
            .query_async(&mut conn)
            .await?;

        Ok(exists > 0)
    }

    /// 检查用户token版本
    pub async fn get_user_version(&self, user_id: &str) -> anyhow::Result<Option<i32>> {
        let key = format!("auth:blacklist:user:{}", user_id);
        let mut conn = self.redis.clone();

        let version: Option<String> = redis::cmd("GET")
            .arg(&key)
            .query_async(&mut conn)
            .await?;

        Ok(version.and_then(|v| v.parse().ok()))
    }
}

// ========== 认证服务 ==========

/// 用户信息 (缓存结构)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub user_id: String,
    pub username: String,
    pub tier: String,
    pub scopes: Vec<String>,
    pub balance: i64,
    pub token_version: i32,
}

/// 认证统计指标
/// 用于跟踪拒绝/通过率
#[derive(Debug, Default)]
pub struct AuthMetrics {
    /// 总验证次数
    pub total_verifications: AtomicU64,
    /// 成功验证次数
    pub successful_verifications: AtomicU64,
    /// 拒绝验证次数
    pub failed_verifications: AtomicU64,
    /// 按原因分类的拒绝计数
    pub rejected_by_expired: AtomicU64,
    pub rejected_by_revoked: AtomicU64,
    pub rejected_by_invalid: AtomicU64,
    pub rejected_by_user_not_found: AtomicU64,
}

use std::sync::atomic::{AtomicU64, Ordering};

impl AuthMetrics {
    /// 记录成功验证
    pub fn record_success(&self) {
        self.total_verifications.fetch_add(1, Ordering::Relaxed);
        self.successful_verifications.fetch_add(1, Ordering::Relaxed);
    }

    /// 记录失败验证
    pub fn record_failure(&self, reason: &AuthError) {
        self.total_verifications.fetch_add(1, Ordering::Relaxed);
        self.failed_verifications.fetch_add(1, Ordering::Relaxed);

        match reason {
            AuthError::TokenExpired => {
                self.rejected_by_expired.fetch_add(1, Ordering::Relaxed);
            }
            AuthError::TokenRevoked => {
                self.rejected_by_revoked.fetch_add(1, Ordering::Relaxed);
            }
            AuthError::InvalidToken(_) => {
                self.rejected_by_invalid.fetch_add(1, Ordering::Relaxed);
            }
            AuthError::UserNotFound(_) => {
                self.rejected_by_user_not_found.fetch_add(1, Ordering::Relaxed);
            }
            _ => {}
        }
    }

    /// 获取通过率 (0.0 - 1.0)
    pub fn pass_rate(&self) -> f64 {
        let total = self.total_verifications.load(Ordering::Relaxed);
        if total == 0 {
            return 1.0;
        }
        let success = self.successful_verifications.load(Ordering::Relaxed);
        success as f64 / total as f64
    }

    /// 获取拒绝率 (0.0 - 1.0)
    pub fn rejection_rate(&self) -> f64 {
        1.0 - self.pass_rate()
    }

    /// 重置统计
    pub fn reset(&self) {
        self.total_verifications.store(0, Ordering::Relaxed);
        self.successful_verifications.store(0, Ordering::Relaxed);
        self.failed_verifications.store(0, Ordering::Relaxed);
        self.rejected_by_expired.store(0, Ordering::Relaxed);
        self.rejected_by_revoked.store(0, Ordering::Relaxed);
        self.rejected_by_invalid.store(0, Ordering::Relaxed);
        self.rejected_by_user_not_found.store(0, Ordering::Relaxed);
    }

    /// 获取统计报告
    pub fn report(&self) -> MetricsReport {
        MetricsReport {
            total_verifications: self.total_verifications.load(Ordering::Relaxed),
            successful_verifications: self.successful_verifications.load(Ordering::Relaxed),
            failed_verifications: self.failed_verifications.load(Ordering::Relaxed),
            rejected_by_expired: self.rejected_by_expired.load(Ordering::Relaxed),
            rejected_by_revoked: self.rejected_by_revoked.load(Ordering::Relaxed),
            rejected_by_invalid: self.rejected_by_invalid.load(Ordering::Relaxed),
            rejected_by_user_not_found: self.rejected_by_user_not_found.load(Ordering::Relaxed),
            pass_rate: self.pass_rate(),
            rejection_rate: self.rejection_rate(),
        }
    }
}

/// 统计报告
#[derive(Debug, Clone, serde::Serialize)]
pub struct MetricsReport {
    pub total_verifications: u64,
    pub successful_verifications: u64,
    pub failed_verifications: u64,
    pub rejected_by_expired: u64,
    pub rejected_by_revoked: u64,
    pub rejected_by_invalid: u64,
    pub rejected_by_user_not_found: u64,
    pub pass_rate: f64,
    pub rejection_rate: f64,
}

/// JWT认证服务
/// 实现本地JWT验证 + Redis黑名单
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

    /// 认证统计指标
    metrics: Arc<AuthMetrics>,
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

        Ok(Self {
            db,
            redis: conn,
            key_manager,
            local_cache,
            blacklist,
            algorithm: Algorithm::HS256,
            metrics: Arc::new(AuthMetrics::default()),
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
            metrics: Arc::new(AuthMetrics::default()),
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
            iss: "api-gateway".to_string(),
            nbf: now.timestamp(),
            user_id: user_id.to_string(),
            username: username.to_string(),
            tier: tier.to_string(),
            scopes,
            token_version,
            jti: jti.clone(),
        };

        let key_manager = self.key_manager.read().unwrap();
        let token = encode(
            &Header::default(),
            &claims,
            &key_manager.encoding_key(),
        )?;

        info!("为用户 {} 生成新token，JTI: {}, 有效期: {}秒",
              user_id, jti, TOKEN_TTL_SECS);

        Ok(token)
    }

    /// 刷新Token (在续期窗口内)
    pub fn refresh_token(&self, old_token: &str) -> anyhow::Result<Option<String>> {
        let claims = self.parse_claims(old_token)?;

        let remaining = claims.remaining_secs();

        // 如果剩余时间 < 5分钟，自动刷新
        if remaining < 300 && remaining > 0 {
            debug!("Token {} 剩余{}秒，执行刷新", claims.jti, remaining);
            return Ok(Some(self.create_token(
                &claims.user_id,
                &claims.username,
                &claims.tier,
                claims.scopes.clone(),
                claims.token_version,
            )?));
        }

        Ok(None)
    }

    /// 验证短 API Key (sk-xxxxxx 格式)
    async fn verify_api_key(&self, api_key: &str) -> Result<UserInfo, AuthError> {
        // 检查是否是 API Key 格式 (sk- 开头，支持任何长度)
        // 旧格式: sk-XXXXXXXXXXXXX
        // Anthropic 格式: sk-ant-api03-XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX
        if api_key.starts_with("sk-") {
            // 从数据库查找 API Key
            let user = sqlx::query_as::<_, (String, String, String, i64, i32)>(
                "SELECT u.user_id, u.username, u.tier, u.balance, u.token_version
                 FROM users u
                 INNER JOIN user_api_keys k ON u.user_id = k.user_id
                 WHERE k.api_key = $1 AND k.is_active = true AND u.active = true"
            )
            .bind(api_key)
            .fetch_optional(&self.db)
            .await
            .map_err(|e| AuthError::Database(e.to_string()))?
            .ok_or_else(|| AuthError::InvalidToken("无效的API Key".to_string()))?;

            // 更新最后使用时间
            let _ = sqlx::query("UPDATE user_api_keys SET created_at = NOW() WHERE api_key = $1")
                .bind(api_key)
                .execute(&self.db)
                .await;

            return Ok(UserInfo {
                user_id: user.0,
                username: user.1,
                tier: user.2,
                balance: user.3,
                token_version: user.4,
                scopes: vec!["read".to_string(), "write".to_string()],
            });
        }
        Err(AuthError::InvalidToken("无效的API Key格式".to_string()))
    }

    /// 验证Token (本地JWT验证 或 API Key)
    /// 满足要求: 本地JWT验证, JWT验证延迟 <5ms
    pub async fn verify_token(&self, token: &str) -> Result<UserInfo, AuthError> {
        let start = std::time::Instant::now();

        // 0. 首先检查是否是 API Key (sk- 或 sk-ant- 开头)
        // Anthropic 格式: sk-ant-api03-XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX (65字符)
        // 旧格式: sk-XXXXXXXXXXXXX (短格式)
        if token.starts_with("sk-") {
            match self.verify_api_key(token).await {
                Ok(user) => {
                    self.metrics.record_success();
                    return Ok(user);
                }
                Err(_) => {
                    // API Key 验证失败，继续尝试 JWT 验证
                }
            }
        }

        // 1. 计算token哈希用于缓存键
        let token_hash = self.hash_token(token);

        // 2. L1本地缓存检查 (<0.1ms)
        if let Some(user) = self.local_cache.get(&token_hash).await {
            debug!("Token命中L1缓存，延迟: {:?}", start.elapsed());
            self.metrics.record_success();
            return Ok(user);
        }

        // 3. 解析JWT (<1ms)
        let claims = match self.parse_claims(token) {
            Ok(c) => c,
            Err(e) => {
                self.metrics.record_failure(&e);
                return Err(e);
            }
        };

        // 4. 检查token是否过期
        if claims.is_expired() {
            self.metrics.record_failure(&AuthError::TokenExpired);
            return Err(AuthError::TokenExpired);
        }

        // 5. 检查黑名单 (Redis)
        if self.blacklist.is_revoked(&claims.jti).await.unwrap_or(false) {
            self.metrics.record_failure(&AuthError::TokenRevoked);
            return Err(AuthError::TokenRevoked);
        }

        // 6. 检查用户token版本
        if let Some(revoked_version) = self.blacklist.get_user_version(&claims.user_id).await.unwrap_or(None) {
            if claims.token_version <= revoked_version {
                self.metrics.record_failure(&AuthError::TokenRevoked);
                return Err(AuthError::TokenRevoked);
            }
        }

        // 7. 获取用户信息
        let user = match self.get_user_info(&claims.user_id).await {
            Ok(u) => u,
            Err(e) => {
                self.metrics.record_failure(&e);
                return Err(e);
            }
        };

        // 8. 缓存到L1
        self.local_cache.insert(token_hash, user.clone()).await;

        let latency = start.elapsed();
        debug!("Token验证完成，延迟: {:?}", latency);

        // 检查延迟要求
        if latency.as_millis() > MAX_VERIFY_LATENCY_MS as u128 {
            warn!("JWT验证延迟超过目标: {:?} > {}ms", latency, MAX_VERIFY_LATENCY_MS);
        }

        // 记录成功验证
        self.metrics.record_success();

        Ok(user)
    }

    /// 解析JWT Claims (公开方法供main.rs使用)
    pub fn parse_claims(&self, token: &str) -> Result<Claims, AuthError> {
        // 尝试用所有密钥解码
        let key_manager = self.key_manager.read().unwrap();
        let keys = key_manager.decoding_keys();

        let mut last_err = None;

        for key in keys {
            let validation = Validation::new(self.algorithm);
            match decode::<Claims>(token, &key, &validation) {
                Ok(data) => return Ok(data.claims),
                Err(e) => last_err = Some(e),
            }
        }

        Err(AuthError::InvalidToken(format!(
            "无法解析JWT: {:?}",
            last_err
        )))
    }

    /// 获取用户信息 (从数据库或Redis)
    async fn get_user_info(&self, user_id: &str) -> Result<UserInfo, AuthError> {
        // 先从Redis获取
        let redis_key = format!("user:info:{}", user_id);
        let mut conn = self.redis.clone();

        if let Ok(Some(cached)) = redis::cmd("GET")
            .arg(&redis_key)
            .query_async::<_, Option<String>>(&mut conn)
            .await
        {
            if let Ok(user) = serde_json::from_str::<UserInfo>(&cached) {
                return Ok(user);
            }
        }

        // 从数据库获取
        let user = sqlx::query_as::<_, (String, String, String, i64, i32)>(
            "SELECT user_id, username, tier, balance, token_version
             FROM users WHERE user_id = $1 AND active = true"
        )
        .bind(user_id)
        .fetch_optional(&self.db)
        .await
        .map_err(|e| AuthError::Database(e.to_string()))?
        .ok_or_else(|| AuthError::UserNotFound(user_id.to_string()))?;

        let user_info = UserInfo {
            user_id: user.0,
            username: user.1,
            tier: user.2,
            balance: user.3,
            token_version: user.4,
            scopes: vec!["read".to_string(), "write".to_string()], // 默认权限
        };

        // 缓存到Redis
        if let Ok(json) = serde_json::to_string(&user_info) {
            let _ = redis::cmd("SETEX")
                .arg(&redis_key)
                .arg(3600) // 1小时
                .arg(json)
                .query_async::<_, ()>(&mut conn)
                .await;
        }

        Ok(user_info)
    }

    /// 撤销单个token (同时清除L1缓存)
    pub async fn revoke_token_by_hash(&self, token_hash: &str) -> anyhow::Result<()> {
        // 从L1缓存中删除
        self.local_cache.invalidate(token_hash).await;
        Ok(())
    }

    /// 撤销单个token (通过jti加入黑名单)
    pub async fn revoke_token(&self, jti: &str) -> anyhow::Result<()> {
        self.blacklist.add(jti, BLACKLIST_TTL_SECS).await
    }

    /// 撤销单个token并清除L1缓存
    pub async fn revoke_token_with_cache(&self, jti: &str, token_hash: &str) -> anyhow::Result<()> {
        // 加入Redis黑名单
        self.blacklist.add(jti, BLACKLIST_TTL_SECS).await?;
        // 清除L1缓存
        self.local_cache.invalidate(token_hash).await;
        Ok(())
    }

    /// 撤销用户所有token
    pub async fn revoke_user_all(&self, user_id: &str, token_version: i32) -> anyhow::Result<()> {
        self.blacklist.revoke_user_all(user_id, token_version, BLACKLIST_TTL_SECS).await?;

        // 清除所有本地缓存 (因为无法知道哪些缓存属于该用户)
        self.local_cache.invalidate_all();

        Ok(())
    }

    /// 密钥轮换
    pub fn rotate_keys(&self) -> bool {
        let mut key_manager = self.key_manager.write().unwrap();
        key_manager.rotate()
    }

    /// 检查是否需要轮换密钥
    pub fn needs_rotation(&self) -> bool {
        let key_manager = self.key_manager.read().unwrap();
        key_manager.needs_rotation()
    }

    /// 获取当前密钥版本
    pub fn key_version(&self) -> i32 {
        let key_manager = self.key_manager.read().unwrap();
        key_manager.version()
    }

    /// 计算token哈希 (用于缓存键)
    fn hash_token(&self, token: &str) -> String {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(token.as_bytes());
        format!("{:x}", hasher.finalize())[..32].to_string()
    }

    /// 获取缓存统计
    pub fn cache_stats(&self) -> CacheStats {
        CacheStats {
            size: self.local_cache.entry_count(),
            hit_count: 0,
            miss_count: 0,
            hit_rate: 0.0,
        }
    }

    /// 获取认证指标 (拒绝/通过率统计)
    pub fn auth_metrics(&self) -> MetricsReport {
        self.metrics.report()
    }

    /// 重置认证统计
    pub fn reset_metrics(&self) {
        self.metrics.reset()
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

impl axum::response::IntoResponse for AuthError {
    fn into_response(self) -> axum::response::Response {
        use axum::http::StatusCode;
        use axum::Json;

        let (status, error_type, message) = match self {
            AuthError::InvalidToken(msg) => {
                (StatusCode::UNAUTHORIZED, "invalid_token", msg)
            }
            AuthError::TokenExpired => {
                (StatusCode::UNAUTHORIZED, "token_expired", "Token已过期".to_string())
            }
            AuthError::TokenRevoked => {
                (StatusCode::UNAUTHORIZED, "token_revoked", "Token已被撤销".to_string())
            }
            AuthError::UserNotFound(_) => {
                (StatusCode::UNAUTHORIZED, "user_not_found", "用户不存在".to_string())
            }
            AuthError::UserSuspended => {
                (StatusCode::FORBIDDEN, "user_suspended", "用户已被禁用".to_string())
            }
            AuthError::Forbidden => {
                (StatusCode::FORBIDDEN, "forbidden", "权限不足".to_string())
            }
            AuthError::Database(msg) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "database_error", msg)
            }
            AuthError::Redis(msg) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "redis_error", msg)
            }
            AuthError::Internal(msg) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "internal_error", msg)
            }
        };

        let body = Json(serde_json::json!({
            "error": error_type,
            "message": message,
            "timestamp": Utc::now().to_rfc3339(),
        }));

        (status, body).into_response()
    }
}

// ========== 缓存统计 ==========

#[derive(Debug, Clone, serde::Serialize)]
pub struct CacheStats {
    pub size: u64,
    pub hit_count: u64,
    pub miss_count: u64,
    pub hit_rate: f64,
}

// ========== Axum集成 ==========

/// 已认证的用户提取器
#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    pub user_id: String,
    pub username: String,
    pub tier: String,
    pub scopes: Vec<String>,
    pub balance: i64,
}

// 简化实现: 直接使用State提取，不使用FromRequestParts
// AuthenticatedUser在中间件中已经添加到extensions中
