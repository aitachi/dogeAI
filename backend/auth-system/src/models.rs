//! 认证系统数据模型

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// JWT Claims结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    // ===== 标准声明 =====
    /// subject: 用户ID
    pub sub: String,
    /// expiry time: 过期时间
    pub exp: usize,
    /// issued at: 签发时间
    pub iat: usize,
    /// issuer: 签发者
    pub iss: String,
    /// not before: 生效时间
    pub nbf: usize,

    // ===== 自定义声明 =====
    /// 用户ID
    pub user_id: i64,
    /// 用户名
    pub username: String,
    /// 邮箱
    pub email: String,
    /// 用户等级: Base/Pro/Max/AMax/Enterprise
    pub tier: String,
    /// 权限范围
    pub scopes: Vec<String>,

    // ===== Token管理 =====
    /// Token版本号 (用于黑名单管理)
    #[serde(default = "default_token_version")]
    pub token_version: i32,
    /// JWT ID (可选, 用于单个token撤销)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jti: Option<String>,
}

fn default_token_version() -> i32 {
    0
}

/// 用户信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub user_id: i64,
    pub username: String,
    pub email: String,
    pub tier: String,
    pub scopes: Vec<String>,
    pub balance: i64,
    pub token_version: i32,
    pub status: String,
}

/// 权限对象
#[derive(Debug, Clone)]
pub struct Permission {
    pub user_id: i64,
    pub tier: String,
    pub scopes: HashSet<String>,
    pub allowed_models: HashSet<String>,
    pub qps_limit: usize,
    pub max_concurrent_tasks: usize,
}

impl Permission {
    /// 检查是否可以使用指定模型
    pub fn can_use_model(&self, model: &str) -> bool {
        // 检查通配符
        if self.allowed_models.contains("*") {
            return true;
        }

        // 精确匹配
        if self.allowed_models.contains(model) {
            return true;
        }

        // 前缀匹配
        for allowed in &self.allowed_models {
            if let Some(prefix) = allowed.strip_prefix("model:") {
                if model.starts_with(prefix) || model == prefix {
                    return true;
                }
            }
        }

        false
    }

    /// 检查是否有指定权限范围
    pub fn has_scope(&self, scope: &str) -> bool {
        self.scopes.contains(scope) || self.scopes.contains("*")
    }

    /// 检查是否有任意一个权限范围
    pub fn has_any_scope(&self, scopes: &[&str]) -> bool {
        scopes.iter().any(|s| self.has_scope(s))
    }

    /// 获取QPS限制
    pub fn get_qps_limit(&self) -> usize {
        self.qps_limit
    }

    /// 获取最大并发任务数
    pub fn get_max_concurrent_tasks(&self) -> usize {
        self.max_concurrent_tasks
    }
}

/// 用户等级配置
#[derive(Debug, Clone)]
pub struct TierConfig {
    pub qps_limit: usize,
    pub max_concurrent_tasks: usize,
    pub daily_points: i64,
    pub allowed_models: Vec<String>,
}

/// 获取用户等级配置
pub fn get_tier_config(tier: &str) -> TierConfig {
    match tier {
        "Base" => TierConfig {
            qps_limit: 10,
            max_concurrent_tasks: 5,
            daily_points: 880,
            allowed_models: vec!["opus".to_string(), "sonnet".to_string(), "haiku".to_string()],
        },
        "Pro" => TierConfig {
            qps_limit: 100,
            max_concurrent_tasks: 5,
            daily_points: 1850,
            allowed_models: vec!["opus".to_string(), "sonnet".to_string(), "haiku".to_string()],
        },
        "Max" => TierConfig {
            qps_limit: 200,
            max_concurrent_tasks: 5,
            daily_points: 3800,
            allowed_models: vec!["opus".to_string(), "sonnet".to_string(), "haiku".to_string()],
        },
        "AMax" => TierConfig {
            qps_limit: 300,
            max_concurrent_tasks: 5,
            daily_points: 7800,
            allowed_models: vec!["opus".to_string(), "sonnet".to_string(), "haiku".to_string()],
        },
        "Enterprise" => TierConfig {
            qps_limit: usize::MAX,
            max_concurrent_tasks: 10,
            daily_points: i64::MAX,
            allowed_models: vec!["*".to_string()],
        },
        _ => TierConfig {
            qps_limit: 10,
            max_concurrent_tasks: 5,
            daily_points: 880,
            allowed_models: vec!["haiku".to_string()],
        },
    }
}

/// 从用户等级创建权限对象
pub fn permission_from_user_info(user_info: &UserInfo) -> Permission {
    let tier_config = get_tier_config(&user_info.tier);

    Permission {
        user_id: user_info.user_id,
        tier: user_info.tier.clone(),
        scopes: user_info.scopes.iter().cloned().collect(),
        allowed_models: tier_config.allowed_models.into_iter().collect(),
        qps_limit: tier_config.qps_limit,
        max_concurrent_tasks: tier_config.max_concurrent_tasks,
    }
}

/// Token类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenType {
    /// 短生命周期Token (24小时)
    Short,
    /// 长生命周期Token (1年)
    Long,
    /// 会话Token (7天)
    Session,
}

impl TokenType {
    /// 获取Token有效期 (秒)
    pub fn ttl_secs(&self) -> i64 {
        match self {
            TokenType::Short => 86400,      // 24小时
            TokenType::Long => 31536000,    // 365天
            TokenType::Session => 604800,   // 7天
        }
    }

    /// 获取续期窗口 (秒)
    pub fn refresh_window_secs(&self) -> i64 {
        match self {
            TokenType::Short => 43200,      // 12小时
            TokenType::Long => 2592000,     // 30天
            TokenType::Session => 86400,    // 24小时
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permission_model_access() {
        let permission = Permission {
            user_id: 1,
            tier: "Pro".to_string(),
            scopes: ["read", "write"].iter().map(|s| s.to_string()).collect(),
            allowed_models: ["opus", "sonnet", "haiku"].iter().map(|s| s.to_string()).collect(),
            qps_limit: 100,
            max_concurrent_tasks: 5,
        };

        assert!(permission.can_use_model("opus"));
        assert!(permission.can_use_model("sonnet"));
        assert!(!permission.can_use_model("gpt4"));
        assert!(permission.has_scope("read"));
        assert!(!permission.has_scope("admin"));
    }

    #[test]
    fn test_tier_config() {
        let pro_config = get_tier_config("Pro");
        assert_eq!(pro_config.qps_limit, 100);
        assert_eq!(pro_config.max_concurrent_tasks, 5);

        let enterprise_config = get_tier_config("Enterprise");
        assert_eq!(enterprise_config.qps_limit, usize::MAX);
        assert!(enterprise_config.allowed_models.contains(&"*".to_string()));
    }

    #[test]
    fn test_token_type() {
        assert_eq!(TokenType::Short.ttl_secs(), 86400);
        assert_eq!(TokenType::Long.ttl_secs(), 31536000);
        assert_eq!(TokenType::Session.ttl_secs(), 604800);
    }
}
