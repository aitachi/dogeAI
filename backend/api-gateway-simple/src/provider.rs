// ========== 模型资源池管理系统 ==========
// 功能：基于用户任务序号的智能路由、健康检查、自动故障转移

use serde::{Deserialize, Serialize};
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn, error, debug};
use std::time::{Duration, Instant};

use crate::ChatRequest;

/// 提供商配置
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub provider_id: String,
    pub provider_name: String,
    pub provider_type: String,
    pub base_url: String,
    pub api_key: String,
    pub priority: i32,
    pub max_concurrent: i32,
    pub timeout_ms: i32,
    pub enabled: bool,
    pub health_status: String,
    pub consecutive_failures: i32,
    pub last_health_check: Option<i64>,
}

/// 模型池配置
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModelPoolConfig {
    pub pool_id: String,
    pub pool_name: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub fallback_chain: Vec<String>,
}

/// 路由规则
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RoutingRule {
    pub rule_id: i64,
    pub pool_id: String,
    pub task_sequence_start: i32,
    pub task_sequence_end: i32,
    pub provider_id: String,
    pub actual_model: String,
    pub priority: i32,
    pub enabled: bool,
}

/// API调用结果
#[derive(Clone, Debug)]
pub struct ApiCallResult {
    pub content: String,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub provider_id: String,
    pub actual_model: String,
    pub pool_id: String,
    pub task_sequence: i32,
    pub response_time_ms: u64,
    pub used_fallback: bool,
}

/// 资源池管理器
pub struct ModelPoolManager {
    db: PgPool,
    providers: Arc<RwLock<HashMap<String, ProviderConfig>>>,
    pools: Arc<RwLock<HashMap<String, ModelPoolConfig>>>,
    routing_rules: Arc<RwLock<HashMap<String, Vec<RoutingRule>>>>,
    http_client: reqwest::Client,
    redis_client: redis::Client,
}

/// 健康检查结果
#[derive(Clone, Debug)]
pub struct HealthCheckResult {
    pub provider_id: String,
    pub is_healthy: bool,
    pub response_time_ms: u64,
    pub error_message: Option<String>,
    pub checked_at: i64,
}

impl ModelPoolManager {
    pub async fn new(database_url: &str, redis_url: &str) -> Result<Self, String> {
        let db = PgPoolOptions::new()
            .max_connections(10)
            .connect(database_url)
            .await
            .map_err(|e| format!("数据库连接失败: {}", e))?;

        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(120))
            .pool_max_idle_per_host(10)
            .pool_idle_timeout(Duration::from_secs(90))
            .build()
            .map_err(|e| format!("HTTP客户端创建失败: {}", e))?;

        let redis_client = redis::Client::open(redis_url)
            .map_err(|e| format!("Redis连接失败: {}", e))?;

        let manager = Self {
            db,
            providers: Arc::new(RwLock::new(HashMap::new())),
            pools: Arc::new(RwLock::new(HashMap::new())),
            routing_rules: Arc::new(RwLock::new(HashMap::new())),
            http_client,
            redis_client,
        };

        // 初始加载配置
        manager.reload_config().await?;

        // 启动健康检查任务
        manager.start_health_check_task();

