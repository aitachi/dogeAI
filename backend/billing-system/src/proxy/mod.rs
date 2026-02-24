use crate::cache::Cache;
use crate::key_manager::KeyManager;
use crate::models::{ChatRequest, AppError, Result};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};

/// 模型健康状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelHealth {
    pub model: String,
    pub available: bool,
    pub consecutive_failures: i32,
    pub last_check: i64,
    pub avg_latency_ms: f64,
}

/// 模型池 - 支持多Key和Claude Code协议
#[derive(Clone)]
pub struct ModelPool {
    cache: Arc<Cache>,
    key_manager: Arc<KeyManager>,
    health_status: Arc<RwLock<HashMap<String, ModelHealth>>>,
}

impl ModelPool {
    pub fn new(cache: Arc<Cache>, key_manager: Arc<KeyManager>) -> Self {
        ModelPool {
            cache,
            key_manager,
            health_status: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 转发聊天请求（支持故障转移）
    pub async fn forward_chat(&self, model: &str, request: ChatRequest) -> Result<serde_json::Value> {
        let mut current_provider: Option<String> = None;
        let mut attempt_count = 0;
        const MAX_ATTEMPTS: usize = 3;

        loop {
            attempt_count += 1;

            // 选择key
            let key_mapping = if let Some(ref provider) = current_provider {
                match self.key_manager.get_failover_key(provider, model).await {
                    Some(k) => {
                        tracing::info!("Attempt {}: Using failover key: {} (provider: {})",
                            attempt_count, k.key_name, k.provider);
                        k
                    }
                    None => {
                        tracing::error!("Attempt {}: No failover key available for model: {}", attempt_count, model);
                        return Err(AppError::InternalError(anyhow::anyhow!(
                            "No available key for model: {} after {} attempts", model, attempt_count
                        )));
                    }
                }
            } else {
                match self.key_manager.select_key_for_model(model).await {
                    Some(k) => {
                        tracing::info!("Attempt {}: Using primary key: {} (provider: {})",
                            attempt_count, k.key_name, k.provider);
                        k
                    }
                    None => {
                        return Err(AppError::InternalError(anyhow::anyhow!("No available key for model: {}", model)));
                    }
                }
            };

            let key_id = key_mapping.key_id;
            let provider = key_mapping.provider.clone();
            current_provider = Some(provider.clone());

            // 增加并发计数
            let _ = self.key_manager.increment_concurrent(key_id).await;

            // 尝试请求
            let result = self.do_forward_chat(&key_mapping, model, &request).await;

            // 减少并发计数
            let _ = self.key_manager.decrement_concurrent(key_id).await;

            match &result {
                Ok(_) => {
                    let _ = self.key_manager.record_success(key_id).await;
                    tracing::info!("Attempt {}: Request succeeded with provider: {}", attempt_count, provider);
                    return result;
                }
                Err(e) => {
                    let _ = self.key_manager.record_failure(key_id, &format!("{:?}", e)).await;
                    tracing::warn!("Attempt {}: Request failed with provider: {}, error: {:?}",
                        attempt_count, provider, e);

                    if attempt_count >= MAX_ATTEMPTS {
                        tracing::error!("All {} attempts failed for model: {}", MAX_ATTEMPTS, model);
                        return Err(AppError::InternalError(anyhow::anyhow!(
                            "All {} attempts failed for model: {}. Last error: {:?}", MAX_ATTEMPTS, model, e
                        )));
                    }
                }
            }
        }
    }

    /// 执行实际的聊天请求转发 (使用reqwest)
    async fn do_forward_chat(&self, key_mapping: &crate::key_manager::ModelKeyMapping, model: &str, request: &ChatRequest) -> Result<serde_json::Value> {
        let start = std::time::Instant::now();

        let (response_text, latency) = if key_mapping.protocol == "claude_code" {
            let resp = self.send_claude_code_request(key_mapping, model, request).await?;
            let latency = start.elapsed().as_millis() as f64;
            (resp, latency)
        } else {
            let resp = self.send_openai_request(key_mapping, model, request).await?;
            let latency = start.elapsed().as_millis() as f64;
            (resp, latency)
        };

        // 解析响应
        let response_json: serde_json::Value = serde_json::from_str(&response_text)
            .map_err(|e| AppError::InternalError(anyhow::anyhow!("Failed to parse response: {}", e)))?;

        // 更新健康状态
        let _ = self.update_health(model, true, latency).await;

        Ok(response_json)
    }

    /// 发送Claude Code协议请求 (使用reqwest)
    async fn send_claude_code_request(
        &self,
        key_mapping: &crate::key_manager::ModelKeyMapping,
        model: &str,
        request: &ChatRequest,
    ) -> Result<String> {
        use crate::models::ChatMessage;

        // Claude Code协议使用anthropic格式
        let claude_model = match model.to_lowercase().as_str() {
            m if m.contains("opus") || m.contains("glm-4.7") || m.contains("glm-plus") => "claude-opus-4",
            m if m.contains("sonnet") => "claude-sonnet-4",
            _ => "claude-opus-4",
        };

        let messages: Vec<serde_json::Value> = request.messages.iter()
            .map(|m: &ChatMessage| {
                if m.role == "system" {
                    serde_json::json!({"type": "text", "text": m.content})
                } else {
                    serde_json::json!({
                        "role": m.role,
                        "content": [{"type": "text", "text": m.content}]
                    })
                }
            })
            .collect();

        let body = serde_json::json!({
            "model": claude_model,
            "max_tokens": request.max_tokens.unwrap_or(4096),
            "messages": messages,
            "stream": false,
            "temperature": request.temperature.unwrap_or(0.7),
        });

        let url = format!("{}/v1/messages", key_mapping.base_url);

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .map_err(|e| AppError::InternalError(anyhow::anyhow!("Failed to build client: {}", e)))?;

        let response = client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("x-api-key", &key_mapping.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&body)
            .send()
            .await
            .map_err(|e| AppError::InternalError(anyhow::anyhow!("HTTP request failed: {}", e)))?;

        if response.status().is_success() {
            response.text().await
                .map_err(|e| AppError::InternalError(anyhow::anyhow!("Failed to read response: {}", e)))
        } else {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            Err(AppError::InternalError(anyhow::anyhow!("HTTP {} from model: {}, body: {}", status, model, text)))
        }
    }

    /// 发送OpenAI协议请求 (使用reqwest)
    async fn send_openai_request(
        &self,
        key_mapping: &crate::key_manager::ModelKeyMapping,
        model: &str,
        request: &ChatRequest,
    ) -> Result<String> {
        // 模型名称映射
        let actual_model = match key_mapping.provider.as_str() {
            "qwen" => {
                if model.contains("codex") || model.contains("gemini") {
                    "qwen-coder-plus"
                } else if model.contains("turbo") {
                    "qwen-turbo-latest"
                } else {
                    "qwen-coder-plus"
                }
            }
            "deepseek" => {
                if model.contains("coder") {
                    "deepseek-coder"
                } else {
                    "deepseek-chat"
                }
            }
            _ => model,
        };

        let body = serde_json::json!({
            "model": actual_model,
            "messages": request.messages,
            "stream": false,
            "max_tokens": request.max_tokens.unwrap_or(2000),
            "temperature": request.temperature.unwrap_or(0.7),
        });

        let url = if key_mapping.base_url.contains("compatible-mode") {
            format!("{}/chat/completions", key_mapping.base_url)
        } else {
            format!("{}/v1/chat/completions", key_mapping.base_url)
        };

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .map_err(|e| AppError::InternalError(anyhow::anyhow!("Failed to build client: {}", e)))?;

        let response = client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", key_mapping.api_key))
            .json(&body)
            .send()
            .await
            .map_err(|e| AppError::InternalError(anyhow::anyhow!("HTTP request failed: {}", e)))?;

        if response.status().is_success() {
            response.text().await
                .map_err(|e| AppError::InternalError(anyhow::anyhow!("Failed to read response: {}", e)))
        } else {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            Err(AppError::InternalError(anyhow::anyhow!("HTTP {} from model: {}, body: {}", status, model, text)))
        }
    }

    /// 更新健康状态
    async fn update_health(&self, model: &str, available: bool, latency: f64) -> Result<()> {
        let mut health = self.health_status.write().await;
        let entry = health.entry(model.to_string()).or_insert(ModelHealth {
            model: model.to_string(),
            available: true,
            consecutive_failures: 0,
            last_check: chrono::Utc::now().timestamp(),
            avg_latency_ms: latency,
        });

        entry.last_check = chrono::Utc::now().timestamp();

        if available {
            entry.consecutive_failures = 0;
            entry.available = true;
            entry.avg_latency_ms = entry.avg_latency_ms * 0.8 + latency * 0.2;
        } else {
            entry.consecutive_failures += 1;
            if entry.consecutive_failures >= 3 {
                entry.available = false;
            }
        }

        Ok(())
    }

    /// 获取所有模型健康状态
    pub async fn get_all_health(&self) -> Vec<ModelHealth> {
        self.health_status.read().await.values().cloned().collect()
    }

    /// 获取所有模型名称
    pub fn get_model_names(&self) -> Vec<String> {
        vec![
            "glm-4-plus".to_string(),
            "glm-4-flashx".to_string(),
            "glm-4-flash".to_string(),
            "qwen-coder".to_string(),
            "qwen-turbo".to_string(),
            "deepseek-chat".to_string(),
            "deepseek-coder".to_string(),
        ]
    }

    /// 检查单个模型健康状态
    pub async fn check_model_health(&self, model: &str) -> Result<ModelHealth> {
        if let Some(_) = self.key_manager.select_key_for_model(model).await {
            Ok(ModelHealth {
                model: model.to_string(),
                available: true,
                consecutive_failures: 0,
                last_check: chrono::Utc::now().timestamp(),
                avg_latency_ms: 0.0,
            })
        } else {
            Ok(ModelHealth {
                model: model.to_string(),
                available: false,
                consecutive_failures: 1,
                last_check: chrono::Utc::now().timestamp(),
                avg_latency_ms: 0.0,
            })
        }
    }

    /// 手动触发健康检查 - 所有模型
    pub async fn health_check_all(&self) {
        for model in self.get_model_names() {
            let _ = self.check_model_health(&model).await;
        }
    }

    /// 启动后台健康检查任务
    pub fn start_health_checker(self: Arc<Self>) {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
            loop {
                interval.tick().await;
                self.health_check_all().await;
            }
        });
    }
}
