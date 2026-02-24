# Proxy API 对比与对齐文档

## 文档概述

本文档详细对比 dogeAI 项目三个分支的代理接口实现，并提供对齐建议。

**分支说明：**
- **main 分支** - Python FastAPI 代理服务 (智谱 AI 中转)
- **pyDogeAI 分支** - Python Claude Code 代理 (原生协议)
- **rust 分支** - Rust 高性能中转系统

---

## 一、端点对照表

### 1.1 核心消息 API (必须实现)

| 功能 | main 分支 | pyDogeAI 分支 | rust 分支 (api-gateway-simple) | rust 分支 (billing-system) | 优先级 |
|------|-----------|---------------|-------------------------------|----------------------------|--------|
| 消息 API (Anthropic) | `POST /v1/messages` | `POST /v1/messages` | `POST /v1/messages` | `POST /v1/chat/completions` | P0 |
| 消息 API (OpenAI) | - | - | `POST /v1/chat/completions` | `POST /v1/chat/completions` | P1 |
| 流式响应 | 支持 | 支持 | 支持 | 支持 | P0 |
| Token 计算 | - | `POST /v1/messages/count_tokens` | - | - | P2 |
| 兼容 Complete | - | `POST /v1/complete` | - | - | P3 |

### 1.2 模型与认证 (必须实现)

| 功能 | main 分支 | pyDogeAI 分支 | rust 分支 (api-gateway-simple) | rust 分支 (billing-system) | 优先级 |
|------|-----------|---------------|-------------------------------|----------------------------|--------|
| 模型列表 | `GET /v1/models` | - | `GET /v1/models` | - | P0 |
| 健康检查 | `GET /health` | `GET /api/hello` | `GET /health` | `GET /health` | P0 |
| 服务信息 | `GET /` | - | - | `GET /` | P2 |
| OAuth 授权 | - | `GET /oauth/authorize` | - | - | P1 |
| OAuth Token | - | `POST /oauth/token` | `POST /auth/login` | `POST /api/user/login` | P1 |

### 1.3 用户管理 (Claude Code 专用)

| 功能 | main 分支 | pyDogeAI 分支 | rust 分支 (api-gateway-simple) | rust 分支 (billing-system) | 优先级 |
|------|-----------|---------------|-------------------------------|----------------------------|--------|
| 引导信息 | - | `GET /api/bootstrap` | - | - | P1 |
| 认证信息 | - | `GET /api/auth` | - | - | P1 |
| 会话信息 | - | `GET /api/auth/session` | - | - | P1 |
| 账户信息 | - | `GET /api/account` | - | `GET /api/user/profile` | P1 |
| 设置信息 | - | `GET /api/settings` | - | - | P2 |
| 用户资料 | - | `GET /api/me` | - | `GET /api/user/profile` | P2 |
| Claude Code 设置 | - | `GET /api/claude_code/settings` | - | - | P1 |
| 策略限制 | - | `GET /api/claude_code/policy_limits` | - | - | P1 |

### 1.4 API Key 管理 (必须实现)

| 功能 | main 分支 | pyDogeAI 分支 | rust 分支 (api-gateway-simple) | rust 分支 (billing-system) | 优先级 |
|------|-----------|---------------|-------------------------------|----------------------------|--------|
| 申请 API Key | `POST /apply` | - | `POST /auth/apply` | `POST /api/apply` | P0 |
| 检查可用性 | `GET /apply/check-availability/{field}/{value}` | - | `GET /auth/check/{username}` | `GET /api/apply/check-availability/username/{username}` | P1 |
| 创建 Key (组织) | - | `POST /v1/organizations/{org_id}/api_keys` | - | - | P1 |
| 创建 Key (API) | - | `POST /api/organizations/{org_id}/api_keys` | - | - | P1 |

### 1.5 管理接口 (管理员专用)

| 功能 | main 分支 | pyDogeAI 分支 | rust 分支 (api-gateway-simple) | rust 分支 (billing-system) | 优先级 |
|------|-----------|---------------|-------------------------------|----------------------------|--------|
| 统计信息 | `GET /admin/stats` | - | - | `GET /api/stats` | P1 |
| 创建密钥 | `POST /admin/keys/create` | - | - | - | P2 |
| 密钥列表 | `GET /admin/keys` | - | - | `GET /api/admin/keys/status` | P2 |
| 最近调用 | `GET /admin/recent-calls` | - | - | `GET /api/admin/billing` | P2 |
| 管理员概览 | - | - | - | `GET /api/admin/overview` | P1 |
| 用户管理 | - | - | - | `GET /api/admin/users` | P1 |
| 模型统计 | - | - | - | `GET /api/admin/models/stats` | P2 |
| 模型健康 | - | - | - | `GET /api/health/model` | P2 |

