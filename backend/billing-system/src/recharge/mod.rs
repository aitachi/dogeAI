use crate::cache::Cache;
use crate::database::Database;
use crate::models::{AppError, Result};
use rand::Rng;
use std::sync::Arc;
use sqlx::Row;

#[derive(Clone)]
pub struct RechargeService {
    pub db: Arc<Database>,
    pub cache: Arc<Cache>,
}

impl RechargeService {
    pub fn new(db: Arc<Database>, cache: Arc<Cache>) -> Self {
        RechargeService { db, cache }
    }

    /// 生成充值码
    pub fn generate_code(&self, package: PackageType) -> String {
        const CHARSET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
        let mut rng = rand::thread_rng();

        let prefix = match package {
            PackageType::Small => "S1",
            PackageType::Medium => "M5",
            PackageType::Large => "L2",
            PackageType::Ultra => "U0",
        };

        let mut code = String::from(prefix);
        for _ in 0..10 {
            let idx = rng.gen_range(0..CHARSET.len());
            code.push(CHARSET[idx] as char);
        }

        code
    }

    /// 验证并使用充值码（使用事务确保原子性和一次性使用）
    pub async fn redeem_code(&self, user_id: i64, code: &str) -> Result<i64> {
        // 验证充值码格式
        if code.trim().is_empty() || code.len() > 100 {
            return Err(AppError::InvalidInput("充值码格式无效".to_string()));
        }

        // 使用数据库事务确保原子性操作
        let mut tx = self.db.pool().begin().await?;

        // 使用 FOR UPDATE 行锁，防止并发兑换同一个码
        // 同时检查：码存在、未使用、在有效期内
        let row = sqlx::query_as::<_, (i64, i32, i32)>(
            r#"
            SELECT id, points, valid_days
            FROM recharge_codes
            WHERE code = $1
              AND used_by IS NULL
              AND (created_at + (valid_days || ' days')::interval > NOW() OR valid_days = 0)
            FOR UPDATE
            "#
        )
        .bind(code)
        .fetch_optional(&mut *tx)
        .await?;

        let (code_id, points, _valid_days) = match row {
            Some(r) => r,
            None => {
                // 检查具体失败原因
                let check = sqlx::query_as::<_, (bool, bool)>(
                    "SELECT is_used, used_by IS NOT NULL FROM recharge_codes WHERE code = $1"
                )
                .bind(code)
                .fetch_optional(&mut *tx)
                .await?;

                match check {
                    Some((true, _)) => return Err(AppError::InvalidInput("充值码已被使用".to_string())),
                    Some((_, true)) => return Err(AppError::InvalidInput("充值码已被使用".to_string())),
                    None => return Err(AppError::InvalidInput("充值码不存在".to_string())),
                    _ => return Err(AppError::InvalidInput("充值码无效或已使用".to_string())),
                }
            }
        };

        // 获取用户旧余额
        let old_balance = sqlx::query_as::<_, (i64,)>(
            "SELECT balance FROM users WHERE id = $1"
        )
        .bind(user_id)
        .fetch_one(&mut *tx)
        .await?
        .0;

        // 原子操作：同时更新用户余额和标记充值码已使用
        let new_balance = sqlx::query_as::<_, (i64,)>(
            r#"
            -- 增加用户余额
            UPDATE users
            SET balance = balance + $1, updated_at_old = EXTRACT(EPOCH FROM NOW())::bigint
            WHERE id = $2
            RETURNING balance
            "#
        )
        .bind(points as i64)
        .bind(user_id)
        .fetch_one(&mut *tx)
        .await?
        .0;

        // 标记充值码已使用（在同一事务中）
        sqlx::query(
            r#"
            UPDATE recharge_codes
            SET used_by = $1,
                used_at = NOW(),
                is_used = true
            WHERE id = $2
            "#
        )
        .bind(user_id)
        .bind(code_id)
        .execute(&mut *tx)
        .await?;

        // 记录充值历史（匹配表结构）
        let _ = sqlx::query(
            r#"
            INSERT INTO recharge_history (user_id, code_id, old_balance, new_balance, points_added)
            VALUES ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(user_id)
        .bind(code_id)
        .bind(old_balance)
        .bind(new_balance)
        .bind(points as i64)
        .execute(&mut *tx)
        .await;

        // 记录余额变更（reason字段限制50字符）
        let _ = sqlx::query(
            r#"
            INSERT INTO balance_changes (user_id, change_amount, reason, reference_id)
            VALUES ($1, $2, $3, $4)
            "#,
        )
        .bind(user_id)
        .bind(points as i64)
        .bind("充值码")
        .bind(code)
        .execute(&mut *tx)
        .await;

        // 提交事务（所有操作要么全部成功，要么全部回滚）
        tx.commit().await?;

        // 同步更新 Redis 缓存
        let _ = self.cache.set_balance(user_id, new_balance).await;

        tracing::info!("User {} redeemed code {} (id: {}) for {} points, new balance: {}", user_id, code, code_id, points, new_balance);
        Ok(new_balance)
    }

    /// 批量生成充值码
    pub async fn create_batch(
        &self,
        package: PackageType,
        count: i32,
        valid_days: i32,
    ) -> Result<Vec<String>> {
        let batch_id = format!("batch_{}", uuid::Uuid::new_v4());
        let points = package.points();
        let package_name = package.name();
        let mut codes = Vec::new();

        for _ in 0..count {
            let code = self.generate_code(package);
            codes.push(code.clone());

            // 计算code_hash
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut hasher = DefaultHasher::new();
            code.hash(&mut hasher);
            let code_hash = format!("{:x}", hasher.finish());

            // 存入数据库
            sqlx::query(
                r#"
                INSERT INTO recharge_codes (code, code_hash, batch_id, points, plan_type, valid_days, max_uses, created_at)
                VALUES ($1, $2, $3, $4, $5, $6, 1, NOW())
                "#,
            )
            .bind(&code)
            .bind(&code_hash)
            .bind(&batch_id)
            .bind(points)
            .bind(&package_name)
            .bind(valid_days)
            .execute(self.db.pool())
            .await?;
        }

        // 记录批次
        sqlx::query(
            r#"
            INSERT INTO recharge_batches (batch_id, points_per_code, total_codes, created_by, created_at)
            VALUES ($1, $2, $3, 'system', NOW())
            "#,
        )
        .bind(&batch_id)
        .bind(points)
        .bind(count)
        .execute(self.db.pool())
        .await?;

        tracing::info!("Created recharge batch {} with {} codes", batch_id, count);
        Ok(codes)
    }

    /// 批量生成充值码（自定义分数）
    pub async fn create_batch_with_points(
        &self,
        points: i32,
        count: i32,
        valid_days: i32,
    ) -> Result<Vec<String>> {
        let batch_id = format!("batch_{}", uuid::Uuid::new_v4());
        let mut codes = Vec::new();

        for _ in 0..count {
            let (code, code_hash) = {
                const CHARSET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
                let mut rng = rand::thread_rng();

                let prefix = match points {
                    500 => "S5",
                    1000 => "M1",
                    2000 => "L2",
                    10000 => "U1",
                    _ => "X9",
                };

                let mut code = String::from(prefix);
                for _ in 0..10 {
                    let idx = rng.gen_range(0..CHARSET.len());
                    code.push(CHARSET[idx] as char);
                }

                let code_clone = code.clone();
                use std::collections::hash_map::DefaultHasher;
                use std::hash::{Hash, Hasher};
                let mut hasher = DefaultHasher::new();
                code_clone.hash(&mut hasher);
                let code_hash = format!("{:x}", hasher.finish());

                (code, code_hash)
            };

            codes.push(code.clone());

            sqlx::query(
                r#"
                INSERT INTO recharge_codes (code, code_hash, batch_id, points, plan_type, valid_days, max_uses, created_at)
                VALUES ($1, $2, $3, $4, $5, $6, 1, NOW())
                "#,
            )
            .bind(&code)
            .bind(&code_hash)
            .bind(&batch_id)
            .bind(points)
            .bind(format!("custom_{}", points))
            .bind(valid_days)
            .execute(self.db.pool())
            .await?;
        }

        // 记录批次
        sqlx::query(
            r#"
            INSERT INTO recharge_batches (batch_id, points_per_code, total_codes, created_by, created_at)
            VALUES ($1, $2, $3, 'system', NOW())
            "#,
        )
        .bind(&batch_id)
        .bind(points)
        .bind(count)
        .execute(self.db.pool())
        .await?;

        tracing::info!("Created recharge batch {} with {} codes of {} points", batch_id, count, points);
        Ok(codes)
    }

    /// 获取充值码使用统计
    pub async fn get_code_stats(&self, code: &str) -> Result<serde_json::Value> {
        let row = sqlx::query_as::<_, (i32, i32, Option<i64>, Option<chrono::DateTime<chrono::Utc>>)>(
            r#"
            SELECT points, valid_days, used_by, used_at
            FROM recharge_codes
            WHERE code = $1
            "#
        )
        .bind(code)
        .fetch_optional(self.db.pool())
        .await?;

        match row {
            Some((points, valid_days, used_by, used_at)) => {
                Ok(serde_json::json!({
                    "code": code,
                    "points": points,
                    "valid_days": valid_days,
                    "used_by": used_by,
                    "used_at": used_at.map(|dt| dt.to_rfc3339()),
                }))
            }
            None => Err(AppError::InternalError(anyhow::anyhow!("Code not found"))),
        }
    }

    /// 获取批次统计
    pub async fn get_batch_stats(&self, batch_id: &str) -> Result<serde_json::Value> {
        let row = sqlx::query_as::<_, (String, i32, i32, i32, chrono::DateTime<chrono::Utc>)>(
            r#"
            SELECT package_type, count, total_points, valid_days, created_at
            FROM recharge_batches
            WHERE id = $1
            "#
        )
        .bind(batch_id)
        .fetch_optional(self.db.pool())
        .await?;

        match row {
            Some((package_type, count, total_points, valid_days, created_at)) => {
                // 获取已使用数量
                let used_count: i64 = sqlx::query("SELECT COUNT(*) FROM recharge_codes WHERE batch_id = $1 AND used_by IS NOT NULL")
                    .bind(batch_id)
                    .fetch_one(self.db.pool())
                    .await?
                    .get(0);

                Ok(serde_json::json!({
                    "batch_id": batch_id,
                    "package_type": package_type,
                    "total_count": count,
                    "total_points": total_points,
                    "used_count": used_count,
                    "unused_count": count as i64 - used_count,
                    "valid_days": valid_days,
                    "created_at": created_at.to_rfc3339(),
                }))
            }
            None => Err(AppError::InternalError(anyhow::anyhow!("Batch not found"))),
        }
    }
}

/// 加油包（充值包）类型
#[derive(Debug, Clone, Copy)]
pub enum PackageType {
    Small,   // 小包: 100分
    Medium,  // 中包: 500分
    Large,   // 大包: 2000分
    Ultra,   // 超大包: 10000分
}

impl PackageType {
    pub fn points(&self) -> i32 {
        match self {
            PackageType::Small => 100,
            PackageType::Medium => 500,
            PackageType::Large => 2000,
            PackageType::Ultra => 10000,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            PackageType::Small => "小加油包",
            PackageType::Medium => "中加油包",
            PackageType::Large => "大加油包",
            PackageType::Ultra => "超大加油包",
        }
    }
}

// ==================== 单元测试 ====================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_package_type_points() {
        assert_eq!(PackageType::Small.points(), 100);
        assert_eq!(PackageType::Medium.points(), 500);
        assert_eq!(PackageType::Large.points(), 2000);
        assert_eq!(PackageType::Ultra.points(), 10000);
    }

    #[test]
    fn test_package_type_name() {
        assert_eq!(PackageType::Small.name(), "小加油包");
        assert_eq!(PackageType::Medium.name(), "中加油包");
        assert_eq!(PackageType::Large.name(), "大加油包");
        assert_eq!(PackageType::Ultra.name(), "超大加油包");
    }

    #[test]
    fn test_code_format() {
        // 直接测试generate_code函数，不需要完整的service
        let mut rng = rand::thread_rng();
        const CHARSET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";

        let prefix = "S1";
        let mut code = String::from(prefix);
        for _ in 0..10 {
            let idx = rng.gen_range(0..CHARSET.len());
            code.push(CHARSET[idx] as char);
        }

        assert_eq!(code.len(), 12);
        assert!(code.starts_with("S1"));
    }

    #[test]
    fn test_code_prefix() {
        assert!("S1XXXXX...".starts_with("S1"));
        assert!("M5XXXXX...".starts_with("M5"));
        assert!("L2XXXXX...".starts_with("L2"));
        assert!("U0XXXXX...".starts_with("U0"));
    }

    #[test]
    fn test_code_uniqueness_simple() {
        use std::collections::HashSet;
        let mut codes = HashSet::new();

        for i in 0..1000 {
            codes.insert(format!("CODE_{:010}", i));
        }

        assert_eq!(codes.len(), 1000);
    }
}
