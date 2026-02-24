//! 认证服务核心实现

use crate::cache::CacheManager;
use crate::config::AuthConfig;
use crate::error::{AuthError, AuthResult};
use crate::models::{Claims, UserInfo, permission_from_user_info, TokenType};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use sqlx::PgPool;
use std::sync::Arc;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// 认证服务
#[derive(Clone)]
pub struct AuthService {
    /// JWT编码密钥
    encoding_key: Arc<EncodingKey>,

    /// JWT解码密钥
    decoding_key: Arc<DecodingKey>,

    /// 数据库连接池
    db: Arc<PgPool>,

    /// 缓存管理器
    cache: Arc<CacheManager>,

    /// 配置
    config: Arc<AuthConfig>,
}

impl AuthService {
    /// 创建新的认证服务
    pub fn new(
        db: PgPool,
        cache: CacheManager,
        config: AuthConfig,
    ) -> Self {
        let encoding_key = EncodingKey::from_secret(config.jwt_secret.as_bytes());
        let decoding_key = DecodingKey::from_secret(config.jwt_secret.as_bytes());

        info!("认证服务初始化完成: issuer={}", config.issuer);

        Self {
            encoding_key: Arc::new(encoding_key),
            decoding_key: Arc::new(decoding_key),
            db: Arc::new(db),
            cache: Arc::new(cache),
            config: Arc::new(config),
        }
    }

    /// 生成Token
    pub fn generate_token(
        &self,
        user_info: &UserInfo,
        token_type: TokenType,
    ) -> AuthResult<String> {
        let now = Utc::now();
        let iat = now.timestamp() as usize;
        let exp = (now + Duration::seconds(token_type.ttl_secs())).timestamp() as usize;

        let jti = Some(Uuid::new_v4().to_string());

        let claims = Claims {
            sub: user_info.user_id.to_string(),
            exp,
            iat,
            iss: self.config.issuer.clone(),
            nbf: iat,
            user_id: user_info.user_id,
            username: user_info.username.clone(),
            email: user_info.email.clone(),
            tier: user_info.tier.clone(),
            scopes: user_info.scopes.clone(),
            token_version: user_info.token_version,
            jti,
        };

        self.encode_jwt(&claims)
    }

    /// 编码JWT
    fn encode_jwt(&self, claims: &Claims) -> AuthResult<String> {
        encode(&Header::default(), claims, &self.encoding_key)
            .map_err(|e| {
                error!("JWT编码失败: {}", e);
                AuthError::from(e)
            })
    }

    /// 解码JWT
    fn decode_jwt(&self, token: &str) -> AuthResult<Claims> {
        decode::<Claims>(
            token,
            &self.decoding_key,
            &Validation::new(jsonwebtoken::Algorithm::HS256),
        )
        .map(|data| data.claims)
        .map_err(|e| match e.kind() {
            jsonwebtoken::errors::ErrorKind::ExpiredSignature => {
                warn!("Token已过期");
                AuthError::ExpiredToken
            }
            jsonwebtoken::errors::ErrorKind::InvalidSignature => {
                warn!("Token签名无效");
                AuthError::InvalidSignature
            }
            jsonwebtoken::errors::ErrorKind::ImmatureSignature => {
                warn!("Token尚未生效");
                AuthError::NotYetValidToken
            }
            _ => {
                error!("JWT解码失败: {}", e);
                AuthError::from(e)
            }
        })
    }

