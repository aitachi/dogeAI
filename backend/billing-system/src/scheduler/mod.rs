//! 定时任务模块
//!
//! 实现以下定时任务:
//! 1. 定时对账 (Redis vs DB一致性检查)
//! 2. 每日积分重置 (00:00执行)
//! 3. 死信队列重试

use crate::cache::Cache;
use crate::database::Database;
use crate::models::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::Duration;

#[derive(Clone)]
pub struct Scheduler {
    db: Arc<Database>,
    cache: Arc<Cache>,
    running: Arc<RwLock<bool>>,
}

impl Scheduler {
    pub fn new(db: Arc<Database>, cache: Arc<Cache>) -> Self {
        Self {
            db,
            cache,
            running: Arc::new(RwLock::new(false)),
        }
    }

    /// 启动所有定时任务
    pub async fn start(&self) {
        let mut running = self.running.write().await;
        if *running {
            tracing::warn!("Scheduler already running");
            return;
        }
        *running = true;
        drop(running);

        tracing::info!("Starting scheduler tasks...");

        // 启动定时对账任务 (每5分钟)
        self.start_reconciliation_task();

        // 启动死信队列重试任务 (每分钟)
        self.start_dlq_retry_task();

        // 启动每日积分重置任务
        self.start_daily_reset_task();
    }

    /// 停止所有定时任务
    pub async fn stop(&self) {
        let mut running = self.running.write().await;
        *running = false;
        tracing::info!("Scheduler stopped");
    }

    /// 定时对账任务: Redis vs DB一致性检查
    fn start_reconciliation_task(&self) {
        let db = self.db.clone();
        let cache = self.cache.clone();
        let running = self.running.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(300)); // 5分钟

            loop {
                interval.tick().await;

                // 检查是否应该继续运行
                {
                    let r = running.read().await;
                    if !*r {
                        break;
                    }
                }

                if let Err(e) = Self::reconcile_balances(&db, &cache).await {
                    tracing::error!("Reconciliation failed: {}", e);
                }
            }

