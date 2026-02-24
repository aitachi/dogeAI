# Claude Code 兼容改造 - 前后对比

## 一、认证方式对比

### 改造前

```rust
// 只支持 Authorization Bearer
let auth_header = req.headers()
    .get("Authorization")
    .and_then(|h| h.to_str().ok())
    .ok_or_else(|| AuthError::InvalidToken("缺少Authorization头".to_string()))?;

if !auth_header.starts_with("Bearer ") {
    return Err(AuthError::InvalidToken("无效的Authorization格式".to_string()));
}
let token = auth_header[7..].to_string();
```

### 改造后

```rust
// 1. 优先使用 x-api-key (Claude Code 标准)
let token = if let Some(api_key) = req.headers()
    .get("x-api-key")
    .and_then(|h| h.to_str().ok())
{
    api_key.to_string()
}
// 2. 回退到 Authorization Bearer (兼容 OpenAI 客户端)
else if let Some(auth_header) = req.headers()
    .get("Authorization")
    .and_then(|h| h.to_str().ok())
{
    if !auth_header.starts_with("Bearer ") {
        return Err(AnthropicErrorResponse { ... });
    }
    auth_header[7..].to_string()
}
else {
    return Err(AnthropicErrorResponse { ... });
};
```

---

## 二、错误响应格式对比

### 改造前

```json
{
  "error": "invalid_token",
  "message": "Token无效",
  "timestamp": "2026-02-10T12:00:00Z"
}
```

### 改造后 (Anthropic 标准)

```json
{
  "type": "error",
  "error": {
    "type": "authentication_error",
    "message": "Invalid API key"
  }
}
```

---

## 三、响应格式对比

### 改造前 (/v1/messages)

```json
{
  "id": "msg_123",
  "type": "message",
  "role": "assistant",
  "content": [
    {
      "type": "text",
      "text": "Hello!"
    }
  ],
  "model": "claude-sonnet-4-20250514",
  "stop_reason": "end_turn",
  "usage": {
    "input_tokens": 10,
    "output_tokens": 25
  }
}
```

### 改造后 (增加 _request_id 字段等)

```json
{
  "id": "msg_abc123xyz",
  "type": "message",
  "role": "assistant",
  "content": [
    {
      "type": "text",
      "text": "Hello!"
    }
  ],
  "model": "claude-sonnet-4-20250514",
  "stop_reason": "end_turn",
  "stop_sequence": null,
  "usage": {
    "input_tokens": 10,
    "output_tokens": 25
  },
  "_request_id": "uuid-v4"
}
```

---

## 四、模型映射扩展

### 改造前

```rust
match model {
    "claude-opus-4-20250514" => "opus".to_string(),
    "claude-sonnet-4-20250514" => "sonnet".to_string(),
    "claude-haiku-4-20250514" => "haiku".to_string(),
    _ => "sonnet".to_string(),
}
```

### 改造后

```rust
match base_model {
    // Claude 4 系列
    "claude-opus-4-6" | "claude-opus-4-5-20250929"
        | "claude-opus-4-20250514" | "claude-opus-4" => "opus".to_string(),

    "claude-sonnet-4-5-20250929" | "claude-sonnet-4-5-20250914"
        | "claude-sonnet-4-20250514" | "claude-sonnet-4" => "sonnet".to_string(),

    "claude-haiku-4-5-20251001" | "claude-haiku-4-20250514"
        | "claude-haiku-4" => "haiku".to_string(),

    // Claude 3 系列
    "claude-3-opus-20240229" => "opus".to_string(),
    "claude-3-5-sonnet-20241022" => "sonnet".to_string(),
    "claude-3-5-haiku-20241022" => "haiku".to_string(),

    // 短名称
    "opus" | "claude-opus" => "opus".to_string(),
    "sonnet" | "claude-sonnet" => "sonnet".to_string(),
    "haiku" | "claude-haiku" => "haiku".to_string(),

    _ => "sonnet".to_string(),
}
```

---

## 五、路由配置对比

### 改造前

```rust
let app = Router::new()
    .route("/v1/messages", post(anthropic_messages_handler))
    .route("/v1/models", get(models_handler))
    .layer(middleware::from_fn_with_state(
        state.clone(),
        jwt_auth_middleware,
    ))
    .with_state(state);
```

### 改造后

```rust
let app = Router::new()
    // Claude Code 专用端点
    .route("/v1/messages", post(anthropic_messages_handler))
    .route("/v1/models", get(models_handler))

    // 兼容 OpenAI 格式
    .route("/v1/chat/completions", post(chat_handler))

    .layer(CorsLayer::new().allow_origin(tower_http::cors::Any))
    .layer(middleware::from_fn_with_state(
        state.clone(),
        claude_auth_middleware,  // 新的认证中间件
    ))
    .with_state(state);
```

---

## 六、数据结构对比

### Anthropic Messages Request (扩展)

#### 改造前
```rust
#[derive(Debug, Deserialize)]
struct AnthropicMessagesRequest {
    model: String,
    messages: Vec<AnthropicMessage>,
    max_tokens: u32,
    stream: bool,
    system: Option<String>,
    temperature: Option<f32>,
    top_p: Option<f32>,
}
```

#### 改造后
```rust
#[derive(Debug, Clone, Deserialize)]
pub struct AnthropicMessagesRequest {
    pub model: String,
    pub messages: Vec<AnthropicMessage>,
    pub max_tokens: u32,
    pub stream: bool,
    pub system: Option<String>,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub top_k: Option<u32>,          // 新增
    pub tools: Option<Vec<Tool>>,     // 新增
    pub tool_choice: Option<ToolChoice>, // 新增
    pub metadata: Option<Metadata>,   // 新增
}
```

### Anthropic Messages Response (扩展)

#### 改造前
```rust
#[derive(Debug, Serialize)]
struct AnthropicMessagesResponse {
    id: String,
    response_type: String,
    role: String,
    content: Vec<AnthropicContentBlock>,
    model: String,
    stop_reason: String,
    usage: AnthropicUsage,
}
```

#### 改造后
```rust
#[derive(Debug, Serialize)]
pub struct AnthropicMessagesResponse {
    pub id: String,
    pub response_type: String,
    pub role: String,
    pub content: Vec<ResponseContentBlock>,
    pub model: String,
    pub stop_reason: String,
    pub stop_sequence: Option<String>,  // 新增
    pub usage: AnthropicUsage,
    pub _pending: Option<String>,       // 新增
    pub _request_id: Option<String>,     // 新增
}
```

---

## 七、关键改进总结

| 方面 | 改进内容 |
|------|----------|
| **认证** | 支持 `x-api-key` 头，优先级高于 `Authorization` |
| **错误格式** | 符合 Anthropic API 规范的 `type` + `error` 结构 |
| **模型支持** | 支持 Claude 4/3 全系列模型名称 |
| **响应字段** | 添加 `stop_sequence`、`_request_id` 等字段 |
| **内容块** | 支持多种内容类型 (text/image/tool_use/tool_result) |
| **中间件** | 新的 `claude_auth_middleware` 替代原 `jwt_auth_middleware` |
| **配置工具** | 一键配置脚本 `setup-claude-code.sh` |
