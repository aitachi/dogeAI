use crate::database::Database;
use crate::models::{AppError, Result};
use rand::Rng;
use sha2::{Sha256, Digest};
use base64::{Engine as _, engine::general_purpose};

#[derive(Clone)]
pub struct AuthUser {
    pub user_id: i64,
    pub email: String,
    pub username: Option<String>,
    pub api_key: String,
    pub tier: String,
    pub status: String,
    pub concurrent_limit: i32,
}

/// 密码哈希结构
#[derive(Debug, Clone)]
pub struct PasswordHash {
    pub hash: String,
    pub salt: String,
}

/// 生成随机盐
pub fn generate_salt() -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let mut rng = rand::thread_rng();
    let salt: String = (0..32)
        .map(|_| CHARSET[rng.gen_range(0..CHARSET.len())] as char)
        .collect();
    salt
}

/// 哈希密码
pub fn hash_password(password: &str, salt: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(password.as_bytes());
    hasher.update(salt.as_bytes());
    let result = hasher.finalize();
    general_purpose::STANDARD.encode(result)
}

/// 验证密码
pub fn verify_password(password: &str, hash: &str, salt: &str) -> bool {
    let computed_hash = hash_password(password, salt);
    computed_hash == hash
}

/// 创建密码哈希
pub fn create_password_hash(password: &str) -> PasswordHash {
    let salt = generate_salt();
    let hash = hash_password(password, &salt);
    PasswordHash { hash, salt }
}

/// 从请求头提取API Key
pub fn extract_api_key(headers: &axum::http::HeaderMap) -> Result<String> {
    let auth_header = headers
        .get("Authorization")
        .ok_or(AppError::Unauthorized)?
        .to_str()
        .map_err(|_| AppError::Unauthorized)?;

    if !auth_header.starts_with("Bearer ") {
        return Err(AppError::Unauthorized);
    }

    let api_key = auth_header[7..].to_string();
    if api_key.is_empty() {
        return Err(AppError::Unauthorized);
    }

    // 验证API Key格式 (长度限制)
    if api_key.len() > 256 {
        return Err(AppError::InvalidApiKey);
    }

    Ok(api_key)
}

/// 验证API Key并返回用户信息
pub async fn verify_api_key(
    db: &Database,
    api_key: &str,
) -> Result<AuthUser> {
    let user = db.get_user_by_api_key(api_key).await?;

    if user.status != "active" {
        return Err(AppError::Unauthorized);
    }

    Ok(AuthUser {
        user_id: user.id,
        email: user.email,
        username: user.username,
        api_key: user.api_key,
        tier: user.tier,
        status: user.status,
        concurrent_limit: user.concurrent_limit,
    })
}
