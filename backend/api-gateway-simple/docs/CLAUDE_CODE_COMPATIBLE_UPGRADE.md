# Claude Code 兼容性改造方案

> **目标**: 让中转系统完全兼容 Claude Code 客户端，使用户可以通过设置 `ANTHROPIC_BASE_URL` 环境变量直接使用中转系统

---

## 一、当前系统分析

### 1.1 现有接口

| 端点 | 方法 | 功能 | 兼容性 |
|------|------|------|--------|
| `/v1/models` | GET | 模型列表 | ✅ 已兼容 |
| `/v1/messages` | POST | Anthropic Messages API | ⚠️ 部分兼容 |
| `/v1/chat/completions` | POST | OpenAI 格式 | ❌ Claude Code 不使用 |

### 1.2 当前问题

1. **认证方式**: 当前支持 JWT 和 `x-api-key`，但验证逻辑需要适配
2. **响应格式**: `/v1/messages` 响应格式需要完全符合 Anthropic API 规范
3. **流式响应**: 未实现 SSE (Server-Sent Events) 流式响应
4. **错误处理**: 错误响应格式需要符合 Anthropic 规范
5. **模型版本**: 需要支持更多 Claude 模型版本标识

---

## 二、改造目标

### 2.1 Claude Code 发送的实际请求

```http
POST /v1/messages HTTP/1.1
Host: api.anthropic.com
x-api-key: sk-ant-api03-...
anthropic-version: 2023-06-01
content-type: application/json

{
  "model": "claude-sonnet-4-20250514",
  "max_tokens": 4096,
  "messages": [
    {"role": "user", "content": "Hello"}
  ]
}
```

### 2.2 期望的响应格式

```json
{
  "id": "msg_123456",
  "type": "message",
  "role": "assistant",
  "content": [
    {
      "type": "text",
      "text": "Hello! How can I help you today?"
    }
  ],
  "model": "claude-sonnet-4-20250514",
  "stop_reason": "end_turn",
  "stop_sequence": null,
  "usage": {
    "input_tokens": 10,
    "output_tokens": 25
  }
}
```

### 2.3 期望的错误格式

```json
{
  "type": "error",
  "error": {
    "type": "invalid_request_error",
    "message": "Invalid request"
  }
}
```

---

## 三、改造实施

### 3.1 新增依赖 (Cargo.toml)

```toml
[dependencies]
# 新增：流式响应支持
async-stream = "0.3"
headers = "0.3"

# 新增：UUID 生成（消息 ID）
uuid = { version = "1.6", features = ["v4", "serde"] }
```

### 3.2 数据结构改造

#### 3.2.1 请求结构（已有，需完善）

```rust
// Anthropic Messages API 请求
#[derive(Debug, Deserialize)]
struct AnthropicMessagesRequest {
    model: String,
    messages: Vec<AnthropicMessage>,
    #[serde(default = "default_anthropic_max_tokens")]
    max_tokens: u32,
    #[serde(default)]
    stream: bool,
    #[serde(default)]
    system: Option<String>,
    #[serde(default)]
    temperature: Option<f32>,
    #[serde(default)]
    top_p: Option<f32>,
    #[serde(default)]
    tools: Option<Vec<Tool>>,
    #[serde(default)]
    tool_choice: Option<ToolChoice>,
}
```

#### 3.2.2 响应结构改造

```rust
// 完整的 Anthropic Messages API 响应
#[derive(Debug, Serialize)]
struct AnthropicMessagesResponse {
    id: String,                    // 消息 ID，格式 msg_xxxxx
    #[serde(rename = "type")]
    response_type: String,         // "message"
    role: String,                  // "assistant"
    content: Vec<ContentBlock>,    // 内容块
    model: String,                 // 模型名称
    stop_reason: String,           // "end_turn", "max_tokens", "stop_sequence"
    stop_sequence: Option<String>, // 停止序列
    usage: AnthropicUsage,         // Token 使用
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
enum ContentBlock {
    Text { #[serde(rename = "type")] content_type: String, text: String },
    Image { #[serde(rename = "type")] content_type: String, source: ImageSource },
    ToolUse { #[serde(rename = "type")] content_type: String, id: String, name: String, input: serde_json::Value },
    ToolResult { #[serde(rename = "type")] content_type: String, tool_use_id: String, content: String },
}

#[derive(Debug, Serialize)]
struct AnthropicUsage {
    input_tokens: u32,
    output_tokens: u32,
}

// 错误响应
#[derive(Debug, Serialize)]
struct AnthropicErrorResponse {
    #[serde(rename = "type")]
    error_type: String,
    error: AnthropicErrorDetail,
}

#[derive(Debug, Serialize)]
struct AnthropicErrorDetail {
    #[serde(rename = "type")]
    detail_type: String,
    message: String,
}
```

