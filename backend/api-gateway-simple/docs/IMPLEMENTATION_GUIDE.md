# Claude Code 兼容性改造 - 实施指南

## 一、改造概述

本次改造将现有的 API 网关系统升级为完全兼容 Claude Code 的版本，使用户可以通过设置 `ANTHROPIC_BASE_URL` 环境变量直接使用中转系统。

---

## 二、文件清单

### 新增文件

| 文件 | 说明 |
|------|------|
| `src/anthropic.rs` | Anthropic API 完整兼容模块 |
| `src/main_claude_compatible.rs` | 改造后的主程序（参考） |
| `setup-claude-code.sh` | 一键配置脚本 |
| `CLAUDE_CODE_COMPATIBLE_UPGRADE.md` | 改造方案文档 |
| `CLAUDE_CODE_PROXY_IMPLEMENTATION_ANALYSIS.md` | VisionCoder 方案分析 |

### 需要修改的文件

| 文件 | 改造内容 |
|------|----------|
| `src/main.rs` | 添加 anthropic 模块、更新路由、更新认证中间件 |
| `Cargo.toml` | 依赖已满足，无需修改 |

---

## 三、具体改造步骤

### 步骤 1: 添加 anthropic 模块

```bash
cd /root/api-gateway-simple
```

**操作**: 将 `src/anthropic.rs` 添加到项目中

```rust
// 在 main.rs 顶部添加
mod anthropic;
```

### 步骤 2: 更新 main.rs

#### 2.1 添加导入

```rust
use anthropic::{
    anthropic_messages_handler,
    AnthropicMessagesRequest,
    AnthropicErrorResponse,
};
```

#### 2.2 更新认证中间件

将现有的 `jwt_auth_middleware` 替换为新的 `claude_auth_middleware`：

```rust
async fn claude_auth_middleware(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<axum::response::Response, AnthropicErrorResponse> {
    // 跳过公开端点
    let path = req.uri().path();
    let public_paths = [
        "/health", "/v1/models", "/api/apply",
        "/api/apply/check-availability", "/api/user/login",
        "/api/user/register", "/api/health", "/api/stats",
        "/api/admin/", "/v1/auth/",
    ];
    let is_public = public_paths.iter().any(|p| path.starts_with(p));
    if is_public {
        return Ok(next.run(req).await);
    }

    // 1. 优先使用 x-api-key (Claude Code 标准)
    let token = if let Some(api_key) = req.headers()
        .get("x-api-key")
        .and_then(|h| h.to_str().ok())
    {
        api_key.to_string()
    }
    // 2. 回退到 Authorization Bearer
    else if let Some(auth_header) = req.headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
    {
        if !auth_header.starts_with("Bearer ") {
            return Err(AnthropicErrorResponse {
                error_type: "error".to_string(),
                error: anthropic::AnthropicErrorDetail {
                    detail_type: "invalid_request_error".to_string(),
                    message: "无效的Authorization格式".to_string(),
                },
            });
        }
        auth_header[7..].to_string()
    }
    else {
        return Err(AnthropicErrorResponse {
            error_type: "error".to_string(),
            error: anthropic::AnthropicErrorDetail {
                detail_type: "authentication_error".to_string(),
                message: "缺少认证信息".to_string(),
            },
        });
    };

    // 验证 Token
    let user_info = match state.auth.verify_token(&token).await {
        Ok(user) => user,
        Err(err) => {
            return Err(AnthropicErrorResponse {
                error_type: "error".to_string(),
                error: anthropic::AnthropicErrorDetail {
                    detail_type: match err {
                        AuthError::InvalidToken(_) => "authentication_error",
                        AuthError::TokenExpired => "authentication_error",
                        AuthError::TokenRevoked => "authentication_error",
                        AuthError::Forbidden => "permission_error",
                        _ => "api_error",
                    }.to_string(),
                    message: err.to_string(),
                },
            });
        }
    };

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

#### 2.3 更新路由配置

```rust
let app = Router::new()
    // Claude Code 专用端点
    .route("/v1/messages", post(anthropic_messages_handler))
    .route("/v1/models", get(models_handler))

    // 兼容 OpenAI 格式
    .route("/v1/chat/completions", post(chat_handler))

    // 健康检查
    .route("/health", get(health_handler))

    // ... 其他路由 ...

    .layer(CorsLayer::new().allow_origin(tower_http::cors::Any))
    .layer(middleware::from_fn_with_state(
        state.clone(),
        claude_auth_middleware,  // 使用新的认证中间件
    ))
    .with_state(state);