### 1.6 充值与计费 (扩展功能)

| 功能 | main 分支 | pyDogeAI 分支 | rust 分支 (api-gateway-simple) | rust 分支 (billing-system) | 优先级 |
|------|-----------|---------------|-------------------------------|----------------------------|--------|
| 充值码兑换 | - | - | - | `POST /api/recharge/redeem` | P2 |
| 创建充值码 | - | - | - | `POST /api/recharge/create` | P2 |
| 计费历史 | - | - | - | `GET /api/user/history` | P2 |
| 余额查询 | - | - | - | `GET /api/user/balance` | P1 |

### 1.7 发现端点 (OAuth 标准)

| 功能 | main 分支 | pyDogeAI 分支 | rust 分支 (api-gateway-simple) | rust 分支 (billing-system) | 优先级 |
|------|-----------|---------------|-------------------------------|----------------------------|--------|
| OpenID 配置 | - | `GET /.well-known/openid-configuration` | - | - | P2 |
| OAuth 元数据 | - | `GET /.well-known/oauth-authorization-server` | - | - | P2 |
| UserInfo | - | `GET /userinfo` | - | - | P2 |

---

## 二、请求/响应格式对比

### 2.1 Anthropic Messages API

#### 请求格式 (三个分支兼容)

```json
{
  "model": "claude-3-5-sonnet-20241022",
  "max_tokens": 4096,
  "messages": [
    {"role": "user", "content": "Hello"}
  ],
  "stream": false,
  "system": "Optional system prompt"
}
```

#### 响应格式

**main 分支 (Python):**
```json
{
  "id": "msg_xxx",
  "type": "message",
  "role": "assistant",
  "content": [{"type": "text", "text": "..."}],
  "model": "claude-3-5-sonnet-20241022",
  "stop_reason": "end_turn",
  "usage": {
    "input_tokens": 10,
    "output_tokens": 20
  }
}
```

**pyDogeAI 分支:** 相同格式，额外支持 `cache_creation_input_tokens` 和 `cache_read_input_tokens`

**rust 分支 (api-gateway-simple):**
```json
{
  "id": "uuid",
  "response_type": "message",
  "role": "assistant",
  "content": [{"content_type": "text", "text": "..."}],
  "model": "requested-model",
  "stop_reason": "end_turn",
  "usage": {"input_tokens": 10, "output_tokens": 20}
}
```

**rust 分支 (billing-system):** 使用 OpenAI 格式

### 2.2 认证方式对比

| 分支 | 认证头 | Token 格式 | 验证方式 |
|------|--------|------------|----------|
| main | `x-api-key` | 自定义 API Key | SQLite 数据库 |
| pyDogeAI | `Authorization: Bearer` | OAuth Bearer Token | 伪造 OAuth 流程 |
| rust (api-gateway-simple) | `x-api-key` 或 `Authorization: Bearer` | JWT Token | PostgreSQL + Redis |
| rust (billing-system) | `x-api-key` | 用户 API Key | PostgreSQL + Redis |

### 2.3 限流策略对比

| 分支 | 限流维度 | 默认限制 | 实现方式 |
|------|----------|----------|----------|
| main | 每日 Token 限额 | 1,000,000 tokens/天 | SQLite + 请求时检查 |
| pyDogeAI | 未实现 | - | - |
| rust (api-gateway-simple) | QPS + 余额 | base: 10, pro: 100, max: 200 | Redis INCR |
| rust (billing-system) | QPS + 余额 | Enterprise: 200, 其他: 100 | Redis + 数据库 |

---

## 三、核心功能对比

### 3.1 并发控制

| 分支 | 实现方式 | 限制 |
|------|----------|------|
| main | `asyncio.Semaphore` + 用户级锁 | 全局 20 并发，单用户 1 并发 |
| pyDogeAI | 未实现 | - |
| rust (api-gateway-simple) | 资源池系统 | 基于提供商和任务序号 |
| rust (billing-system) | 未明确 | - |

### 3.2 模型映射

