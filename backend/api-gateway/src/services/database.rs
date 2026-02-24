use crate::models::{ApiError, BillingRecord, ModelConfig, TokenPricing, UserTier};
use sqlx::postgres::PgPoolOptions;
use std::collections::HashMap;
use std::sync::Arc;

/// 数据库连接池
#[derive(Clone)]
pub struct DbPool {
    pool: sqlx::PgPool,
}

impl DbPool {
    /// 创建新的数据库连接池
    pub async fn new(database_url: &str) -> Result<Self, ApiError> {
        let pool = PgPoolOptions::new()
            .max_connections(30)
            .connect(database_url)
            .await
            .map_err(|e| ApiError::Database(e.to_string()))?;

        Ok(Self { pool })
    }

    /// 获取连接池
    pub fn get_pool(&self) -> &sqlx::PgPool {
        &self.pool
    }

    /// 运行数据库迁移
    pub async fn migrate(&self) -> Result<(), ApiError> {
        sqlx::migrate!("./migrations")
            .run(&self.pool)
            .await
            .map_err(|e| ApiError::Database(e.to_string()))?;
        Ok(())
    }
}

/// 计费服务
#[derive(Clone)]
pub struct BillingService {
    db: DbPool,
    model_configs: Arc<HashMap<String, ModelConfig>>,
}

impl BillingService {
    pub fn new(db: DbPool) -> Self {
        // 初始化模型价格配置
        let mut model_configs = HashMap::new();

        // Claude Opus
        model_configs.insert(
            "opus".to_string(),
            ModelConfig {
                name: "opus".to_string(),
                pricing: TokenPricing {
                    input_price_per_million: 150_000,  // 15积分/百万tokens
                    output_price_per_million: 750_000, // 75积分/百万tokens
                },
                max_tokens: 200_000,
            },
        );

        // Claude Sonnet
        model_configs.insert(
            "sonnet".to_string(),
            ModelConfig {
                name: "sonnet".to_string(),
                pricing: TokenPricing {
                    input_price_per_million: 30_000,
                    output_price_per_million: 150_000,
                },
                max_tokens: 200_000,
            },
        );

        // Claude Haiku
        model_configs.insert(
            "haiku".to_string(),
            ModelConfig {
                name: "haiku".to_string(),
                pricing: TokenPricing {
                    input_price_per_million: 2_500,
                    output_price_per_million: 12_500,
                },
                max_tokens: 200_000,
            },
        );

        Self {
            db,
            model_configs: Arc::new(model_configs),
        }
    }

    /// 计算Token消耗费用
    pub fn calculate_cost(
        &self,
        model: &str,
        input_tokens: u32,
        output_tokens: u32,
    ) -> Result<i64, ApiError> {
        let config = self
            .model_configs
            .get(model)
            .ok_or_else(|| ApiError::ModelNotFound(model.to_string()))?;

        Ok(config.pricing.calculate_cost(input_tokens, output_tokens))
    }