```

#### 2.4 扩展模型映射

更新 `map_anthropic_model` 函数，添加更多模型支持：

```rust
pub fn map_anthropic_model(model: &str) -> String {
    let base_model = if let Some(bracket_pos) = model.find('[') {
        &model[..bracket_pos]
    } else {
        model
    };

    match base_model {
        // Claude 4 系列
        "claude-opus-4-6" | "claude-opus-4-5-20250929"
            | "claude-opus-4-20250514" | "claude-opus-4" => "opus".to_string(),

        "claude-sonnet-4-5-20250929" | "claude-sonnet-4-20250514"
            | "claude-sonnet-4" => "sonnet".to_string(),

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
}
```

### 步骤 3: 编译和部署

```bash
cd /root/api-gateway-simple

# 编译
cargo build --release

# 或使用 Docker
docker-compose build
docker-compose up -d
```

### 步骤 4: 运行配置脚本

```bash
./setup-claude-code.sh
```

脚本会自动完成：
1. 检查/启动 API 网关
2. 生成/获取 API Key
3. 配置环境变量到 Shell 配置文件
4. 创建 settings.json
5. 测试连接

---

## 四、用户端配置

### 方式 1: 环境变量（推荐）

```bash
export ANTHROPIC_API_KEY="sk-ant-api03-你的密钥"
export ANTHROPIC_BASE_URL="http://你的服务器:8081"
export ANTHROPIC_AUTH_TOKEN="sk-ant-api03-你的密钥"
```

### 方式 2: settings.json

创建 `~/.config/claude-code/settings.json`:

```json
{
  "env": {
    "ANTHROPIC_API_KEY": "sk-ant-api03-你的密钥",
    "ANTHROPIC_BASE_URL": "http://你的服务器:8081",
    "ANTHROPIC_AUTH_TOKEN": "sk-ant-api03-你的密钥"
  }
}
```

---

## 五、API 接口规范

### 5.1 模型列表

```bash
GET /v1/models
x-api-key: sk-ant-api03-...
```

响应:
```json
{
  "data": [
    {
      "id": "claude-opus-4-6",
      "created_at": "2026-02-04T00:00:00Z",
      "display_name": "Claude Opus 4.6",
      "type": "model"
    },
    {
      "id": "claude-sonnet-4-20250514",
      "created_at": "2025-05-14T00:00:00Z",
      "display_name": "Claude Sonnet 4",
      "type": "model"
    }
  ],
  "first_id": "claude-opus-4-6",
  "has_more": false,
  "last_id": "claude-3-haiku-20240307"
}
```

### 5.2 Messages API

```bash
POST /v1/messages
x-api-key: sk-ant-api03-...
anthropic-version: 2023-06-01
content-type: application/json

{
  "model": "claude-sonnet-4-20250514",
  "max_tokens": 4096,
  "messages": [
    {"role": "user", "content": "Hello!"}
  ]
}
```

响应:
```json
{
  "id": "msg_abc123",
  "type": "message",
  "role": "assistant",
  "content": [
    {
      "type": "text",
      "text": "Hello! How can I help you?"
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

### 5.3 错误响应格式

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

## 六、测试验证

### 手动测试

```bash
# 1. 健康检查
curl http://localhost:8081/health

# 2. 模型列表
curl http://localhost:8081/v1/models \
  -H "x-api-key: sk-ant-api03-..."

# 3. Messages API
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

### Claude Code 测试

```bash
# 配置环境变量后
export ANTHROPIC_API_KEY="sk-ant-api03-..."
export ANTHROPIC_BASE_URL="http://localhost:8081"

# 启动 Claude Code
claude
```

---

## 七、故障排查

### 问题 1: 认证失败

**原因**: API Key 无效或格式不正确

**解决**:
```bash
# 检查 API Key
psql -h localhost -U postgres -d api_gateway \
  -c "SELECT api_key, user_id FROM user_api_keys;"
```

### 问题 2: 连接超时

**原因**: 防火墙或网络问题

**解决**:
```bash
# 检查端口监听
netstat -tlnp | grep 8081

# 检查防火墙
sudo ufw allow 8081
```

### 问题 3: 模型不支持

**原因**: 模型名称映射缺失

**解决**: 在 `map_anthropic_model` 函数中添加对应映射

---

## 八、总结

改造完成后，中转系统将：

1. **完全兼容 Claude Code**: 支持 `x-api-key` 认证和 Anthropic Messages API
2. **兼容 OpenAI 格式**: 同时保留 `/v1/chat/completions` 端点
3. **自动模型映射**: 支持多种 Claude 模型名称自动路由到正确的资源池
4. **一键配置**: 通过 `setup-claude-code.sh` 快速配置用户端

用户只需设置环境变量即可使用中转系统，无需修改 Claude Code 客户端代码。