**main 分支:**
```python
MODEL_MAPPING = {
    "claude-sonnet-4.5": "claude-3-5-sonnet-20241022",
    "claude-opus-4.5": "claude-3-opus-20240229",
    # ...
}
```

**pyDogeAI 分支:**
```python
MODEL_MAP = {
    "claude-sonnet-4-6": "claude-sonnet-4",
    "claude-opus-4-6": "claude-opus-4",
    # ...
}
```

**rust (api-gateway-simple) 分支:**
```rust
// 基于资源池的动态路由
map_anthropic_model() -> pool_id (opus/sonnet/haiku/codex)
```

### 3.3 日志与监控

| 分支 | 日志位置 | 监控指标 |
|------|----------|----------|
| main | `/var/log/anthropic-proxy/api-proxy.log` | 请求统计、响应时间、错误率 |
| pyDogeAI | 标准输出 | 请求/响应日志 |
| rust (api-gateway-simple) | tracing | 请求 ID、耗时、提供商、故障转移 |
| rust (billing-system) | tracing + 数据库 | 完整计费记录 |

---

## 四、缺失功能清单

### 4.1 rust (api-gateway-simple) 需要补充的端点

| 端点 | 优先级 | 说明 |
|------|--------|------|
| `POST /v1/complete` | P3 | 兼容旧版接口 |
| `POST /v1/messages/count_tokens` | P2 | Token 预计算 |
| `GET /api/bootstrap` | P1 | Claude Code 引导信息 |
| `GET /api/auth` | P1 | 认证信息 |
| `GET /api/auth/session` | P1 | 会话信息 |
| `GET /api/account` | P1 | 账户信息 |
| `GET /api/claude_code/settings` | P1 | Claude Code 专用设置 |
| `GET /api/claude_code/policy_limits` | P1 | 策略限制 |
| `GET /.well-known/openid-configuration` | P2 | OAuth 发现 |
| `GET /oauth/authorize` | P1 | OAuth 授权 |
| `POST /oauth/token` | P1 | OAuth Token 交换 |
| `POST /v1/organizations/{org_id}/api_keys` | P1 | 组织 API Key 创建 |
| `GET /admin/stats` | P1 | 统计信息 |
| `GET /admin/recent-calls` | P2 | 最近调用 |

### 4.2 rust (billing-system) 需要补充的端点

| 端点 | 优先级 | 说明 |
|------|--------|------|
| `POST /v1/messages` | P0 | Anthropic 格式消息 API |
| `GET /v1/models` | P0 | Anthropic 格式模型列表 |
| `GET /api/bootstrap` | P1 | Claude Code 引导信息 |
| `GET /api/claude_code/settings` | P1 | Claude Code 专用设置 |
| `GET /api/claude_code/policy_limits` | P1 | 策略限制 |
| `POST /v1/messages/count_tokens` | P2 | Token 预计算 |
| `GET /oauth/authorize` | P1 | OAuth 授权 |
| `POST /oauth/token` | P1 | OAuth Token 交换 |

### 4.3 pyDogeAI 需要补充的端点

| 端点 | 优先级 | 说明 |
|------|--------|------|
| `GET /v1/models` | P0 | 模型列表 |
| `POST /apply` | P0 | API Key 申请 |
| `GET /apply/check-availability/{field}/{value}` | P1 | 可用性检查 |
| 用户管理接口 | P1 | 注册、登录、余额查询 |
| 管理员接口 | P2 | 统计、用户管理 |

---

## 五、对齐路线图

### 阶段一：核心 API 对齐 (P0 - 1-2 周)

**目标：** 确保 Rust 系统完全兼容 Claude Code 基础功能

1. **实现 Anthropic Messages API (billing-system)**
   - [ ] `POST /v1/messages` - 非流式和流式
   - [ ] `GET /v1/models` - 返回标准 Anthropic 模型列表
   - [ ] 请求/响应格式完全兼容

2. **统一认证方式**
   - [ ] 支持 `x-api-key` 头认证
   - [ ] 支持 `Authorization: Bearer` 认证
   - [ ] 实现 OAuth 流程 (pyDogeAI 兼容)

### 阶段二：Claude Code 专用端点 (P1 - 1 周)

**目标：** 支持完整的 Claude Code 客户端功能

1. **引导和认证端点**
   - [ ] `GET /api/bootstrap`
   - [ ] `GET /api/auth`
   - [ ] `GET /api/auth/session`
   - [ ] `GET /api/account`
   - [ ] `GET /api/me`
   - [ ] `GET /userinfo`