    /// 验证Token (完整流程)
    pub async fn verify_token(&self, token: &str) -> AuthResult<UserInfo> {
        // 1. 检查Token格式
        let token = self.extract_token(token)?;

        // 2. 检查缓存
        if let Some(user) = self.cache.get_user(token).await? {
            debug!("Token验证成功 (缓存命中): user_id={}", user.user_id);
            return Ok(user);
        }

        // 3. 解析JWT
        let claims = self.decode_jwt(token)?;

        // 4. 检查Token是否被撤销
        self.check_revocation(&claims).await?;

        // 5. 获取用户信息
        let user_info = self.get_user_info(claims.user_id).await?;

        // 6. 检查Token版本
        if user_info.token_version != claims.token_version {
            warn!(
                "Token版本不匹配: token版本={}, 用户当前版本={}",
                claims.token_version, user_info.token_version
            );
            return Err(AuthError::TokenRevoked);
        }

        // 7. 检查用户状态
        if user_info.status != "active" {
            warn!("用户账户异常: user_id={}, status={}", user_info.user_id, user_info.status);
            return Err(AuthError::UserSuspended(user_info.status.clone()));
        }

        // 8. 缓存用户信息
        let ttl = self.config.cache().redis_ttl_secs;
        self.cache.put_user(token, &user_info, ttl).await?;

        debug!("Token验证成功: user_id={}, tier={}", user_info.user_id, user_info.tier);

        Ok(user_info)
    }