    /// 获取用户余额
    pub async fn get_balance(&self, user_id: i64) -> Result<i64, ApiError> {
        let row: Option<(i64,)> = sqlx::query_as("SELECT balance FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_optional(self.db.get_pool())
            .await
            .map_err(|e| ApiError::Database(e.to_string()))?;

        row.map(|(balance,)| balance)
            .ok_or_else(|| ApiError::InvalidRequest("用户不存在".to_string()))
    }

    /// 检查用户余额是否足够
    pub async fn has_sufficient_balance(
        &self,
        user_id: i64,
        required: i64,
    ) -> Result<bool, ApiError> {
        let balance = self.get_balance(user_id).await?;
        Ok(balance >= required)
    }

    /// 同步扣费（用于非流式响应）
    pub async fn deduct_balance(
        &self,
        user_id: i64,
        amount: i64,
    ) -> Result<(), ApiError> {
        let now = chrono::Utc::now();
        let mut tx = self
            .db
            .get_pool()
            .begin()
            .await
            .map_err(|e| ApiError::Database(e.to_string()))?;

        // 扣除余额
        sqlx::query("UPDATE users SET balance = balance - $1, updated_at = $2 WHERE id = $3")
            .bind(amount)
            .bind(now)
            .bind(user_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| ApiError::Database(e.to_string()))?;

        // 记录交易
        sqlx::query(
            "INSERT INTO transactions (user_id, amount, transaction_type, created_at)
             VALUES ($1, $2, 'deduct', $3)",
        )
        .bind(user_id)
        .bind(amount)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(|e| ApiError::Database(e.to_string()))?;

        tx.commit()
            .await
            .map_err(|e| ApiError::Database(e.to_string()))?;

        Ok(())
    }

    /// 异步记录计费（用于流式响应或高并发场景）
    pub async fn record_billing_async(&self, record: BillingRecord) -> Result<(), ApiError> {
        let pool = self.db.get_pool().clone();

        tokio::spawn(async move {
            if let Ok(mut tx) = pool.begin().await {
                // 扣除余额
                let _ = sqlx::query("UPDATE users SET balance = balance - $1 WHERE id = $2")
                    .bind(record.cost)
                    .bind(record.user_id)
                    .execute(&mut *tx)
                    .await;

                // 记录交易
                let _ = sqlx::query(
                    "INSERT INTO transactions (user_id, amount, transaction_type, created_at)
                     VALUES ($1, $2, 'deduct', $3)",
                )
                .bind(record.user_id)
                .bind(record.cost)
                .bind(record.timestamp)
                .execute(&mut *tx)
                .await;

                let _ = tx.commit().await;
            }
        });

        Ok(())
    }

    /// 获取用户信息
    pub async fn get_user(&self, user_id: i64) -> Result<(i64, String, i64), ApiError> {
        let row: Option<(i64, String, i64)> = sqlx::query_as(
            "SELECT id, tier, balance FROM users WHERE id = $1",
        )
        .bind(user_id)
        .fetch_optional(self.db.get_pool())
        .await
        .map_err(|e| ApiError::Database(e.to_string()))?;

        row.ok_or_else(|| ApiError::InvalidRequest("用户不存在".to_string()))
    }

    /// 获取用户套餐等级
    pub async fn get_user_tier(&self, user_id: i64) -> Result<UserTier, ApiError> {
        let row: Option<(String,)> = sqlx::query_as("SELECT tier FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_optional(self.db.get_pool())
            .await
            .map_err(|e| ApiError::Database(e.to_string()))?;

        let tier_str = row
            .ok_or_else(|| ApiError::InvalidRequest("用户不存在".to_string()))?
            .0;

        UserTier::from_str(&tier_str).ok_or_else(|| ApiError::Internal)
    }

    /// 验证Token有效性
    pub async fn validate_token(&self, token: &str) -> Result<Option<i64>, ApiError> {
        let row: Option<(i64,)> = sqlx::query_as(
            "SELECT user_id FROM api_tokens WHERE token = $1 AND is_active = true AND (expires_at IS NULL OR expires_at > NOW())",
        )
        .bind(token)
        .fetch_optional(self.db.get_pool())
        .await
        .map_err(|e| ApiError::Database(e.to_string()))?;

        Ok(row.map(|(user_id,)| user_id))
    }
}

/// 模型服务
#[derive(Clone)]
pub struct ModelService {
    configs: Arc<HashMap<String, ModelConfig>>,
}

impl ModelService {
    pub fn new() -> Self {
        let mut configs = HashMap::new();

        // 添加完整的 Claude 模型名称（Claude Code 使用的格式）
        // Claude 4.6 系列
        configs.insert(
            "claude-opus-4-6".to_string(),
            ModelConfig {
                name: "claude-opus-4-6".to_string(),
                pricing: TokenPricing {
                    input_price_per_million: 150_000,
                    output_price_per_million: 750_000,
                },
                max_tokens: 200_000,
            },
        );

        configs.insert(
            "claude-sonnet-4-5-20241022".to_string(),
            ModelConfig {
                name: "claude-sonnet-4-5-20241022".to_string(),
                pricing: TokenPricing {
                    input_price_per_million: 30_000,
                    output_price_per_million: 150_000,
                },
                max_tokens: 200_000,
            },
        );

        configs.insert(
            "claude-haiku-4-5-20251001".to_string(),
            ModelConfig {
                name: "claude-haiku-4-5-20251001".to_string(),
                pricing: TokenPricing {
                    input_price_per_million: 2_500,
                    output_price_per_million: 12_500,
                },
                max_tokens: 200_000,
            },
        );

        // Claude 3.5 系列（兼容）
        configs.insert(
            "claude-3-5-sonnet-20241022".to_string(),
            ModelConfig {
                name: "claude-3-5-sonnet-20241022".to_string(),
                pricing: TokenPricing {
                    input_price_per_million: 30_000,
                    output_price_per_million: 150_000,
                },
                max_tokens: 200_000,
            },
        );

        configs.insert(
            "claude-3-5-haiku-20241022".to_string(),
            ModelConfig {
                name: "claude-3-5-haiku-20241022".to_string(),
                pricing: TokenPricing {
                    input_price_per_million: 2_500,
                    output_price_per_million: 12_500,
                },
                max_tokens: 200_000,
            },
        );

        // Claude 3 Opus
        configs.insert(
            "claude-3-opus-20240229".to_string(),
            ModelConfig {
                name: "claude-3-opus-20240229".to_string(),
                pricing: TokenPricing {
                    input_price_per_million: 150_000,
                    output_price_per_million: 750_000,
                },
                max_tokens: 200_000,
            },
        );

        // 简短名称（向后兼容）
        configs.insert(
            "opus".to_string(),
            ModelConfig {
                name: "claude-opus-4-6".to_string(),
                pricing: TokenPricing {
                    input_price_per_million: 150_000,
                    output_price_per_million: 750_000,
                },
                max_tokens: 200_000,
            },
        );

        configs.insert(
            "sonnet".to_string(),
            ModelConfig {
                name: "claude-sonnet-4-5-20241022".to_string(),
                pricing: TokenPricing {
                    input_price_per_million: 30_000,
                    output_price_per_million: 150_000,
                },
                max_tokens: 200_000,
            },
        );

        configs.insert(
            "haiku".to_string(),
            ModelConfig {
                name: "claude-haiku-4-5-20251001".to_string(),
                pricing: TokenPricing {
                    input_price_per_million: 2_500,
                    output_price_per_million: 12_500,
                },
                max_tokens: 200_000,
            },
        );

        Self {
            configs: Arc::new(configs),
        }
    }

    pub fn get_all_models(&self) -> Vec<crate::models::Model> {
        // 返回 Claude Code 需要的模型列表格式
        vec![
            crate::models::Model {
                id: "claude-opus-4-6".to_string(),
                object: "model".to_string(),
                owned_by: "anthropic".to_string(),
            },
            crate::models::Model {
                id: "claude-sonnet-4-5-20241022".to_string(),
                object: "model".to_string(),
                owned_by: "anthropic".to_string(),
            },
            crate::models::Model {
                id: "claude-haiku-4-5-20251001".to_string(),
                object: "model".to_string(),
                owned_by: "anthropic".to_string(),
            },
            crate::models::Model {
                id: "claude-3-5-sonnet-20241022".to_string(),
                object: "model".to_string(),
                owned_by: "anthropic".to_string(),
            },
            crate::models::Model {
                id: "claude-3-5-haiku-20241022".to_string(),
                object: "model".to_string(),
                owned_by: "anthropic".to_string(),
            },
            crate::models::Model {
                id: "claude-3-opus-20240229".to_string(),
                object: "model".to_string(),
                owned_by: "anthropic".to_string(),
            },
        ]
    }

    pub fn get_model_config(&self, model: &str) -> Option<&ModelConfig> {
        // 先尝试直接匹配
        if let Some(cfg) = self.configs.get(model) {
            return Some(cfg);
        }

        // 尝试规范化模型名称后再匹配
        let normalized = crate::models::normalize_model_name(model);
        self.configs.get(&normalized)
    }

    pub fn model_exists(&self, model: &str) -> bool {
        // 先尝试直接匹配
        if self.configs.contains_key(model) {
            return true;
        }

        // 尝试规范化模型名称后再匹配
        let normalized = crate::models::normalize_model_name(model);
        self.configs.contains_key(&normalized)
    }

    /// 获取用于上游 API 的模型名称
    pub fn get_upstream_model(&self, model: &str) -> String {
        let normalized = crate::models::normalize_model_name(model);
        // 对于某些模型，可能需要映射到不同的上游模型
        match normalized.as_str() {
            "claude-opus-4-6" => "claude-3-5-sonnet-20241022".to_string(), // 如果上游不支持 4.6
            _ => normalized,
        }
    }
}
