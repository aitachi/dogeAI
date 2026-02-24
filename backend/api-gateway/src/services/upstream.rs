use crate::models::{
    ApiError, AnthropicRequest, AnthropicResponse, AnthropicStreamEvent,
    AnthropicUsage, to_upstream_model_name,
};
use reqwest::Client;
use std::time::Duration;

/// 上游服务客户端
#[derive(Clone)]
pub struct UpstreamClient {
    client: Client,
    api_url: String,
    api_key: String,
}

impl UpstreamClient {
    pub fn new(api_url: String, api_key: String) -> Result<Self, ApiError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(180))
            .build()
            .map_err(|e| ApiError::Internal)?;

        Ok(Self {
            client,
            api_url,
            api_key,
        })
    }

    /// 发送 Anthropic Messages API 请求（非流式）
    pub async fn send_messages(
        &self,
        request: &AnthropicRequest,
    ) -> Result<AnthropicResponse, ApiError> {
        let url = format!("{}/v1/messages", self.api_url);

        // 转换模型名称
        let mut upstream_request = request.clone();
        upstream_request.model = to_upstream_model_name(&request.model);

        // 确保 max_tokens 设置（Anthropic 要求）
        if upstream_request.max_tokens == 0 {
            upstream_request.max_tokens = 4096;
        }

        let response = self
            .client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&upstream_request)
            .send()
            .await
            .map_err(|e| ApiError::Internal)?;

        let status = response.status();
        let response_text = response
            .text()
            .await
            .map_err(|e| ApiError::Internal)?;

        if !status.is_success() {
            return Err(ApiError::UpstreamError {
                status: status.as_u16(),
                message: response_text,
            });
        }

        serde_json::from_str(&response_text).map_err(|e| {
            ApiError::InvalidRequest(format!("解析上游响应失败: {}", e))
        })
    }

    /// 发送流式请求（返回 SSE 流）
    pub async fn send_messages_stream(
        &self,
        request: &AnthropicRequest,
    ) -> Result<reqwest::Response, ApiError> {
        let url = format!("{}/v1/messages", self.api_url);

        let mut upstream_request = request.clone();
        upstream_request.model = to_upstream_model_name(&request.model);

        if upstream_request.max_tokens == 0 {
            upstream_request.max_tokens = 4096;
        }

        let response = self
            .client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&upstream_request)
            .send()
            .await
            .map_err(|e| ApiError::Internal)?;

        if !response.status().is_success() {
            let status = response.status();
            let response_text = response
                .text()
                .await
                .unwrap_or_else(|_| "无法读取错误响应".to_string());
            return Err(ApiError::UpstreamError {
                status: status.as_u16(),
                message: response_text,
            });
        }

        Ok(response)
    }

    /// 验证 API 连接
    pub async fn health_check(&self) -> Result<bool, ApiError> {
        let url = format!("{}/v1/models", self.api_url);

        let response = self
            .client
            .get(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .send()
            .await
            .map_err(|e| ApiError::Internal)?;

        Ok(response.status().is_success())
    }
}

/// 创建默认的上游客户端
pub fn create_default_upstream() -> Result<UpstreamClient, ApiError> {
    // 从环境变量读取上游配置
    let api_url = std::env::var("UPSTREAM_API_URL")
        .unwrap_or_else(|_| "https://api.anthropic.com".to_string());
    let api_key = std::env::var("UPSTREAM_API_KEY")
        .unwrap_or_else(|_| String::new());

    if api_key.is_empty() {
        tracing::warn!("未设置 UPSTREAM_API_KEY 环境变量，上游调用将失败");
    }

    UpstreamClient::new(api_url, api_key)
}