2. **Claude Code 配置**
   - [ ] `GET /api/claude_code/settings`
   - [ ] `GET /api/claude_code/policy_limits`
   - [ ] `GET /api/claude_code/penguin_mode`

3. **OAuth 端点**
   - [ ] `GET /oauth/authorize`
   - [ ] `POST /oauth/token`
   - [ ] `GET /.well-known/openid-configuration`

### 阶段三：管理功能 (P1 - 1 周)

**目标：** 完善管理和监控功能

1. **API Key 管理**
   - [ ] `POST /v1/organizations/{org_id}/api_keys`
   - [ ] `POST /api/organizations/{org_id}/api_keys`
   - [ ] `GET /admin/keys`

2. **统计和监控**
   - [ ] `GET /admin/stats`
   - [ ] `GET /admin/recent-calls`
   - [ ] `GET /api/stats`

### 阶段四：扩展功能 (P2 - 按需实现)

1. **Token 相关**
   - [ ] `POST /v1/messages/count_tokens`
   - [ ] `POST /v1/complete` (兼容)

2. **高级管理**
   - [ ] 充值码系统
   - [ ] 用户等级管理
   - [ ] 详细计费报表

---

## 六、实现建议

### 6.1 架构统一

**建议采用 api-gateway-simple 的架构模式：**
- 使用资源池管理多个上游提供商
- 实现基于任务序号的智能路由
- 支持自动故障转移

### 6.2 认证统一

**推荐认证方式：**
1. 主要使用 `x-api-key` 头 (简单直接)
2. 兼容 `Authorization: Bearer` (OAuth 场景)
3. 实现 JWT Token 刷新机制

### 6.3 响应格式统一

**Anthropic Messages API 响应格式：**
```rust
pub struct AnthropicMessagesResponse {
    pub id: String,
    pub response_type: String,  // "message"
    pub role: String,           // "assistant"
    pub content: Vec<ContentBlock>,
    pub model: String,          // 请求时的模型名
    pub stop_reason: String,
    pub usage: Usage,
}
```

### 6.4 错误处理统一

**标准错误响应：**
```json
{
  "type": "error",
  "error": {
    "type": "invalid_request_error|authentication_error|rate_limit_error|api_error",
    "message": "详细错误信息"
  }
}
```

---

## 七、测试建议

### 7.1 兼容性测试

1. 使用 Claude Code CLI 测试所有端点
2. 验证请求/响应格式一致性
3. 测试流式和非流式模式

### 7.2 性能测试

1. 并发请求测试 (10/50/100 并发)
2. 流式响应延迟测试
3. 故障转移测试

### 7.3 集成测试

1. 上游 API 模拟测试
2. 限流和计费准确性测试
3. 多提供商切换测试

---

## 八、附录

### A. 端点优先级说明

| 优先级 | 说明 | 示例 |
|--------|------|------|
| P0 | 核心功能，必须实现 | `/v1/messages`, `/health` |
| P1 | 重要功能，强烈建议 | `/api/bootstrap`, OAuth 端点 |
| P2 | 扩展功能，按需实现 | `/v1/messages/count_tokens` |
| P3 | 兼容性功能，低优先级 | `/v1/complete` |

### B. 模型名称映射建议

**内部池 ID 到 Anthropic 模型名：**
```
opus    -> claude-opus-4-6, claude-opus-4-5-20250929, claude-3-opus-20240229
sonnet  -> claude-sonnet-4-5-20250929, claude-3-5-sonnet-20241022
haiku   -> claude-haiku-4-5-20251001, claude-3-5-haiku-20241022
codex   -> (预留)
```

### C. 环境变量对照

| Python (main) | Python (pyDogeAI) | Rust | 说明 |
|---------------|-------------------|------|------|
| `UPSTREAM_API_KEY` | `API_KEY` | `UPSTREAM_API_KEY` | 上游 API 密钥 |
| `UPSTREAM_BASE_URL` | `API_BASE_URL` | `UPSTREAM_API_URL` | 上游 API 地址 |
| `ADMIN_KEY` | - | `JWT_SECRET` | 管理员密钥/JWT 密钥 |
| `PROXY_PORT` | `HTTP_PORT` | `SERVER_PORT` | 服务端口 |
| - | `DEFAULT_MODEL` | `DEFAULT_MODEL` | 默认模型 |

---

**文档版本:** 1.0
**更新日期:** 2026-02-24
**维护者:** dogeAI 项目组