### 3.3 认证中间件改造

```rust
// 支持两种认证方式的中间件
async fn claude_auth_middleware(
    State(state): State<AppState>,
    headers: HeaderMap,
    req: Request,
    next: Next,
) -> Result<axum::response::Response, AuthError> {
    // 跳过公开端点
    let path = req.uri().path();
    let public_paths = ["/health", "/v1/models", "/api/apply", "/api/user/login"];
    if public_paths.iter().any(|p| path.starts_with(p)) {
        return Ok(next.run(req).await);
    }

    // 1. 优先使用 x-api-key (Claude Code 标准)
    let api_key = if let Some(key) = headers.get("x-api-key").and_then(|h| h.to_str().ok()) {
        key.to_string()
    }
    // 2. 回退到 Authorization Bearer (兼容 OpenAI 客户端)
    else if let Some(auth) = headers.get("authorization").and_then(|h| h.to_str().ok()) {
        if !auth.starts_with("Bearer ") {
            return Err(AuthError::InvalidToken("无效的Authorization格式".to_string()));
        }
        auth[7..].to_string()
    }
    else {
        return Err(AuthError::InvalidToken("缺少认证信息".to_string()));
    };

    // 验证 API Key（支持两种格式）
    let user_info = state.auth.verify_token(&api_key).await?;

    // 注入用户信息
    let auth_user = AuthenticatedUser {
        user_id: user_info.user_id.clone(),
        username: user_info.username.clone(),
        tier: user_info.tier.clone(),
        scopes: user_info.scopes.clone(),
        balance: user_info.balance,
    };

    req.extensions_mut().insert(auth_user);
    req.extensions_mut().insert(user_info);

    Ok(next.run(req).await)
}
```

### 3.4 主路由改造

```rust
// 在 main() 中更新路由配置
let app = Router::new()
    // Claude Code 专用端点
    .route("/v1/messages", post(anthropic_messages_handler))
    .route("/v1/messages/:message_id", get(get_message_handler))
    .route("/v1/models", get(models_handler))

    // 兼容 OpenAI 格式
    .route("/v1/chat/completions", post(chat_handler))

    // 健康检查
    .route("/health", get(health_handler))

    .layer(CorsLayer::new().allow_origin(tower_http::cors::Any))
    .layer(middleware::from_fn_with_state(
        state.clone(),
        claude_auth_middleware,  // 使用新的认证中间件
    ))
    .with_state(state);
```

### 3.5 模型映射扩展

```rust
// 扩展模型映射，支持更多 Claude 模型标识
fn map_anthropic_model(model: &str) -> String {
    // 移除可能的后缀，如 [1m], [200k] 等
    let base_model = if let Some(bracket_pos) = model.find('[') {
        &model[..bracket_pos]
    } else {
        model
    };

    match base_model {
        // ========== Claude 4 系列 ==========
        "claude-opus-4-6" | "claude-opus-4-6-20250214" => "opus".to_string(),
        "claude-opus-4-5-20250929" => "opus".to_string(),
        "claude-opus-4-5-20250514" => "opus".to_string(),
        "claude-opus-4-20250514" => "opus".to_string(),
        "claude-opus-4" => "opus".to_string(),

        "claude-sonnet-4-5-20250929" => "sonnet".to_string(),
        "claude-sonnet-4-5-20250914" => "sonnet".to_string(),
        "claude-sonnet-4-5-20250514" => "sonnet".to_string(),
        "claude-sonnet-4-20250514" => "sonnet".to_string(),
        "claude-sonnet-4" => "sonnet".to_string(),

        "claude-haiku-4-5-20251001" => "haiku".to_string(),
        "claude-haiku-4-20250514" => "haiku".to_string(),
        "claude-haiku-4" => "haiku".to_string(),

        // ========== Claude 3 系列 ==========
        "claude-3-opus-20240229" => "opus".to_string(),
        "claude-3-5-sonnet-20241022" => "sonnet".to_string(),
        "claude-3-5-sonnet-20240620" => "sonnet".to_string(),
        "claude-3-5-haiku-20241022" => "haiku".to_string(),
        "claude-3-haiku-20240307" => "haiku".to_string(),

        // ========== 短名称 ==========
        "opus" | "claude-opus" => "opus".to_string(),
        "sonnet" | "claude-sonnet" => "sonnet".to_string(),
        "haiku" | "claude-haiku" => "haiku".to_string(),

        // ========== 默认 ==========
        _ => "sonnet".to_string(),
    }
}
```

### 3.6 流式响应支持（可选）

