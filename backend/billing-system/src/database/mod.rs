use crate::models::{User, BillingRecord};
use crate::models::Result;
use sqlx::{PgPool, FromRow};
use chrono::{DateTime, Utc};

#[derive(Clone)]
pub struct Database {
    pool: PgPool,
}

// ==================== 管理端结构体 ====================

/// 对账记录结构体
#[derive(Debug, Clone, FromRow)]
pub struct ReconciliationRecord {
    pub id: i64,
    pub user_id: i64,
    pub db_balance: i64,
    pub redis_balance: i64,
    pub difference: i64,
    pub reconciled_at: DateTime<Utc>,
}

/// 充值码批次结构体
#[derive(Debug, Clone, FromRow)]
pub struct RechargeBatch {
    pub id: i64,
    pub batch_id: String,
    pub package_type: String,
    pub total_count: i64,
    pub used_count: i64,
    pub created_at: Option<DateTime<Utc>>,
}

/// 充值码统计
#[derive(Debug, Clone)]
pub struct RechargeStats {
    pub total: i64,
    pub used: i64,
    pub unused: i64,
    pub batches: i64,
}

impl Database {
    pub async fn new(_database_url: &str) -> Result<Self> {
        let pool = PgPool::connect("postgresql://postgres@localhost:5432/api_gateway").await?;
        Ok(Database { pool })
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    // ==================== 用户操作 ====================

    pub async fn get_user_by_api_key(&self, api_key: &str) -> Result<User> {
        use sqlx::Error as SqlxError;

        let user = sqlx::query_as::<_, User>(
            r#"
            SELECT
                id, email, username, api_key, balance, tier, status, concurrent_limit,
                password_hash, salt, created_at, updated_at
            FROM users WHERE api_key = $1
            "#
        )
        .bind(api_key)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            SqlxError::RowNotFound => crate::models::AppError::InvalidApiKey,
            _ => crate::models::AppError::DatabaseError(e),
        })?;
        Ok(user)
    }

    pub async fn get_user_by_email(&self, email: &str) -> Result<User> {
        use sqlx::Error as SqlxError;

        let user = sqlx::query_as::<_, User>(
            r#"
            SELECT
                id, email, username, api_key, balance, tier, status, concurrent_limit,
                password_hash, salt, created_at, updated_at
            FROM users WHERE email = $1
            "#
        )
        .bind(email)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            SqlxError::RowNotFound => crate::models::AppError::Unauthorized,
            _ => crate::models::AppError::DatabaseError(e),
        })?;
        Ok(user)
    }

    pub async fn create_user(&self, email: &str, username: Option<&str>) -> Result<User> {
        let api_key = generate_api_key();
        let balance = 200i64;

        // 生成user_id
        let user_id = format!("user_{}", uuid::Uuid::new_v4());
        // 生成时间戳
        let now = chrono::Utc::now().timestamp();

        let user = sqlx::query_as::<_, User>(
            r#"
            INSERT INTO users (user_id, email, username, api_key, balance, tier, status, concurrent_limit, created_at_old, updated_at_old, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, 'Base', 'active', 100, $6, $6, NOW(), NOW())
            RETURNING
                id, email, username, api_key, balance, tier, status, concurrent_limit,
                password_hash, salt, created_at, updated_at
            "#
        )
        .bind(&user_id)
        .bind(email)
        .bind(username)
        .bind(&api_key)
        .bind(balance)
        .bind(now)
        .fetch_one(&self.pool)
        .await?;
        Ok(user)
    }

    /// 创建用户（带密码）
    pub async fn create_user_with_password(
        &self,
        username: &str,
        email: &str,
        password_hash: &str,
        salt: &str,
    ) -> Result<User> {
        let api_key = generate_api_key();
        let balance = 200i64;
        let user_id = format!("user_{}", uuid::Uuid::new_v4());

        let user = sqlx::query_as::<_, User>(
            r#"
            INSERT INTO users (user_id, username, email, api_key, balance, tier, status, concurrent_limit, password_hash, salt, created_at_old, updated_at_old, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, 'Base', 'active', 100, $6, $7, $8, $8, NOW(), NOW())
            RETURNING
                id, email, username, api_key, balance, tier, status, concurrent_limit,
                password_hash, salt, created_at, updated_at
            "#
        )
        .bind(&user_id)
        .bind(username)
        .bind(email)
        .bind(&api_key)
        .bind(balance)
        .bind(password_hash)
        .bind(salt)
        .bind(chrono::Utc::now().timestamp())
        .fetch_one(&self.pool)
        .await?;
        Ok(user)
    }

    /// 通过用户名获取用户
    pub async fn get_user_by_username(&self, username: &str) -> Result<User> {
        use sqlx::Error as SqlxError;

        let user = sqlx::query_as::<_, User>(
            r#"
            SELECT
                id, email, username, api_key, balance, tier, status, concurrent_limit,
                password_hash, salt, created_at, updated_at
            FROM users WHERE username = $1
            "#
        )
        .bind(username)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            SqlxError::RowNotFound => crate::models::AppError::Unauthorized,
            _ => crate::models::AppError::DatabaseError(e),
        })?;
        Ok(user)
    }

    /// 检查邮箱或用户名是否已存在
    pub async fn check_user_exists(&self, email: &str, username: &str) -> Result<bool> {
        let count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM users
            WHERE email = $1 OR username = $2
            "#
        )
        .bind(email)
        .bind(username)
        .fetch_one(&self.pool)
        .await?;
        Ok(count > 0)
    }

    /// 检查邮箱是否已存在
    pub async fn check_email_exists(&self, email: &str) -> Result<bool> {
        let count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM users WHERE email = $1
            "#
        )
        .bind(email)
        .fetch_one(&self.pool)
        .await?;
        Ok(count > 0)
    }

    /// 检查用户名是否已存在
    pub async fn check_username_exists(&self, username: &str) -> Result<bool> {
        let count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM users WHERE username = $1
            "#
        )
        .bind(username)
        .fetch_one(&self.pool)
        .await?;
        Ok(count > 0)
    }

    /// 更新用户密码
    pub async fn update_user_password(&self, user_id: i64, password_hash: &str, salt: &str) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE users SET password_hash = $1, salt = $2, updated_at_old = $3, updated_at = NOW()
            WHERE id = $4
            "#
        )
        .bind(password_hash)
        .bind(salt)
        .bind(chrono::Utc::now().timestamp())
        .bind(user_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// 更新用户名
    pub async fn update_user_username(&self, user_id: i64, username: &str) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE users SET username = $1, updated_at_old = $2, updated_at = NOW()
            WHERE id = $3
            "#
        )
        .bind(username)
        .bind(chrono::Utc::now().timestamp())
        .bind(user_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// 获取用户详情（包含密码哈希）
    pub async fn get_user_details_by_api_key(&self, api_key: &str) -> Result<User> {
        use sqlx::Error as SqlxError;

        let user = sqlx::query_as::<_, User>(
            r#"
            SELECT
                id, email, username, api_key, balance, tier, status, concurrent_limit,
                password_hash, salt, created_at, updated_at
            FROM users WHERE api_key = $1
            "#
        )
        .bind(api_key)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            SqlxError::RowNotFound => crate::models::AppError::InvalidApiKey,
            _ => crate::models::AppError::DatabaseError(e),
        })?;
        Ok(user)
    }

    // ==================== 计费记录 ====================

    pub async fn create_billing_record(
        &self,
        user_id: i64,
        request_id: &str,
        model: &str,
        input_tokens: i32,
        output_tokens: i32,
        cost: i32,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO billing_records
            (user_id, request_id, model, input_tokens, output_tokens, total_tokens, cost, context_doubled)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (request_id) DO NOTHING
            "#
        )
        .bind(user_id)
        .bind(request_id)
        .bind(model)
        .bind(input_tokens)
        .bind(output_tokens)
        .bind(input_tokens + output_tokens)
        .bind(cost)
        .bind(input_tokens + output_tokens > 32000)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_billing_records(&self, user_id: i64, limit: i64) -> Result<Vec<BillingRecord>> {
        let records = sqlx::query_as::<_, BillingRecord>(
            r#"
            SELECT
                id, user_id, request_id, model, input_tokens, output_tokens,
                total_tokens, cost, context_doubled, billing_status, created_at
            FROM billing_records
            WHERE user_id = $1
            ORDER BY created_at DESC
            LIMIT $2
            "#
        )
        .bind(user_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(records)
    }

    // ==================== 管理端方法 ====================

    /// 获取所有用户（分页）
    pub async fn get_all_users(&self, page: i32, limit: i32) -> Result<Vec<User>> {
        let offset = (page - 1) * limit;
        let users = sqlx::query_as::<_, User>(
            r#"
            SELECT
                id, email, username, api_key, balance, tier, status, concurrent_limit,
                created_at, updated_at
            FROM users
            ORDER BY created_at DESC
            LIMIT $1 OFFSET $2
            "#
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;
        Ok(users)
    }

    /// 获取所有计费记录（分页）
    pub async fn get_all_billing_records(&self, page: i32, limit: i32) -> Result<Vec<BillingRecord>> {
        let offset = (page - 1) * limit;
        let records = sqlx::query_as::<_, BillingRecord>(
            r#"
            SELECT
                id, user_id, request_id, model, input_tokens, output_tokens,
                total_tokens, cost, context_doubled, billing_status, created_at
            FROM billing_records
            ORDER BY created_at DESC
            LIMIT $1 OFFSET $2
            "#
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;
        Ok(records)
    }

    /// 获取对账记录
    pub async fn get_reconciliation_records(&self, limit: i32) -> Result<Vec<ReconciliationRecord>> {
        let records = sqlx::query_as::<_, ReconciliationRecord>(
            r#"
            SELECT
                id, user_id, db_balance, redis_balance, difference, reconciled_at
            FROM balance_reconciliation
            ORDER BY reconciled_at DESC
            LIMIT $1
            "#
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(records)
    }

    /// 获取待处理的计费记录
    pub async fn get_pending_billing_records(&self, limit: i32) -> Result<Vec<BillingRecord>> {
        let records = sqlx::query_as::<_, BillingRecord>(
            r#"
            SELECT
                id, user_id, request_id, model, input_tokens, output_tokens,
                total_tokens, cost, context_doubled, billing_status, created_at
            FROM billing_records
            WHERE billing_status = 'pending'
            ORDER BY created_at ASC
            LIMIT $1
            "#
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(records)
    }

    /// 获取充值码统计
    pub async fn get_recharge_stats(&self) -> Result<RechargeStats> {
        let row = sqlx::query_as::<_, (i64, i64, i64, i64)>(
            r#"
            SELECT
                (SELECT COUNT(*) FROM recharge_codes) as total,
                (SELECT COUNT(*) FROM recharge_codes WHERE used_by IS NOT NULL) as used,
                (SELECT COUNT(*) FROM recharge_codes WHERE used_by IS NULL) as unused,
                (SELECT COUNT(DISTINCT batch_id) FROM recharge_codes) as batches
            "#
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(RechargeStats {
            total: row.0,
            used: row.1,
            unused: row.2,
            batches: row.3,
        })
    }

    /// 获取充值码批次列表
    pub async fn get_recharge_batches(&self) -> Result<Vec<RechargeBatch>> {
        let batches = sqlx::query_as::<_, RechargeBatch>(
            r#"
            SELECT
                rb.id,
                rb.batch_id,
                CASE rb.points_per_code
                    WHEN 100 THEN 'S100'
                    WHEN 500 THEN 'S500'
                    WHEN 1000 THEN 'S1000'
                    WHEN 5000 THEN 'S5000'
                    ELSE 'S' || rb.points_per_code
                END as package_type,
                rb.total_codes::bigint as total_count,
                rb.used_codes::bigint as used_count,
                rb.created_at::timestamptz as created_at
            FROM recharge_batches rb
            ORDER BY rb.created_at DESC
            LIMIT 50
            "#
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(batches)
    }

    // ==================== 模型访问控制 ====================

    /// 检查用户等级是否可以访问指定模型
    pub async fn check_model_access(&self, tier: &str, model: &str) -> Result<bool> {
        use sqlx::Error as SqlxError;

        // 标准化tier名称（处理Base, base等）
        let tier_normalized = tier.to_lowercase();
        let model_lower = model.to_lowercase();

        // 尝试精确匹配模型
        let exact_match: std::result::Result<bool, SqlxError> = sqlx::query_scalar::<_, bool>(
            "SELECT allowed FROM tier_model_access WHERE LOWER(tier) = $1 AND model_pattern = $2"
        )
        .bind(&tier_normalized)
        .bind(&model_lower)
        .fetch_one(&self.pool)
        .await;

        match exact_match {
            Ok(allowed) => return Ok(allowed),
            Err(SqlxError::RowNotFound) => {},
            Err(e) => return Err(crate::models::AppError::DatabaseError(e)),
        }

        // 尝试模糊匹配（检查模型名是否包含pattern）
        let pattern_match: std::result::Result<bool, SqlxError> = sqlx::query_scalar::<_, bool>(
            r#"
            SELECT allowed FROM tier_model_access
            WHERE LOWER(tier) = $1
            AND $2 LIKE '%' || model_pattern || '%'
            ORDER BY LENGTH(model_pattern) DESC
            LIMIT 1
            "#
        )
        .bind(&tier_normalized)
        .bind(&model_lower)
        .fetch_one(&self.pool)
        .await;

        match pattern_match {
            Ok(allowed) => Ok(allowed),
            Err(SqlxError::RowNotFound) => Ok(true), // 默认允许
            Err(e) => Err(crate::models::AppError::DatabaseError(e)),
        }
    }

    /// 记录请求日志
    pub async fn log_request(
        &self,
        request_id: &str,
        user_id: i64,
        api_key_prefix: &str,
        model_requested: &str,
        model_actual: Option<&str>,
        provider_used: Option<&str>,
        key_id_used: Option<i32>,
        status: &str,
        error_message: Option<&str>,
        input_tokens: i32,
        output_tokens: i32,
        cost: i32,
        latency_ms: i32,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO request_log
            (request_id, user_id, api_key_prefix, model_requested, model_actual,
             provider_used, key_id_used, status, error_message, input_tokens,
             output_tokens, cost, latency_ms, completed_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, NOW())
            "#
        )
        .bind(request_id)
        .bind(user_id)
        .bind(api_key_prefix)
        .bind(model_requested)
        .bind(model_actual)
        .bind(provider_used)
        .bind(key_id_used)
        .bind(status)
        .bind(error_message)
        .bind(input_tokens)
        .bind(output_tokens)
        .bind(cost)
        .bind(latency_ms)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// 获取用户请求统计
    pub async fn get_user_request_stats(&self, user_id: i64, days: i32) -> Result<(i64, i64)> {
        let row = sqlx::query_as::<_, (i64, i64)>(
            r#"
            SELECT
                COUNT(*) as total_requests,
                COALESCE(SUM(cost), 0) as total_cost
            FROM request_log
            WHERE user_id = $1
            AND created_at > NOW() - INTERVAL '1 day' * $2
            AND status = 'success'
            "#
        )
        .bind(user_id)
        .bind(days)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }
}

// ==================== 辅助函数 ====================

use rand::Rng;

fn generate_api_key() -> String {
    const CHARSET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    let mut rng = rand::thread_rng();

    // Anthropic API key 格式: sk-ant-xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
    let mut key = String::from("sk-ant-");
    for _ in 0..48 {
        let idx = rng.gen_range(0..CHARSET.len());
        key.push(CHARSET[idx] as char);
    }

    key
}
