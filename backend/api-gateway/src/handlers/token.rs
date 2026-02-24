use crate::models::{
    ApiError, TokenQueryRequest, TokenQueryResponse, ChatRequest, ChatResponse,
    ChatUsage, ChatChoice, ChatMessage, UserContext,
};
use crate::middleware::{extract_user_context, AppState};
use axum::{
    extract::{Request, State},
    response::{IntoResponse, Response, sse::Event},
    Json,
};
use uuid::Uuid;

/// Token查询处理器
/// POST /v1/token/query
pub async fn token_query_handler(
    State(state): State<AppState>,
    mut request: Request,
) -> Result<Json<TokenQueryResponse>, ApiError> {
    // 1. 提取用户信息（在消费body之前）
    let user_ctx = extract_user_context(&request)?;

    // 2. 解析请求体 - 使用 hyper 的 Body::collect
    use http_body_util::{BodyExt, Full};
    use hyper::body::Bytes;

    let whole_body = request.body_mut().collect().await
        .map_err(|e| ApiError::InvalidRequest(e.to_string()))?
        .to_bytes();

    let req: TokenQueryRequest = serde_json::from_slice(&whole_body)
        .map_err(|e| ApiError::InvalidRequest(format!("无效的请求体: {}", e)))?;

    // 3. 计算费用
    let cost = state
        .billing
        .calculate_cost(&req.model, req.input_tokens, req.output_tokens)?;

    // 4. 获取余额
    let balance = state.billing.get_balance(user_ctx.user_id).await?;

    // 5. 构造响应
    Ok(Json(TokenQueryResponse {
        status: "ok".to_string(),
        cost,
        balance,
        can_proceed: balance >= cost,
    }))
}

/// 模型列表处理器
/// GET /v1/models
pub async fn models_handler(
    State(state): State<AppState>,
) -> Result<Json<crate::models::ModelsResponse>, ApiError> {
    let models = state.model_service.get_all_models();

    Ok(Json(crate::models::ModelsResponse {
        object: "list".to_string(),
        data: models,
    }))
}

/// 健康检查处理器
/// GET /health
pub async fn health_handler() -> Json<crate::models::HealthResponse> {
    Json(crate::models::HealthResponse {
        status: "healthy".to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        services: crate::models::ServicesHealth {
            database: "ok".to_string(),
            redis: "ok".to_string(),
            upstream: "ok".to_string(),
        },
    })
}