        Ok(manager)
    }

    /// 从数据库重新加载配置
    pub async fn reload_config(&self) -> Result<(), String> {
        // 加载提供商配置
        let providers_rows = sqlx::query_as::<_, (String, String, String, String, String, i32, i32, i32, bool, String, i32, Option<i64>)>(
            "SELECT provider_id, provider_name, provider_type, base_url, api_key_encrypted,
                    priority, max_concurrent, timeout_ms, enabled, health_status,
                    consecutive_failures, last_health_check
             FROM model_providers WHERE enabled = true
             ORDER BY priority ASC"
        )
        .fetch_all(&self.db)
        .await
        .map_err(|e| format!("查询提供商失败: {}", e))?;

        let mut providers = HashMap::new();
        for row in providers_rows {
            let config = ProviderConfig {
                provider_id: row.0,
                provider_name: row.1,
                provider_type: row.2,
                base_url: row.3,
                api_key: row.4,
                priority: row.5,
                max_concurrent: row.6,
                timeout_ms: row.7,
                enabled: row.8,
                health_status: row.9,
                consecutive_failures: row.10,
                last_health_check: row.11,
            };
            providers.insert(config.provider_id.clone(), config);
        }

        // 加载模型池配置
        let pools_rows = sqlx::query_as::<_, (String, String, Option<String>, bool, Vec<String>)>(
            "SELECT pool_id, pool_name, description, enabled, fallback_chain
             FROM model_pools WHERE enabled = true"
        )
        .fetch_all(&self.db)
        .await
        .map_err(|e| format!("查询模型池失败: {}", e))?;

        let mut pools = HashMap::new();
        for row in pools_rows {
            let config = ModelPoolConfig {
                pool_id: row.0,
                pool_name: row.1,
                description: row.2,
                enabled: row.3,
                fallback_chain: row.4,
            };
            pools.insert(config.pool_id.clone(), config);
        }

        // 加载路由规则
        let rules_rows = sqlx::query_as::<_, (i64, String, i32, i32, String, String, i32, bool)>(
            "SELECT rule_id, pool_id, task_sequence_start, task_sequence_end,
                    provider_id, actual_model, priority, enabled
             FROM pool_routing_rules WHERE enabled = true
             ORDER BY pool_id, task_sequence_start, priority"
        )
        .fetch_all(&self.db)
        .await
        .map_err(|e| format!("查询路由规则失败: {}", e))?;

        let mut routing_rules: HashMap<String, Vec<RoutingRule>> = HashMap::new();
        for row in rules_rows {
            let rule = RoutingRule {
                rule_id: row.0,
                pool_id: row.1.clone(),
                task_sequence_start: row.2,
                task_sequence_end: row.3,
                provider_id: row.4,
                actual_model: row.5,
                priority: row.6,
                enabled: row.7,
            };
            routing_rules.entry(row.1).or_insert_with(Vec::new).push(rule);
        }

        // 记录数量（在移动之前）
        let provider_count = providers.len();
        let pool_count = pools.len();
        let rule_count = routing_rules.values().map(|v| v.len()).sum::<usize>();

        *self.providers.write().await = providers;
        *self.pools.write().await = pools;
        *self.routing_rules.write().await = routing_rules;

        info!("资源池配置已重新加载: {} 个提供商, {} 个模型池, {} 条路由规则",
              provider_count, pool_count, rule_count);

        Ok(())
    }

    /// 获取用户的任务序号（从Redis获取，不存在则从数据库获取并缓存）
    async fn get_user_task_sequence(&self, user_id: &str) -> Result<i32, String> {
        // 先尝试从Redis获取
        let mut redis_conn = self.redis_client.get_async_connection()
            .await
            .map_err(|e| format!("Redis连接失败: {}", e))?;

        let redis_key = format!("task_seq:{}", user_id);
        if let Ok(seq) = redis::cmd("GET").arg(&redis_key).query_async::<_, i64>(&mut redis_conn).await {
            return Ok(seq as i32);
        }

        // Redis不存在，从数据库获取
        let seq = sqlx::query_scalar::<_, i64>(
            "SELECT COALESCE(task_sequence, 0) FROM user_task_sequences WHERE user_id = $1"
        )
        .bind(user_id)
        .fetch_one(&self.db)
        .await
        .unwrap_or(0);

        // 缓存到Redis（24小时过期）
        let _ = redis::cmd("SETEX")
            .arg(&redis_key)
            .arg(86400)
            .arg(seq)
            .query_async::<_, ()>(&mut redis_conn)
            .await;

        Ok(seq as i32)
    }

    /// 增加用户任务序号
    async fn increment_user_task_sequence(&self, user_id: &str, pool_id: &str) -> Result<i32, String> {
        let new_seq = sqlx::query_scalar::<_, i64>(
            "INSERT INTO user_task_sequences (user_id, task_sequence, last_updated, pool_id, created_at_old, updated_at_old)
             VALUES ($1, 1, EXTRACT(EPOCH FROM NOW())::BIGINT, $2, EXTRACT(EPOCH FROM NOW())::BIGINT, EXTRACT(EPOCH FROM NOW())::BIGINT)
             ON CONFLICT (user_id) DO UPDATE
             SET task_sequence = user_task_sequences.task_sequence + 1,
                 last_updated = EXTRACT(EPOCH FROM NOW())::BIGINT,
                 pool_id = $2,
                 updated_at_old = EXTRACT(EPOCH FROM NOW())::BIGINT,
                 updated_at = NOW()
             RETURNING task_sequence"
        )
        .bind(user_id)
        .bind(pool_id)
        .fetch_one(&self.db)
        .await
        .map_err(|e| format!("更新任务序号失败: {}", e))?;

        // 更新Redis缓存
        if let Ok(mut redis_conn) = self.redis_client.get_async_connection().await {
            let redis_key = format!("task_seq:{}", user_id);
            let _ = redis::cmd("SETEX")
                .arg(&redis_key)
                .arg(86400)
                .arg(new_seq)
                .query_async::<_, ()>(&mut redis_conn)
                .await;
        }

        Ok(new_seq as i32)
    }

    /// 根据池ID和任务序号获取路由规则
    async fn get_routing_rule(&self, pool_id: &str, task_sequence: i32) -> Option<RoutingRule> {
        let rules = self.routing_rules.read().await;
        if let Some(pool_rules) = rules.get(pool_id) {
            for rule in pool_rules {
                if task_sequence >= rule.task_sequence_start && task_sequence <= rule.task_sequence_end {
                    return Some(rule.clone());
                }
            }
        }
        None
    }

    /// 获取提供商配置
    async fn get_provider(&self, provider_id: &str) -> Option<ProviderConfig> {
        self.providers.read().await.get(provider_id).cloned()
    }

    /// 获取模型池配置
    async fn get_pool(&self, pool_id: &str) -> Option<ModelPoolConfig> {
        self.pools.read().await.get(pool_id).cloned()
    }

    /// 调用API（支持自动故障转移）
    pub async fn call_api(&self, user_id: &str, req: &ChatRequest, pool_alias: &str) -> Result<ApiCallResult, String> {
        let start_time = Instant::now();

        // 将模型别名映射到池ID
        let pool_id = self.map_alias_to_pool(pool_alias);

        // 获取模型池配置
        let pool = self.get_pool(&pool_id).await
            .ok_or_else(|| format!("模型池不存在: {}", pool_id))?;

        if !pool.enabled {
            return Err(format!("模型池已禁用: {}", pool_id));
        }

        // 获取用户当前任务序号
        let task_sequence = self.get_user_task_sequence(user_id).await?;

        // 查找路由规则
        let mut rule = self.get_routing_rule(&pool_id, task_sequence).await
            .ok_or_else(|| format!("没有找到路由规则: pool={}, task_seq={}", pool_id, task_sequence))?;

        // 获取提供商
        let mut provider = self.get_provider(&rule.provider_id).await
            .ok_or_else(|| format!("提供商不存在: {}", rule.provider_id))?;

        // 检查提供商健康状态
        if provider.health_status == "unhealthy" && provider.consecutive_failures >= 5 {
            warn!("提供商 {} 不健康，尝试故障转移", provider.provider_id);
            // 尝试故障转移
            let fallback_result = self.try_fallback(&pool, &rule, user_id, req, &task_sequence).await;
            if fallback_result.is_ok() {
                return fallback_result;
            }
        }

        // 尝试调用
        let call_result = self.call_provider(&provider, &rule.actual_model, req).await;

        match call_result {
            Ok(result) => {
                // 调用成功，更新任务序号
                let new_seq = self.increment_user_task_sequence(user_id, &pool_id).await?;

                // 记录成功统计
                self.record_success(&pool_id, &provider.provider_id, user_id, new_seq,
                                   result.input_tokens, result.output_tokens,
                                   start_time.elapsed().as_millis() as u64).await;

                // 重置提供商失败计数
                self.reset_provider_failures(&provider.provider_id).await;

                Ok(ApiCallResult {
                    content: result.content,
                    input_tokens: result.input_tokens,
                    output_tokens: result.output_tokens,
                    provider_id: provider.provider_id,
                    actual_model: rule.actual_model,
                    pool_id,
                    task_sequence: new_seq,
                    response_time_ms: start_time.elapsed().as_millis() as u64,
                    used_fallback: false,
                })
            }
            Err(e) => {
                error!("提供商 {} 调用失败: {}", provider.provider_id, e);
                self.record_failure(&provider.provider_id, &e).await;

                // 尝试故障转移
                self.try_fallback(&pool, &rule, user_id, req, &task_sequence).await
            }
        }
    }

    /// 尝试故障转移
    async fn try_fallback(&self, pool: &ModelPoolConfig, original_rule: &RoutingRule,
                          user_id: &str, req: &ChatRequest, task_sequence: &i32) -> Result<ApiCallResult, String> {
        let start_time = Instant::now();

        for fallback_provider_id in &pool.fallback_chain {
            let provider = match self.get_provider(fallback_provider_id).await {
                Some(p) => p,
                None => continue,
            };

            if provider.health_status == "unhealthy" {
                continue;
            }

            // 根据提供商类型选择合适的模型
            let actual_model = self.select_fallback_model(&provider, &original_rule.actual_model);

            info!("故障转移到: {} (模型: {})", fallback_provider_id, actual_model);

            match self.call_provider(&provider, &actual_model, req).await {
                Ok(result) => {
                    // 更新任务序号
                    let new_seq = self.increment_user_task_sequence(user_id, &pool.pool_id).await?;

                    // 记录成功
                    self.record_success(&pool.pool_id, &provider.provider_id, user_id, new_seq,
                                       result.input_tokens, result.output_tokens,
                                       start_time.elapsed().as_millis() as u64).await;

                    return Ok(ApiCallResult {
                        content: result.content,
                        input_tokens: result.input_tokens,
                        output_tokens: result.output_tokens,
                        provider_id: provider.provider_id.clone(),
                        actual_model,
                        pool_id: pool.pool_id.clone(),
                        task_sequence: new_seq,
                        response_time_ms: start_time.elapsed().as_millis() as u64,
                        used_fallback: true,
                    });
                }
                Err(e) => {
                    warn!("故障转移到 {} 失败: {}", fallback_provider_id, e);
                    self.record_failure(&provider.provider_id, &e).await;
                    continue;
                }
            }
        }

        Err(format!("所有故障转移均失败，池: {}", pool.pool_id))
    }

    /// 为故障转移选择合适的模型
    fn select_fallback_model(&self, provider: &ProviderConfig, original_model: &str) -> String {
        match provider.provider_type.as_str() {
            "glm" => match original_model {
                "glm-4-plus" => "glm-4-plus".to_string(),
                "glm-4-flashx" => "glm-4-flashx".to_string(),
                _ => "glm-4-plus".to_string(),
            },
            "qwen" => match original_model {
                "qwen-coder-plus-latest" => "qwen-coder-plus-latest".to_string(),
                _ => "qwen-turbo-latest".to_string(),
            },
            "deepseek" => "deepseek-coder".to_string(),
            _ => original_model.to_string(),
        }
    }

    /// 调用单个提供商
    async fn call_provider(&self, provider: &ProviderConfig, actual_model: &str, req: &ChatRequest) -> Result<ProviderCallResult, String> {
        let start_time = Instant::now();

        // 构建请求
        let request_body = self.build_request_body(provider, actual_model, req)?;
        let url = self.build_url(provider, actual_model)?;

        let mut request_builder = self.http_client.post(&url);

        // 添加认证头
        if provider.provider_type == "glm" {
            request_builder = request_builder
                .header("x-api-key", &provider.api_key)
                .header("anthropic-version", "2023-06-01");
        } else {
            request_builder = request_builder
                .header("Authorization", format!("Bearer {}", provider.api_key));
        }

        request_builder = request_builder.header("content-type", "application/json");

        let response = request_builder
            .json(&request_body)
            .timeout(Duration::from_millis(provider.timeout_ms as u64))
            .send()
            .await
            .map_err(|e| format!("请求发送失败: {}", e))?;

        let response_time_ms = start_time.elapsed().as_millis() as u64;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("API返回错误: {} - {}", status, error_text));
        }

        let response_json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("解析响应失败: {}", e))?;

        // 解析响应
        let (content, input_tokens, output_tokens) = self.parse_response(&response_json, &provider.provider_type)?;

        Ok(ProviderCallResult {
            content,
            input_tokens,
            output_tokens,
            response_time_ms,
        })
    }

    /// 构建请求体
    fn build_request_body(&self, provider: &ProviderConfig, model: &str, req: &ChatRequest) -> Result<serde_json::Value, String> {
        Ok(serde_json::json!({
            "model": model,
            "max_tokens": req.max_tokens,
            "messages": req.messages
        }))
    }

    /// 构建请求URL
    fn build_url(&self, provider: &ProviderConfig, model: &str) -> Result<String, String> {
        match provider.provider_type.as_str() {
            "glm" => Ok(format!("{}/v1/messages", provider.base_url)),
            "qwen" | "deepseek" => Ok(format!("{}/chat/completions", provider.base_url)),
            _ => Ok(format!("{}/v1/chat/completions", provider.base_url)),
        }
    }

    /// 解析API响应
    fn parse_response(&self, response: &serde_json::Value, provider_type: &str) -> Result<(String, u32, u32), String> {
        match provider_type {
            "glm" => {
                let content = response["content"][0]["text"]
                    .as_str()
                    .unwrap_or("")
                    .to_string();
                let input_tokens = response["usage"]["input_tokens"]
                    .as_u64()
                    .unwrap_or(0) as u32;
                let output_tokens = response["usage"]["output_tokens"]
                    .as_u64()
                    .unwrap_or(0) as u32;
                Ok((content, input_tokens, output_tokens))
            }
            "qwen" | "deepseek" => {
                let content = response["choices"][0]["message"]["content"]
                    .as_str()
                    .unwrap_or("")
                    .to_string();
                let input_tokens = response["usage"]["prompt_tokens"]
                    .as_u64()
                    .unwrap_or(0) as u32;
                let output_tokens = response["usage"]["completion_tokens"]
                    .as_u64()
                    .unwrap_or(0) as u32;
                Ok((content, input_tokens, output_tokens))
            }
            _ => Err(format!("不支持的提供商类型: {}", provider_type)),
        }
    }

    /// 将模型别名映射到池ID
    fn map_alias_to_pool(&self, alias: &str) -> String {
        match alias {
            "opus" | "gpt-4" => "opus".to_string(),
            "sonnet" | "gpt-3.5-turbo" => "sonnet".to_string(),
            "haiku" | "gpt-3.5-turbo-16k" => "haiku".to_string(),
            "codex" | "code-davinci-002" => "codex".to_string(),
            _ => alias.to_string(),
        }
    }

    /// 记录成功调用
    async fn record_success(&self, pool_id: &str, provider_id: &str, user_id: &str,
                          task_sequence: i32, input_tokens: u32, output_tokens: u32, response_time_ms: u64) {
        let now = chrono::Utc::now().timestamp();
        let _ = sqlx::query(
            "INSERT INTO model_call_stats (pool_id, provider_id, user_id, task_sequence, input_tokens, output_tokens, total_tokens, response_time_ms, success, created_at_old)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, true, $9)"
        )
        .bind(pool_id)
        .bind(provider_id)
        .bind(user_id)
        .bind(task_sequence)
        .bind(input_tokens as i32)
        .bind(output_tokens as i32)
        .bind((input_tokens + output_tokens) as i32)
        .bind(response_time_ms as i64)
        .bind(now)
        .execute(&self.db)
        .await;
    }

    /// 记录失败调用
    async fn record_failure(&self, provider_id: &str, error: &str) {
        let now = chrono::Utc::now().timestamp();
        let _ = sqlx::query(
            "UPDATE model_providers
             SET consecutive_failures = consecutive_failures + 1,
                 last_health_check = $1,
                 updated_at_old = $1,
                 updated_at = NOW()
             WHERE provider_id = $2"
        )
        .bind(now)
        .bind(provider_id)
        .execute(&self.db)
        .await;

        // 如果连续失败超过5次，标记为不健康
        let _ = sqlx::query(
            "UPDATE model_providers
             SET health_status = 'unhealthy'
             WHERE provider_id = $1 AND consecutive_failures >= 5"
        )
        .bind(provider_id)
        .execute(&self.db)
        .await;
    }

    /// 重置提供商失败计数
    async fn reset_provider_failures(&self, provider_id: &str) {
        let now = chrono::Utc::now().timestamp();
        let _ = sqlx::query(
            "UPDATE model_providers
             SET consecutive_failures = 0,
                 health_status = 'healthy',
                 last_health_check = $1,
                 updated_at_old = $1,
                 updated_at = NOW()
             WHERE provider_id = $2"
        )
        .bind(now)
        .bind(provider_id)
        .execute(&self.db)
        .await;

        // 更新内存中的状态
        if let Some(mut provider) = self.providers.write().await.get_mut(provider_id) {
            provider.consecutive_failures = 0;
            provider.health_status = "healthy".to_string();
            provider.last_health_check = Some(now);
        }
    }

    /// 启动健康检查任务
    fn start_health_check_task(&self) {
        let providers = self.providers.clone();
        let db = self.db.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));
            loop {
                interval.tick().await;

                // 获取所有需要检查的提供商
                let provider_ids: Vec<String> = {
                    providers.read().await.keys().cloned().collect()
                };

                for provider_id in &provider_ids {
                    // 这里可以添加实际的健康检查逻辑
                    // 暂时只更新检查时间
                    let now = chrono::Utc::now().timestamp();
                    let _ = sqlx::query(
                        "UPDATE model_providers
                         SET last_health_check = $1
                         WHERE provider_id = $2"
                    )
                    .bind(now)
                    .bind(&provider_id)
                    .execute(&db)
                    .await;
                }

                debug!("健康检查完成，检查了 {} 个提供商", provider_ids.len());
            }
        });
    }

    /// 获取系统状态
    pub async fn get_system_status(&self) -> SystemStatus {
        let providers = self.providers.read().await;
        let pools = self.pools.read().await;

        let healthy_providers = providers.values()
            .filter(|p| p.health_status == "healthy")
            .count();
        let total_providers = providers.len();

        let enabled_pools = pools.values().filter(|p| p.enabled).count();

        SystemStatus {
            total_providers,
            healthy_providers,
            total_pools: pools.len(),
            enabled_pools,
        }
    }

    /// 获取所有模型池列表
    pub async fn list_pools(&self) -> Vec<ModelPoolConfig> {
        self.pools.read().await.values().cloned().collect()
    }
}

/// 提供商调用结果（内部）
#[derive(Clone)]
struct ProviderCallResult {
    content: String,
    input_tokens: u32,
    output_tokens: u32,
    response_time_ms: u64,
}

/// 系统状态
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SystemStatus {
    pub total_providers: usize,
    pub healthy_providers: usize,
    pub total_pools: usize,
    pub enabled_pools: usize,
}