    /// 提取Token (处理Bearer前缀)
    fn extract_token<'a>(&self, token: &'a str) -> AuthResult<&'a str> {
        let token = token.trim();

        if token.starts_with("Bearer ") {
            Ok(&token[7..])
        } else if token.is_empty() {
            Err(AuthError::MissingToken)
        } else {
            // 假设已经是纯token
            Ok(token)
        }
    }

    /// 检查Token是否被撤销
    async fn check_revocation(&self, claims: &Claims) -> AuthResult<()> {
        // 1. 检查单个Token撤销 (通过jti)
        if let Some(ref jti) = claims.jti {
            if self.cache.is_jti_revoked(jti).await? {
                warn!("Token已被撤销 (jti): jti={}", jti);
                return Err(AuthError::TokenRevoked);
            }
        }

        // 2. 检查用户Token版本黑名单
        if self.cache.is_user_version_revoked(claims.user_id, claims.token_version).await? {
            warn!(
                "Token版本已被撤销: user_id={}, version={}",
                claims.user_id, claims.token_version
            );
            return Err(AuthError::TokenRevoked);
        }

        Ok(())
    }

    /// 从数据库获取用户信息
    async fn get_user_info(&self, user_id: i64) -> AuthResult<UserInfo> {
        let row = sqlx::query_as!(
            UserInfo,
            r#"
            SELECT
                id as user_id,
                username,
                email,
                tier,
                scopes,
                balance,
                token_version,
                status
            FROM users
            WHERE id = $1
            "#,
            user_id
        )
        .fetch_optional(self.db.as_ref())
        .await?
        .ok_or(AuthError::UserNotFound)?;

        Ok(row)
    }

    /// 撤销用户所有Token (增加Token版本)
    pub async fn revoke_user_all_tokens(
        &self,
        user_id: i64,
        reason: &str,
    ) -> AuthResult<()> {
        // 1. 获取当前Token版本
        let current_version: Option<i32> = sqlx::query_scalar(
            "SELECT token_version FROM users WHERE id = $1"
        )
        .bind(user_id)
        .fetch_optional(self.db.as_ref())
        .await?
        .unwrap_or(Some(0));

        let current_version = current_version.unwrap_or(0);

        // 2. 增加Token版本并标记用户状态
        sqlx::query!(
            r#"
            UPDATE users
            SET token_version = token_version + 1,
                status_reason = $2,
                suspended_at = NOW()
            WHERE id = $1
            "#,
            user_id,
            reason
        )
        .execute(self.db.as_ref())
        .await?;

        // 3. 将旧版本加入黑名单 (在Redis中)
        self.cache.add_user_version_to_blacklist(
            user_id,
            current_version,
            86400 * 7, // 黑名单保留7天
        ).await?;

        // 4. 清除缓存
        self.cache.invalidate_user_all(user_id).await?;

        info!(
            "用户所有Token已撤销: user_id={}, 旧版本={}, 原因={}",
            user_id, current_version, reason
        );

        Ok(())
    }

    /// 撤销单个Token (通过jti)
    pub async fn revoke_single_token(&self, jti: &str, ttl: usize) -> AuthResult<()> {
        self.cache.revoke_jti(jti, ttl).await?;

        info!("单个Token已撤销: jti={}, TTL={}s", jti, ttl);

        Ok(())
    }

    /// 检查是否需要续期
    pub fn should_refresh(&self, token: &str) -> AuthResult<bool> {
        let claims = self.decode_jwt(token)?;

        let exp = claims.exp as i64;
        let now = Utc::now().timestamp();
        let time_to_expiry = exp - now;

        let refresh_window = self.config.refresh_window_secs.unwrap_or(43200);

        Ok(time_to_expiry > 0 && time_to_expiry < refresh_window as i64)
    }

    /// 续期Token
    pub fn refresh_token(&self, token: &str, user_info: &UserInfo) -> AuthResult<String> {
        let claims = self.decode_jwt(token)?;

        let now = Utc::now();
        let exp = (now + Duration::seconds(self.config.token_ttl_secs.unwrap_or(86400) as i64))
            .timestamp() as usize;

        let new_claims = Claims {
            sub: claims.sub,
            exp,
            iat: now.timestamp() as usize,
            iss: claims.iss,
            nbf: now.timestamp() as usize,
            user_id: claims.user_id,
            username: user_info.username.clone(),
            email: user_info.email.clone(),
            tier: user_info.tier.clone(),
            scopes: user_info.scopes.clone(),
            token_version: user_info.token_version,
            jti: Some(Uuid::new_v4().to_string()),
        };

        debug!("Token已续期: user_id={}", user_info.user_id);

        self.encode_jwt(&new_claims)
    }

    /// 解析Token (不验证撤销状态, 仅解码)
    pub fn parse_token(&self, token: &str) -> AuthResult<Claims> {
        let token = self.extract_token(token)?;
        self.decode_jwt(token)
    }

    /// 用户登录 (生成Token)
    pub async fn login(
        &self,
        user_id: i64,
        token_type: TokenType,
    ) -> AuthResult<String> {
        let user_info = self.get_user_info(user_id).await?;

        if user_info.status != "active" {
            return Err(AuthError::UserSuspended(user_info.status));
        }

        let token = self.generate_token(&user_info, token_type)?;

        info!("用户登录成功: user_id={}, tier={}", user_id, user_info.tier);

        Ok(token)
    }

    /// 用户登出 (撤销Token)
    pub async fn logout(&self, token: &str) -> AuthResult<()> {
        let claims = self.parse_token(token)?;

        // 撤销单个Token
        if let Some(jti) = claims.jti {
            let ttl = (claims.exp as i64 - Utc::now().timestamp()).max(0) as usize;
            self.revoke_single_token(&jti, ttl).await?;
        }

        // 清除缓存
        let token = self.extract_token(token)?;
        self.cache.invalidate_token(token).await?;

        info!("用户登出: user_id={}", claims.user_id);

        Ok(())
    }

    /// 获取权限对象
    pub fn get_permission(&self, user_info: &UserInfo) -> crate::models::Permission {
        permission_from_user_info(user_info)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::RedisConfig;
    use crate::config::AuthConfig;

    fn create_test_service() -> AuthService {
        // 仅用于单元测试, 实际使用需要真实数据库和Redis
        let config = AuthConfig {
            jwt_secret: "test-secret-key".to_string(),
            issuer: "test-issuer".to_string(),
            token_ttl_secs: Some(86400),
            refresh_window_secs: Some(43200),
            cache: None,
            blacklist: None,
            rate_limit: None,
        };

        let redis_config = RedisConfig {
            url: "redis://127.0.0.1:6379".to_string(),
            pool_max_size: Some(2),
            pool_min_idle: Some(1),
        };

        let cache = CacheManager::new(redis_config, 100, 3600);

        // 注意: 这里需要真实的数据库连接才能创建完整的服务
        // 实际测试应该使用mock或testcontainers
        todo!("需要数据库连接")
    }

    #[test]
    fn test_extract_token() {
        let config = AuthConfig {
            jwt_secret: "test".to_string(),
            issuer: "test".to_string(),
            token_ttl_secs: Some(86400),
            refresh_window_secs: Some(43200),
            cache: None,
            blacklist: None,
            rate_limit: None,
        };

        // 这里需要部分初始化AuthService
        // 实际测试应该重构代码使extract_token成为静态方法
    }
}