/// 聊天处理器（非流式和流式）
/// POST /v1/chat/completions
pub async fn chat_handler(
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

    let chat_req: ChatRequest = serde_json::from_slice(&whole_body)
        .map_err(|e| ApiError::InvalidRequest(format!("无效的请求体: {}", e)))?;

    // 3. 验证模型
    if !state.model_service.model_exists(&chat_req.model) {
        return Err(ApiError::ModelNotFound(chat_req.model.clone()));
    }

    // 4. 计费预估
    let estimated_cost = state.billing.calculate_cost(
        &chat_req.model,
        100, // 预估输入tokens
        chat_req.max_tokens,
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
    if chat_req.stream {
        // 流式响应
        chat_handler_stream(State(state), user_ctx, chat_req, request).await
    } else {
        // 非流式响应
        chat_handler_non_stream(State(state), user_ctx, chat_req).await
    }
}

/// 非流式聊天处理
async fn chat_handler_non_stream(
    state: State<AppState>,
    user_ctx: UserContext,
    req: ChatRequest,
) -> Result<Response, ApiError> {
    // 1. 转发到上游（这里简化处理，实际应该调用上游API）
    let response_text = forward_to_upstream_non_stream(&req).await?;

    // 2. 计算实际tokens（这里简化，实际应该从上游响应获取）
    let prompt_tokens = estimate_tokens(&req.messages);
    let completion_tokens = estimate_tokens_from_text(&response_text);
    let total_tokens = prompt_tokens + completion_tokens;

    // 3. 计算实际费用
    let actual_cost = state
        .billing
        .calculate_cost(&req.model, prompt_tokens, completion_tokens)?;

    // 4. 扣费
    state.billing.deduct_balance(user_ctx.user_id, actual_cost).await?;

    // 5. 构造响应
    let chat_response = ChatResponse {
        id: Uuid::new_v4().to_string(),
        object: "chat.completion".to_string(),
        created: chrono::Utc::now().timestamp(),
        model: req.model.clone(),
        choices: vec![ChatChoice {
            index: 0,
            message: ChatMessage {
                role: "assistant".to_string(),
                content: response_text,
            },
            finish_reason: "stop".to_string(),
        }],
        usage: ChatUsage {
            prompt_tokens,
            completion_tokens,
            total_tokens,
        },
    };

    // 6. 添加响应头
    let mut response = Json(chat_response).into_response();
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
    headers.insert("X-Processing-Time", "5ms".parse().unwrap());

    Ok(response)
}

/// 流式聊天处理
async fn chat_handler_stream(
    state: State<AppState>,
    user_ctx: UserContext,
    req: ChatRequest,
    original_request: Request,
) -> Result<Response, ApiError> {
    use axum::response::sse;
    use futures::stream::Stream;

    let request_id = Uuid::new_v4().to_string();
    let created = chrono::Utc::now().timestamp();

    // 模拟流式响应
    let model_name = req.model.clone();  // Clone before moving into stream
    type StreamResult = Result<Event, std::io::Error>;
    let stream: futures::stream::BoxStream<'static, StreamResult> = Box::pin(async_stream::try_stream! {
        // 模拟从上游接收的流式数据
        let chunks = vec![
            "Hello",
            " world",
            "!",
            " This",
            " is",
            " a",
            " streaming",
            " response",
            ".",
        ];

        for (i, chunk) in chunks.iter().enumerate() {
            let chunk_data = crate::models::ChatStreamChunk {
                id: request_id.clone(),
                object: "chat.completion.chunk".to_string(),
                created,
                model: model_name.clone(),
                choices: vec![crate::models::StreamChoice {
                    index: 0,
                    delta: crate::models::StreamDelta {
                        content: Some(chunk.to_string()),
                    },
                    finish_reason: if i == chunks.len() - 1 { Some("stop".to_string()) } else { None },
                }],
            };

            let event = Event::default()
                .json_data(chunk_data)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

            yield event;

            // 模拟延迟
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }

        // 发送结束标记
        let event = Event::default().data("[DONE]");
        yield event;
    });

    // 计算tokens和费用（异步）
    let prompt_tokens = estimate_tokens(&req.messages);
    let completion_tokens = 20; // 简化
    let cost = state.billing.calculate_cost(&req.model, prompt_tokens, completion_tokens)?;

    // 异步扣费
    let billing_record = crate::models::BillingRecord {
        user_id: user_ctx.user_id,
        model: req.model.clone(),
        prompt_tokens,
        completion_tokens,
        cost,
        timestamp: chrono::Utc::now(),
    };
    state.billing.record_billing_async(billing_record).await?;

    // 返回SSE流
    Ok(sse::Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(std::time::Duration::from_secs(1))
            .text("keepalive"),
    )
    .into_response())
}

/// 转发到上游（非流式）- 简化实现
async fn forward_to_upstream_non_stream(req: &ChatRequest) -> Result<String, ApiError> {
    // 这里应该实际调用上游API（如Claude API）
    // 为了演示，返回模拟响应

    // 模拟网络延迟
    tokio::time::sleep(std::time::Duration::from_millis(150)).await;

    // 简单模拟响应
    let user_message = req
        .messages
        .iter()
        .filter(|m| m.role == "user")
        .map(|m| m.content.as_str())
        .collect::<Vec<_>>()
        .join(" ");

    Ok(format!(
        "I received your message: '{}'. This is a simulated response from the upstream service.",
        user_message
    ))
}

/// 估算消息的token数（简化版，实际应该使用tiktoken）
fn estimate_tokens(messages: &[crate::models::ChatMessage]) -> u32 {
    let text = messages
        .iter()
        .map(|m| m.content.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    estimate_tokens_from_text(&text)
}

/// 从文本估算token数（简化：1 token ≈ 4 characters）
fn estimate_tokens_from_text(text: &str) -> u32 {
    ((text.len() as f32) / 4.0).ceil() as u32
}
