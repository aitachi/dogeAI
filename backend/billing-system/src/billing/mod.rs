use crate::cache::Cache;
use crate::database::Database;
use crate::models::{BillingRule, AppError, Result};
use std::sync::Arc;
use tokio::sync::mpsc;

#[derive(Clone)]
pub struct BillingService {
    db: Arc<Database>,
    cache: Arc<Cache>,
    sender: mpsc::Sender<BillingTask>,
}

#[derive(Debug, Clone)]
pub struct BillingTask {
    pub user_id: i64,
    pub request_id: String,
    pub model: String,
    pub input_tokens: i32,
    pub output_tokens: i32,
}

impl BillingService {
    pub async fn new(db: Arc<Database>, cache: Arc<Cache>) -> Result<Self> {
        let (sender, mut receiver) = mpsc::channel(10000);
        let db_clone = db.clone();

        // 启动后台处理任务
        tokio::spawn(async move {
            let mut buffer = Vec::new();
            let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(100));

            loop {
                tokio::select! {
                    Some(task) = receiver.recv() => {
                        buffer.push(task);
                        if buffer.len() >= 1000 {
                            Self::flush_buffer(db_clone.clone(), buffer).await;
                            buffer = Vec::new();
                        }
                    }
                    _ = interval.tick() => {
                        if !buffer.is_empty() {
                            let tasks = std::mem::take(&mut buffer);
                            Self::flush_buffer(db_clone.clone(), tasks).await;
                        }
                    }
                }
            }
        });

        Ok(BillingService { db, cache, sender })
    }

    async fn flush_buffer(db: Arc<Database>, tasks: Vec<BillingTask>) {
        for task in tasks {
            let rule = match BillingRule::get(&task.model) {
                Some(r) => r,
                None => continue,
            };
            let cost = rule.calculate_cost(task.input_tokens + task.output_tokens);

            let _ = db.create_billing_record(
                task.user_id,
                &task.request_id,
                &task.model,
                task.input_tokens,
                task.output_tokens,
                cost,
            ).await;
        }
    }

    /// 计算费用
    pub fn calculate_cost(&self, model: &str, input_tokens: i32, output_tokens: i32) -> Result<i32> {
        let rule = BillingRule::get(model)
            .ok_or_else(|| AppError::InternalError(anyhow::anyhow!("Unknown model: {}", model)))?;
        Ok(rule.calculate_cost(input_tokens + output_tokens))
    }

    /// 检查用户余额
    pub async fn check_balance(&self, user_id: i64, required: i32) -> Result<bool> {
        match self.cache.get_balance(user_id).await {
            Ok(balance) => {
                Ok(balance >= -30 && balance >= required as i64)
            }
            Err(_) => {
                // 从数据库获取
                let user = self.db.get_user_by_api_key(&format!("user_{}", user_id)).await?;
                let _ = self.cache.set_balance(user_id, user.balance).await;
                Ok(user.balance >= -30 && user.balance >= required as i64)
            }
        }
    }

    /// 预扣费（同步）
    pub async fn pre_deduct(&self, user_id: i64, amount: i32) -> Result<i64> {
        let current_balance = self.cache.get_balance(user_id).await.unwrap_or(0);

        if current_balance < -30 {
            return Err(AppError::InsufficientBalance {
                required: amount as i64,
                balance: current_balance,
            });
        }

        if current_balance < amount as i64 {
            return Err(AppError::InsufficientBalance {
                required: amount as i64,
                balance: current_balance,
            });
        }

        self.cache.deduct_balance(user_id, amount as i64).await
    }

    /// 异步记录计费
    pub async fn record_billing(&self, task: BillingTask) -> Result<()> {
        self.sender
            .send(task)
            .await
            .map_err(|_| AppError::InternalError(anyhow::anyhow!("Billing queue full")))?;
        Ok(())
    }

    /// 退还积分（请求失败时调用）
    pub async fn refund(&self, user_id: i64, amount: i32) -> Result<i64> {
        self.cache.add_balance(user_id, amount as i64).await
    }

    /// 获取用户余额（优先从缓存，缓存miss时从数据库读取并更新缓存）
    pub async fn get_balance(&self, user_id: i64) -> Result<i64> {
        match self.cache.get_balance(user_id).await {
            Ok(balance) => Ok(balance),
            Err(_) => {
                // 从数据库获取
                let row = sqlx::query_as::<_, (i64,)>(
                    "SELECT balance FROM users WHERE id = $1"
                )
                .bind(user_id)
                .fetch_optional(self.db.pool())
                .await?;

                if let Some((balance,)) = row {
                    // 更新缓存
                    let _ = self.cache.set_balance(user_id, balance).await;
                    Ok(balance)
                } else {
                    Err(AppError::UserNotFound)
                }
            }
        }
    }
}