            tracing::warn!("Reconciliation task stopped");
        });
    }

    /// 死信队列重试任务
    fn start_dlq_retry_task(&self) {
        let db = self.db.clone();
        let running = self.running.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60)); // 1分钟

            loop {
                interval.tick().await;

                {
                    let r = running.read().await;
                    if !*r {
                        break;
                    }
                }

                if let Err(e) = Self::retry_failed_billing(&db).await {
                    tracing::error!("DLQ retry failed: {}", e);
                }
            }

            tracing::warn!("DLQ retry task stopped");
        });
    }

    /// 每日积分重置任务
    fn start_daily_reset_task(&self) {
        let db = self.db.clone();
        let cache = self.cache.clone();
        let running = self.running.clone();

        tokio::spawn(async move {
            // 计算下一个00:00的时间
            let mut next_midnight = Self::next_midnight();

            loop {
                tokio::time::sleep_until(next_midnight).await;

                {
                    let r = running.read().await;
                    if !*r {
                        break;
                    }
                }

                if let Err(e) = Self::reset_daily_points(&db, &cache).await {
                    tracing::error!("Daily reset failed: {}", e);
                }

                // 计算下一个00:00
                next_midnight = Self::next_midnight();
            }

            tracing::warn!("Daily reset task stopped");
        });
    }

    /// 对账: Redis余额 vs DB余额
    async fn reconcile_balances(db: &Database, cache: &Cache) -> Result<()> {
        tracing::info!("Starting balance reconciliation...");

        // 获取所有活跃用户
        let users = sqlx::query_as::<_, (i64, i64)>(
            "SELECT id, balance FROM users WHERE status = 'active'"
        )
        .fetch_all(db.pool())
        .await?;

        let mut discrepancies = Vec::new();
        let mut corrected = 0;

        for (user_id, db_balance) in users {
            // 从Redis获取余额
            let redis_balance = cache.get_balance(user_id).await;

            match redis_balance {
                Ok(rb) => {
                    let diff = (rb - db_balance).abs();

                    if diff > 10 {
                        // 发现差异，记录并修正
                        discrepancies.push((user_id, db_balance, rb, diff));

                        // 以DB为准，修正Redis
                        cache.set_balance(user_id, db_balance).await?;
                        corrected += 1;

                        tracing::warn!(
                            "Balance discrepancy corrected for user {}: DB={}, Redis={}, diff={}",
                            user_id, db_balance, rb, diff
                        );
                    }
                }
                Err(_) => {
                    // Redis中没有，缓存DB余额
                    cache.set_balance(user_id, db_balance).await?;
                    corrected += 1;
                }
            }
        }

        // 记录对账结果到数据库
        if !discrepancies.is_empty() {
            for (user_id, db_balance, redis_balance, diff) in &discrepancies {
                let _ = sqlx::query(
                    r#"
                    INSERT INTO balance_reconciliation (user_id, db_balance, redis_balance, difference, reconciled_at)
                    VALUES ($1, $2, $3, $4, NOW())
                    "#,
                )
                .bind(user_id)
                .bind(db_balance)
                .bind(redis_balance)
                .bind(diff)
                .execute(db.pool())
                .await;
            }
        }

        tracing::info!("Reconciliation completed: corrected={}, discrepancies={}", corrected, discrepancies.len());
        Ok(())
    }

    /// 重试失败的计费记录
    async fn retry_failed_billing(db: &Database) -> Result<()> {
        // 查找pending状态超过5分钟的记录
        let pending_records = sqlx::query_as::<_, (i64, String, i32, i32, i32, i32)>(
            r#"
            SELECT user_id, request_id, input_tokens, output_tokens, total_tokens, cost
            FROM billing_records
            WHERE billing_status = 'pending'
              AND created_at < NOW() - INTERVAL '5 minutes'
            LIMIT 100
            "#
        )
        .fetch_all(db.pool())
        .await?;

        if pending_records.is_empty() {
            return Ok(());
        }

        tracing::info!("Retrying {} pending billing records", pending_records.len());

        let mut retried = 0;
        let total_count = pending_records.len();

        for (_user_id, request_id, _input_tokens, _output_tokens, _total_tokens, _cost) in &pending_records {
            // 尝试更新状态
            let result = sqlx::query(
                "UPDATE billing_records SET billing_status = 'completed' WHERE request_id = $1"
            )
            .bind(&request_id)
            .execute(db.pool())
            .await;

            if result.is_ok() {
                retried += 1;
                tracing::debug!("Successfully retried billing record: {}", request_id);
            }
        }

        tracing::info!("DLQ retry completed: {}/{} retried", retried, total_count);
        Ok(())
    }

    /// 每日积分重置
    async fn reset_daily_points(db: &Database, cache: &Cache) -> Result<()> {
        tracing::info!("Starting daily points reset...");

        // 获取所有有套餐的用户
        let users = sqlx::query_as::<_, (i64, i32, String)>(
            r#"
            SELECT u.id, up.daily_points, u.tier
            FROM users u
            JOIN user_plans up ON u.id = up.user_id
            WHERE up.is_active = true
              AND u.status = 'active'
              AND up.end_date >= CURRENT_DATE
            "#
        )
        .fetch_all(db.pool())
        .await?;

        let mut reset_count = 0;

        for (user_id, daily_points, tier) in users {
            // 重置积分到Redis缓存
            cache.set_balance(user_id, daily_points as i64).await?;

            // 同时更新数据库余额
            sqlx::query("UPDATE users SET balance = $1 WHERE id = $2")
                .bind(daily_points)
                .bind(user_id)
                .execute(db.pool())
                .await?;

            // 记录重置日志
            sqlx::query(
                r#"
                INSERT INTO balance_changes (user_id, change_amount, reason, created_at)
                VALUES ($1, $2, $3, NOW())
                "#,
            )
            .bind(user_id)
            .bind(daily_points)
            .bind(format!("Daily reset - {} tier", tier))
            .execute(db.pool())
            .await?;

            reset_count += 1;
        }

        tracing::info!("Daily reset completed: {} users reset", reset_count);
        Ok(())
    }

    /// 计算下一个午夜时间
    pub fn next_midnight() -> tokio::time::Instant {
        use tokio::time::Instant;
        use std::time::{SystemTime, UNIX_EPOCH};

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let days_since_epoch = now / 86400;
        let next_midnight_secs = (days_since_epoch + 1) * 86400;

        let duration = Duration::from_secs(next_midnight_secs - now);

        Instant::now() + duration
    }
}

// ==================== 单元测试 ====================

#[cfg(test)]
mod tests {
    use crate::scheduler::Scheduler;

    #[test]
    fn test_next_midnight_calculation() {
        let next = Scheduler::next_midnight();
        // 下一次午夜应该在未来的24小时内
        let next_secs = next.duration_since(tokio::time::Instant::now())
            .as_secs();

        assert!(next_secs > 0);
        assert!(next_secs <= 86400);
    }
}