```rust
use async_stream::stream;
use futures_util::stream::Stream;

async fn anthropic_messages_stream_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Extension(user): Extension<AuthenticatedUser>,
    Json(req): Json<AnthropicMessagesRequest>,
) -> Result<impl Stream<Item = String>, AuthError> {
    let pool_id = map_anthropic_model(&req.model);

    // 调用上游 API
    let result = state.pool_manager.call_api(&user.user_id, &req, &pool_id).await?;

    // 生成事件流
    let message_id = format!("msg_{}", Uuid::new_v4());
    let stream = stream! {
        // 事件开始
        yield format!("event: message_start\ndata: {{\"type\":\"message_start\",\"message\":{{\"id\":\"{}\",\"role\":\"assistant\"}}}}\n\n", message_id);

        // 内容块
        yield format!("event: content_block_start\ndata: {{\"type\":\"content_block_start\",\"index\":0,\"content_block\":{{\"type\":\"text\",\"text\":\"\"}}}}\n\n");

        // 文本增量
        for chunk in split_text_chunks(&result.content, 20) {
            yield format!("event: content_block_delta\ndata: {{\"type\":\"content_block_delta\",\"index\":0,\"delta\":{{\"type\":\"text_delta\",\"text\":\"{}\"}}}}\n\n", chunk);
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        // 内容块结束
        yield "event: content_block_stop\ndata: {\"type\":\"content_block_stop\",\"index\":0}\n\n".to_string();

        // 消息结束
        yield format!("event: message_stop\ndata: {{\"type\":\"message_stop\",\"usage\":{{\"input_tokens\":{},\"output_tokens\":{}}}}}\n\n", result.input_tokens, result.output_tokens);
    };

    Ok(stream)
}
```

---

## 四、部署配置

### 4.1 环境变量

```bash
# 数据库
DATABASE_URL=postgresql://postgres:postgres@localhost/api_gateway

# Redis
REDIS_URL=redis://127.0.0.1:6379

# 服务地址
SERVER_ADDR=0.0.0.0:8081

# JWT 密钥
JWT_SECRET=your-secret-key-min-32-bytes

# 上游 API（如果需要）
UPSTREAM_API_URL=https://api.anthropic.com
UPSTREAM_API_KEY=your-upstream-key
```

### 4.2 用户端配置

用户在 Claude Code 中配置：

```bash
# 方式 1: 环境变量
export ANTHROPIC_API_KEY="sk-ant-api03-用户自己的密钥"
export ANTHROPIC_BASE_URL="http://your-server:8081"

# 方式 2: settings.json
{
  "env": {
    "ANTHROPIC_API_KEY": "sk-ant-api03-用户自己的密钥",
    "ANTHROPIC_BASE_URL": "http://your-server:8081"
  }
}
```

---

## 五、测试验证

### 5.1 手动测试

```bash
# 测试模型列表
curl http://localhost:8081/v1/models \
  -H "x-api-key: sk-ant-api03-..."

# 测试消息 API
curl -X POST http://localhost:8081/v1/messages \
  -H "x-api-key: sk-ant-api03-..." \
  -H "anthropic-version: 2023-06-01" \
  -H "content-type: application/json" \
  -d '{
    "model": "claude-sonnet-4-20250514",
    "max_tokens": 100,
    "messages": [{"role": "user", "content": "Hello"}]
  }'
```

### 5.2 Claude Code 测试

```bash
# 配置环境变量
export ANTHROPIC_API_KEY="sk-ant-api03-测试密钥"
export ANTHROPIC_BASE_URL="http://localhost:8081"

# 启动 Claude Code
claude
```

---

## 六、完整代码改造清单

| 文件 | 改造内容 |
|------|----------|
| `src/main.rs` | 更新认证中间件、响应结构、错误处理 |
| `src/models.rs` | 新增完整的 Anthropic API 数据结构 |
| `src/anthropic.rs` | 新增 Anthropic API 专用处理模块 |
| `src/streaming.rs` | 新增 SSE 流式响应支持 |
| `Cargo.toml` | 添加新依赖 |
| `migrations/*.sql` | 数据库迁移（如需要） |

---

## 七、实施步骤

1. **阶段 1**: 基础兼容（当前已部分完成）
   - ✅ 支持 `/v1/models`
   - ✅ 支持 `/v1/messages`
   - ⚠️ 完善 `/v1/messages` 响应格式

2. **阶段 2**: 认证优化
   - 确保 `x-api-key` 认证完全兼容
   - 优化错误响应格式

3. **阶段 3**: 流式响应（可选）
   - 实现 SSE 流式响应
   - 支持实时打字效果

4. **阶段 4**: 完整测试
   - 单元测试
   - 集成测试
   - Claude Code 实际测试

---

*本文档详细说明了将现有中转系统改造为完全兼容 Claude Code 的方案。*
