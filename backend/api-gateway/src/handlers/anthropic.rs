use crate::models::{
    ApiError, AnthropicRequest, AnthropicResponse, AnthropicStreamEvent,
    UserContext,
};
use crate::middleware::{extract_user_context, AppState};
use axum::{
    extract::{Request, State},
    response::{IntoResponse, Response, sse::Event},
    Json,
};
use futures::stream::{Stream, StreamExt};
use std::sync::Arc;
use uuid::Uuid;

/// Anthropic Messages API 处理器（非流式）
/// POST /v1/messages
pub async fn messages_handler(
    State(state): State<AppState>,
    mut request: Request,
) -> Result<Response, ApiError> {
    // 1. 提取用户信息
    let user_ctx = extract_user_context(&request)?;

    // 2. 解析请求体
    use http_body_util::BodyExt;

    let whole_body = request.body_mut().collect().await
        .map_err(|e| ApiError::InvalidRequest(e.to_string()))?
        .to_bytes();

    let anthropic_req: AnthropicRequest = serde_json::from_slice(&whole_body)
        .map_err(|e| ApiError::InvalidRequest(format!("无效的请求体: {}", e)))?;

    // 3. 验证模型
    if !state.model_service.model_exists(&anthropic_req.model) {
        return Err(ApiError::ModelNotFound(anthropic_req.model.clone()));
    }

    // 4. 预估 tokens
    let input_tokens = estimate_input_tokens(&anthropic_req);
    let estimated_output = anthropic_req.max_tokens.min(4096);
    let estimated_cost = state.billing.calculate_cost(
        &anthropic_req.model,
        input_tokens,
        estimated_output,
    )?;

    // 5. 余额检查
    let balance = state.billing.get_balance(user_ctx.user_id).await?;
    if balance < estimated_cost {
        return Err(ApiError::InsufficientBalance {
            required: estimated_cost,
            balance,
        });
    }

    // 6. 根据stream参数选择处理方式
    if anthropic_req.stream {
        // 流式响应
        anthropic_stream_handler_internal(State(state), user_ctx, anthropic_req).await
    } else {
        // 非流式响应
        anthropic_non_stream_handler(State(state), user_ctx, anthropic_req).await
    }
}

/// 非流式 Anthropic Messages 处理
async fn anthropic_non_stream_handler(
    state: State<AppState>,
    user_ctx: UserContext,
    req: AnthropicRequest,
) -> Result<Response, ApiError> {
    // 1. 调用上游 API
    let upstream_response = state.upstream.send_messages(&req).await?;

    // 2. 计算实际费用
    let actual_cost = state.billing.calculate_cost(
        &req.model,
        upstream_response.usage.input_tokens,
        upstream_response.usage.output_tokens,
    )?;

    // 3. 扣费
    state.billing.deduct_balance(user_ctx.user_id, actual_cost).await?;

    // 4. 添加响应头
    let mut response = Json(upstream_response).into_response();
    let headers = response.headers_mut();
    headers.insert("X-Cost", actual_cost.to_string().parse().unwrap());
    headers.insert(
        "X-Balance",
        (state.billing.get_balance(user_ctx.user_id).await? - actual_cost)
            .to_string()
            .parse()
            .unwrap(),
    );
    headers.insert("X-Request-ID", Uuid::new_v4().to_string().parse().unwrap());

    Ok(response)
}

/// 流式 Anthropic Messages 处理
async fn anthropic_stream_handler_internal(
    state: State<AppState>,
    user_ctx: UserContext,
    req: AnthropicRequest,
) -> Result<Response, ApiError> {
    use axum::response::sse;

    // 1. 调用上游流式 API
    let upstream_response = state.upstream.send_messages_stream(&req).await?;

    // 2. 创建流式响应处理器
    let request_id = Uuid::new_v4().to_string();

    // 预估费用（流式响应在结束时扣费）
    let input_tokens = estimate_input_tokens(&req);
    let estimated_cost = state.billing.calculate_cost(
        &req.model,
        input_tokens,
        req.max_tokens.min(4096),
    )?;

    // 3. 转换上游 SSE 流到下游
    type StreamResult = Result<Event, std::io::Error>;
    let stream: futures::stream::BoxStream<'static, StreamResult> = Box::pin(async_stream::try_stream! {
        // 使用 bytes 流处理
        let mut upstream_bytes_stream = upstream_response_bytes_stream(upstream_response).await;

        let mut total_output_tokens = 0u32;

        while let Some(chunk_result) = upstream_bytes_stream.next().await {
            let chunk = chunk_result.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
            // 简单估算输出 tokens（在移动chunk之前）
            let token_estimate = estimate_tokens_from_bytes(&chunk);
            // 转发 SSE 事件
            let event = Event::default().data(chunk);
            yield event;

            // 简单估算输出 tokens（实际应该解析事件）
            total_output_tokens += token_estimate;
        }

        // 发送结束标记
        let event = Event::default().data("event: message_stop\ndata: {}\n\n");
        yield event;

        // 异步记录计费
        let actual_cost = estimated_cost; // 简化，使用预估费用
        let billing_record = crate::models::BillingRecord {
            user_id: user_ctx.user_id,
            model: req.model.clone(),
            prompt_tokens: input_tokens,
            completion_tokens: total_output_tokens,
            cost: actual_cost,
            timestamp: chrono::Utc::now(),
        };

        // 在后台记录（忽略错误）
        let _ = state.billing.record_billing_async(billing_record).await;
    });

    // 4. 返回 SSE 流
    Ok(sse::Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(std::time::Duration::from_secs(1))
            .text("keepalive"),
    )
    .into_response())
}

/// 将上游响应转换为字节流
async fn upstream_response_bytes_stream(
    response: reqwest::Response,
) -> impl Stream<Item = Result<String, ApiError>> {
    use futures::stream::{self, StreamExt};

    // 使用 reqwest 的字节流
    let bytes_stream = response.bytes_stream();

    bytes_stream.map(|result| {
        result
            .map_err(|e| ApiError::Upstream(e.to_string()))
            .and_then(|bytes| {
                String::from_utf8(bytes.to_vec())
                    .map_err(|e| ApiError::InvalidRequest(format!("无效的UTF-8: {}", e)))
            })
    })
}

/// 估算输入 tokens（简化版本）
fn estimate_input_tokens(req: &AnthropicRequest) -> u32 {
    let mut total_chars = 0usize;

    for msg in &req.messages {
        match &msg.content {
            crate::models::AnthropicContent::Simple(text) => {
                total_chars += text.len();
            }
            crate::models::AnthropicContent::Blocks(blocks) => {
                for block in blocks {
                    if let Some(text) = &block.text {
                        total_chars += text.len();
                    }
                }
            }
        }
    }

    // 添加 system prompt 的字符数
    if let Some(system) = &req.system {
        total_chars += system.len();
    }

    // 粗略估算：4 字符 ≈ 1 token
    ((total_chars as f32) / 4.0).ceil() as u32
}

/// 从字节估算 tokens
fn estimate_tokens_from_bytes(text: &str) -> u32 {
    ((text.len() as f32) / 4.0).ceil() as u32
}

/// Anthropic 流式处理器（备用入口）
/// POST /v1/messages with stream=true
pub async fn anthropic_stream_handler(
    State(state): State<AppState>,
    request: Request,
) -> Result<Response, ApiError> {
    // 重用 messages_handler
    messages_handler(State(state), request).await
}
