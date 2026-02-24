use crate::cache::Cache;
use crate::database::Database;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// API Key配置
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ApiKey {
    pub id: i32,
    pub key_name: String,
    pub provider: String,
    pub api_key: String,
    pub base_url: Option<String>,
    pub protocol: String,
    pub max_tasks: i32,
    pub max_concurrent: i32,
    pub priority: i32,
    pub enabled: bool,
    pub owner: Option<String>,
    pub models: Option<Vec<String>>,
    pub usage_count: i64,
    pub last_used_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Key使用状态
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct KeyUsage {
    pub id: i32,
    pub key_id: i32,
    pub active_tasks: i32,
    pub concurrent_requests: i32,
    pub total_requests: i64,
    pub failed_requests: i64,
    pub last_check_at: chrono::DateTime<chrono::Utc>,
    pub is_healthy: bool,
    pub error_message: Option<String>,
}

/// 模型到Key的映射
#[derive(Debug, Clone)]
pub struct ModelKeyMapping {
    pub model: String,
    pub key_id: i32,
    pub key_name: String,
    pub provider: String,
    pub api_key: String,
    pub base_url: String,
    pub protocol: String,
}

/// Key管理器
#[derive(Clone)]
pub struct KeyManager {
    db: Arc<Database>,
    cache: Arc<Cache>,
    keys: Arc<RwLock<HashMap<i32, ApiKey>>>,
    usage: Arc<RwLock<HashMap<i32, KeyUsage>>>,
}

impl KeyManager {
    pub fn new(db: Arc<Database>, cache: Arc<Cache>) -> Self {
        Self {
            db,
            cache,
            keys: Arc::new(RwLock::new(HashMap::new())),
            usage: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 加载所有API Key
    pub async fn load_keys(&self) -> Result<(), sqlx::Error> {
        let pool = self.db.pool();

        let keys = sqlx::query_as::<_, ApiKey>(
            r#"
            SELECT id, key_name, provider, api_key, base_url, protocol,
                   max_tasks, max_concurrent, priority, enabled, owner, models,
                   usage_count, last_used_at
            FROM api_keys WHERE enabled = true
            "#
        )
        .fetch_all(pool)
        .await?;

        let mut keys_map = HashMap::new();
        for key in keys {
            keys_map.insert(key.id, key);
        }

        *self.keys.write().await = keys_map;

        // 加载使用状态
        let usage_records = sqlx::query_as::<_, KeyUsage>(
            "SELECT * FROM key_usage"
        )
        .fetch_all(pool)
        .await?;

        let mut usage_map = HashMap::new();
        for usage in usage_records {
            usage_map.insert(usage.key_id, usage);
        }

        *self.usage.write().await = usage_map;

        tracing::info!("Loaded {} API keys", self.keys.read().await.len());
        Ok(())
    }

    /// 根据模型选择最佳Key
    pub async fn select_key_for_model(&self, model: &str) -> Option<ModelKeyMapping> {
        let keys = self.keys.read().await;
        let usage = self.usage.read().await;

        // 查找支持该模型的keys
        let mut candidates: Vec<&ApiKey> = keys.values()
            .filter(|k| {
                k.enabled &&
                k.models.as_ref().map_or(false, |m| {
                    m.iter().any(|m| {
                        m.to_lowercase() == model.to_lowercase() ||
                        model.to_lowercase().contains(&m.to_lowercase())
                    })
                })
            })
            .collect();

        if candidates.is_empty() {
            // 按provider查找
            candidates = keys.values()
                .filter(|k| {
                    k.enabled &&
                    (k.provider == "glm" && (model.contains("opus") || model.contains("claude") || model.contains("glm"))) ||
                    (k.provider == "qwen" && (model.contains("qwen") || model.contains("codex") || model.contains("gemini"))) ||
                    (k.provider == "deepseek" && model.contains("deepseek"))
                })
                .collect();
        }

        // 按优先级排序，然后按当前负载排序
        candidates.sort_by(|a, b| {
            let usage_a = usage.get(&a.id);
            let usage_b = usage.get(&b.id);

            // 首先按优先级降序
            let priority_cmp = b.priority.cmp(&a.priority);
            if priority_cmp != std::cmp::Ordering::Equal {
                return priority_cmp;
            }

            // 然后按负载排序（选择负载最低的）
            let load_a = usage_a.map_or(0, |u| u.active_tasks + u.concurrent_requests);
            let load_b = usage_b.map_or(0, |u| u.active_tasks + u.concurrent_requests);
            load_a.cmp(&load_b)
        });

        candidates.first().and_then(|k| {
            let u = usage.get(&k.id);
            // 检查是否超过限制
            if let Some(usage) = u {
                if usage.active_tasks >= k.max_tasks || usage.concurrent_requests >= k.max_concurrent {
                    return None;
                }
            }

            Some(ModelKeyMapping {
                model: model.to_string(),
                key_id: k.id,
                key_name: k.key_name.clone(),
                provider: k.provider.clone(),
                api_key: k.api_key.clone(),
                base_url: k.base_url.clone().unwrap_or_else(|| {
                    match k.provider.as_str() {
                        "glm" => "https://open.bigmodel.cn/api/paas/v4/chat/completions".to_string(),
                        "qwen" => "https://dashscope.aliyuncs.com/compatible-mode/v1".to_string(),
                        "deepseek" => "https://api.deepseek.com".to_string(),
                        _ => "".to_string(),
                    }
                }),
                protocol: k.protocol.clone(),
            })
        })
    }

    /// 获取故障转移的key
    pub async fn get_failover_key(&self, failed_provider: &str, model: &str) -> Option<ModelKeyMapping> {
        tracing::warn!("Failover triggered for provider: {}, model: {}", failed_provider, model);

        let keys = self.keys.read().await;
        let usage = self.usage.read().await;

        // 故障转移顺序: glm -> qwen -> deepseek
        // Deepseek有3个key作为储备池
        let providers = match failed_provider {
            "glm" => vec!["qwen", "deepseek"],
            "qwen" => vec!["deepseek", "glm"],
            "deepseek" => vec!["qwen", "glm"],
            _ => vec!["qwen", "deepseek", "glm"],
        };

        for provider in providers {
            // 查找该provider下所有支持该模型的keys
            let mut candidates: Vec<&ApiKey> = keys.values()
                .filter(|k| {
                    k.enabled && k.provider == *provider &&
                    k.models.as_ref().map_or(false, |m| {
                        m.iter().any(|m| model.to_lowercase().contains(&m.to_lowercase()) ||
                                     m.to_lowercase() == model.to_lowercase())
                    })
                })
                .collect();

            if candidates.is_empty() {
                continue;
            }

            // 按负载排序（选择负载最低的）
            candidates.sort_by(|a, b| {
                let load_a = usage.get(&a.id).map_or(0, |u| u.active_tasks + u.concurrent_requests);
                let load_b = usage.get(&b.id).map_or(0, |u| u.active_tasks + u.concurrent_requests);
                load_a.cmp(&load_b)
            });

            let k = candidates.first()?;
            tracing::info!("Failover to provider: {}, key: {} for model: {}", provider, k.key_name, model);

            return Some(ModelKeyMapping {
                model: model.to_string(),
                key_id: k.id,
                key_name: k.key_name.clone(),
                provider: k.provider.clone(),
                api_key: k.api_key.clone(),
                base_url: k.base_url.clone().unwrap_or_else(|| {
                    match k.provider.as_str() {
                        "glm" => "https://open.bigmodel.cn/api/anthropic".to_string(),
                        "qwen" => "https://dashscope.aliyuncs.com/compatible-mode/v1".to_string(),
                        "deepseek" => "https://api.deepseek.com".to_string(),
                        _ => String::new(),
                    }
                }),
                protocol: k.protocol.clone(),
            });
        }

        tracing::error!("No failover key available for model: {}", model);
        None
    }

    /// 增加活动任务计数
    pub async fn increment_active_tasks(&self, key_id: i32) -> Result<(), sqlx::Error> {
        let mut usage = self.usage.write().await;
        if let Some(u) = usage.get_mut(&key_id) {
            u.active_tasks += 1;
            u.last_check_at = chrono::Utc::now();

            // 异步更新数据库
            let pool = self.db.pool().clone();
            let new_count = u.active_tasks;
            tokio::spawn(async move {
                let _ = sqlx::query("UPDATE key_usage SET active_tasks = $1, last_check_at = NOW() WHERE key_id = $2")
                    .bind(new_count)
                    .bind(key_id)
                    .execute(&pool)
                    .await;
            });
        }
        Ok(())
    }

    /// 减少活动任务计数
    pub async fn decrement_active_tasks(&self, key_id: i32) -> Result<(), sqlx::Error> {
        let mut usage = self.usage.write().await;
        if let Some(u) = usage.get_mut(&key_id) {
            u.active_tasks = (u.active_tasks - 1).max(0);
            u.last_check_at = chrono::Utc::now();

            let pool = self.db.pool().clone();
            let new_count = u.active_tasks;
            tokio::spawn(async move {
                let _ = sqlx::query("UPDATE key_usage SET active_tasks = $1, last_check_at = NOW() WHERE key_id = $2")
                    .bind(new_count)
                    .bind(key_id)
                    .execute(&pool)
                    .await;
            });
        }
        Ok(())
    }

    /// 增加并发请求计数
    pub async fn increment_concurrent(&self, key_id: i32) -> Result<(), sqlx::Error> {
        let mut usage = self.usage.write().await;
        if let Some(u) = usage.get_mut(&key_id) {
            u.concurrent_requests += 1;
            u.total_requests += 1;
            u.last_check_at = chrono::Utc::now();

            let pool = self.db.pool().clone();
            let new_concurrent = u.concurrent_requests;
            let new_total = u.total_requests;
            tokio::spawn(async move {
                let _ = sqlx::query("UPDATE key_usage SET concurrent_requests = $1, total_requests = $2, last_check_at = NOW() WHERE key_id = $3")
                    .bind(new_concurrent)
                    .bind(new_total)
                    .bind(key_id)
                    .execute(&pool)
                    .await;
            });
        }
        Ok(())
    }

    /// 减少并发请求计数
    pub async fn decrement_concurrent(&self, key_id: i32) -> Result<(), sqlx::Error> {
        let mut usage = self.usage.write().await;
        if let Some(u) = usage.get_mut(&key_id) {
            u.concurrent_requests = (u.concurrent_requests - 1).max(0);
            u.last_check_at = chrono::Utc::now();

            let pool = self.db.pool().clone();
            let new_count = u.concurrent_requests;
            tokio::spawn(async move {
                let _ = sqlx::query("UPDATE key_usage SET concurrent_requests = $1, last_check_at = NOW() WHERE key_id = $2")
                    .bind(new_count)
                    .bind(key_id)
                    .execute(&pool)
                    .await;
            });
        }
        Ok(())
    }

    /// 记录请求失败
    pub async fn record_failure(&self, key_id: i32, error: &str) -> Result<(), sqlx::Error> {
        let mut usage = self.usage.write().await;
        if let Some(u) = usage.get_mut(&key_id) {
            u.failed_requests += 1;
            u.error_message = Some(error.to_string());
            u.last_check_at = chrono::Utc::now();

            // 连续失败过多时标记为不健康
            if u.failed_requests > 5 {
                u.is_healthy = false;
            }

            let pool = self.db.pool().clone();
            let new_failed = u.failed_requests;
            let is_healthy = u.is_healthy;
            let error_msg = error.to_string();
            tokio::spawn(async move {
                let _ = sqlx::query("UPDATE key_usage SET failed_requests = $1, is_healthy = $2, error_message = $3, last_check_at = NOW() WHERE key_id = $4")
                    .bind(new_failed)
                    .bind(is_healthy)
                    .bind(error_msg)
                    .bind(key_id)
                    .execute(&pool)
                    .await;
            });
        }
        Ok(())
    }

    /// 记录请求成功
    pub async fn record_success(&self, key_id: i32) -> Result<(), sqlx::Error> {
        let mut usage = self.usage.write().await;
        if let Some(u) = usage.get_mut(&key_id) {
            u.is_healthy = true;
            u.error_message = None;
            u.last_check_at = chrono::Utc::now();

            let pool = self.db.pool().clone();
            tokio::spawn(async move {
                let _ = sqlx::query("UPDATE key_usage SET is_healthy = true, error_message = NULL, last_check_at = NOW() WHERE key_id = $1")
                    .bind(key_id)
                    .execute(&pool)
                    .await;
            });
        }
        Ok(())
    }

    /// 获取所有Key状态
    pub async fn get_all_keys_status(&self) -> Vec<(ApiKey, KeyUsage)> {
        let keys = self.keys.read().await;
        let usage = self.usage.read().await;

        keys.values().filter_map(|k| {
            usage.get(&k.id).map(|u| (k.clone(), u.clone()))
        }).collect()
    }

    /// 根据provider获取keys
    pub async fn get_keys_by_provider(&self, provider: &str) -> Vec<ApiKey> {
        let keys = self.keys.read().await;
        keys.values().filter(|k| k.provider == provider).cloned().collect()
    }
}
