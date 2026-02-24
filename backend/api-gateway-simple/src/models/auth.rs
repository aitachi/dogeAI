// ========== 认证相关模型 ==========
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct TokenQueryRequest {
    pub model: String,
    pub input_tokens: u32,
    pub output_tokens: u32,
}

#[derive(Debug, Serialize)]
pub struct TokenQueryResponse {
    pub status: String,
    pub cost: i64,
    pub balance: i64,
    pub can_proceed: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginRequest {
    /// 兼容用户名或邮箱登录
    pub account: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub status: String,
    pub token: String,
    pub user: UserInfoData,
}

#[derive(Debug, Serialize)]
pub struct UserInfoData {
    pub id: String,
    pub username: String,
    pub balance: i64,
}

#[derive(Debug, Serialize)]
pub struct UserInfoResponse {
    pub id: String,
    pub username: String,
    pub balance: i64,
    pub level: i32,
}
