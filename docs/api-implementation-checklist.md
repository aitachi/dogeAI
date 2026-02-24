# API 实现清单

本文件用于跟踪三个分支的 API 实现状态，方便对齐工作。

## 评分说明
- ✅ 已实现
- ⚠️ 部分实现
- ❌ 未实现
- 🔄 需要对齐

---

## 一、核心消息 API

| 端点 | 方法 | main | pyDogeAI | rust-simple | rust-billing | 优先级 | 对齐目标 |
|------|------|------|----------|-------------|--------------|--------|----------|
| `/v1/messages` | POST | ✅ | ✅ | ✅ | ❌ | P0 | rust-billing 需实现 Anthropic 格式 |
| `/v1/chat/completions` | POST | ❌ | ❌ | ✅ | ✅ | P1 | 需统一 OpenAI 格式 |
| `/v1/messages` (stream) | POST | ✅ | ✅ | ✅ | ⚠️ | P0 | rust-billing 流式需完善 |
| `/v1/messages/count_tokens` | POST | ❌ | ✅ | ❌ | ❌ | P2 | rust 系统需补充 |
| `/v1/complete` | POST | ❌ | ✅ | ❌ | ❌ | P3 | 可选，兼容旧版 |

**对齐说明：**
1. `rust-billing` 需添加 Anthropic Messages API (`/v1/messages`)
2. 确保流式响应格式完全兼容
3. Token 计算可作为可选功能

---

## 二、模型与服务信息

| 端点 | 方法 | main | pyDogeAI | rust-simple | rust-billing | 优先级 | 对齐目标 |
|------|------|------|----------|-------------|--------------|--------|----------|
| `/v1/models` | GET | ✅ | ❌ | ✅ | ❌ | P0 | pyDogeAI 和 rust-billing 需补充 |
| `/health` | GET | ✅ | ✅ | ✅ | ✅ | P0 | 已对齐 |
| `/` | GET | ✅ | ❌ | ❌ | ✅ | P2 | 统一服务信息端点 |
| `/api/hello` | GET | ❌ | ✅ | ❌ | ❌ | P1 | pyDogeAI 专用 |

**对齐说明：**
1. 所有分支应支持 `/v1/models` 返回标准 Anthropic 模型列表
2. 健康检查端点已基本对齐

---

## 三、OAuth 认证

| 端点 | 方法 | main | pyDogeAI | rust-simple | rust-billing | 优先级 | 对齐目标 |
|------|------|------|----------|-------------|--------------|--------|----------|
| `/oauth/authorize` | GET | ❌ | ✅ | ❌ | ❌ | P1 | Rust 系统需补充 |
| `/oauth/token` | POST | ❌ | ✅ | 🔄 | 🔄 | P1 | Rust 系统需补充 |
| `/oauth/code/callback` | GET | ❌ | ✅ | ❌ | ❌ | P1 | pyDogeAI 专用 |
| `/generate-code` | GET | ❌ | ✅ | ❌ | ❌ | P2 | pyDogeAI 辅助端点 |
| `/.well-known/openid-configuration` | GET | ❌ | ✅ | ❌ | ❌ | P2 | OAuth 标准发现 |
| `/.well-known/oauth-authorization-server` | GET | ❌ | ✅ | ❌ | ❌ | P2 | OAuth 标准发现 |

**对齐说明：**
1. Rust 系统需要实现完整的 OAuth 2.0 流程
2. `rust-simple` 的 `/auth/login` 和 `rust-billing` 的 `/api/user/login` 需对齐到 OAuth 格式

---

## 四、用户与账户管理

| 端点 | 方法 | main | pyDogeAI | rust-simple | rust-billing | 优先级 | 对齐目标 |
|------|------|------|----------|-------------|--------------|--------|----------|
| `/api/bootstrap` | GET | ❌ | ✅ | ❌ | ❌ | P1 | Rust 系统需补充 |
| `/api/auth` | GET | ❌ | ✅ | ❌ | ❌ | P1 | Rust 系统需补充 |
| `/api/auth/session` | GET | ❌ | ✅ | ❌ | ❌ | P1 | Rust 系统需补充 |
| `/api/account` | GET | ❌ | ✅ | ❌ | ❌ | P1 | Rust 系统需补充 |
| `/api/settings` | GET | ❌ | ✅ | ❌ | ❌ | P2 | Rust 系统需补充 |
| `/api/me` | GET | ❌ | ✅ | ❌ | ❌ | P2 | Rust 系统需补充 |
| `/v1/me` | GET | ❌ | ✅ | ❌ | ❌ | P2 | Rust 系统需补充 |
| `/userinfo` | GET | ❌ | ✅ | ❌ | ❌ | P2 | OAuth 标准 |

**对齐说明：**
1. 这些是 Claude Code 客户端必需的端点
2. Rust 系统需要实现这些端点以支持 Claude Code

---

## 五、Claude Code 专用配置

| 端点 | 方法 | main | pyDogeAI | rust-simple | rust-billing | 优先级 | 对齐目标 |
|------|------|------|----------|-------------|--------------|--------|----------|
| `/api/claude_code/settings` | GET | ❌ | ✅ | ❌ | ❌ | P1 | Rust 系统需补充 |
| `/api/claude_code/policy_limits` | GET | ❌ | ✅ | ❌ | ❌ | P1 | Rust 系统需补充 |
| `/api/claude_code/penguin_mode` | GET | ❌ | ✅ | ❌ | ❌ | P2 | Rust 系统需补充 |

**对齐说明：**
1. 返回格式示例：
```json
// /api/claude_code/settings
{
  "expiry": null,
  "isolated": false,
  "allowed_tools": ["computer", "text_editor", "bash"],
  "max_turns": null,
  "internet_policy": "allow"
}

// /api/claude_code/policy_limits
{
  "rate_limits": {
    "requests_per_minute": 60,
    "tokens_per_minute": 1000000,
    "tokens_per_day": 50000000
  },
  "usage": {
    "tokens_used_today": 0,
    "requests_today": 0
  },
  "limits": {}
}
```

---

## 六、API Key 管理

| 端点 | 方法 | main | pyDogeAI | rust-simple | rust-billing | 优先级 | 对齐目标 |
|------|------|------|----------|-------------|--------------|--------|----------|
| `/apply` | POST | ✅ | ❌ | ✅ | ✅ | P0 | 已对齐 |
| `/apply/check-availability/{field}/{value}` | GET | ✅ | ❌ | 🔄 | ✅ | P1 | 需统一格式 |
| `/auth/apply` | POST | ❌ | ❌ | ✅ | ❌ | P0 | 与 `/apply` 对齐 |
| `/api/apply` | POST | ❌ | ❌ | ❌ | ✅ | P0 | 与 `/apply` 对齐 |
| `/v1/organizations/{org_id}/api_keys` | POST | ❌ | ✅ | ❌ | ❌ | P1 | Rust 系统需补充 |
| `/api/organizations/{org_id}/api_keys` | POST | ❌ | ✅ | ❌ | ❌ | P1 | Rust 系统需补充 |
| `/api/organizations/{org_id}/api-keys` | POST | ❌ | ✅ | ❌ | ❌ | P1 | 同上 (连字符变体) |
| `/admin/keys/create` | POST | ✅ | ❌ | ❌ | ❌ | P2 | Rust 系统需补充 |
| `/admin/keys` | GET | ✅ | ❌ | ❌ | ✅ | P2 | 需对齐响应格式 |

**对齐说明：**
1. 统一 API Key 申请端点为 `/apply`
2. 组织 API Key 创建端点是 Claude Code 管理功能必需

---

## 七、用户认证与管理

| 端点 | 方法 | main | pyDogeAI | rust-simple | rust-billing | 优先级 | 对齐目标 |
|------|------|------|----------|-------------|--------------|--------|----------|
| `/api/user/register` | POST | ❌ | ❌ | ✅ | ✅ | P1 | 已对齐 |
| `/api/user/login` | POST | ❌ | ❌ | ✅ | ✅ | P1 | 已对齐 |
| `/api/user/email-login` | POST | ❌ | ❌ | ❌ | ✅ | P2 | rust-billing 专用 |
| `/auth/login` | POST | ❌ | ❌ | ✅ | ❌ | P1 | 与 `/api/user/login` 对齐 |
| `/api/user/profile` | GET | ❌ | ❌ | ❌ | ✅ | P1 | Rust 系统需补充 |
| `/api/user/balance` | GET | ❌ | ❌ | ❌ | ✅ | P1 | Rust 系统需补充 |
| `/api/user/change-password` | POST | ❌ | ❌ | ❌ | ✅ | P2 | 可选 |

---

## 八、管理接口

| 端点 | 方法 | main | pyDogeAI | rust-simple | rust-billing | 优先级 | 对齐目标 |
|------|------|------|----------|-------------|--------------|--------|----------|
| `/admin/stats` | GET | ✅ | ❌ | ❌ | 🔄 | P1 | Rust 系统需补充 |
| `/admin/keys/create` | POST | ✅ | ❌ | ❌ | ❌ | P2 | Rust 系统需补充 |
| `/admin/keys` | GET | ✅ | ❌ | ❌ | ✅ | P2 | 需对齐响应格式 |
| `/admin/recent-calls` | GET | ✅ | ❌ | ❌ | 🔄 | P2 | Rust 系统需补充 |
| `/api/stats` | GET | ❌ | ❌ | ❌ | ✅ | P1 | 与 `/admin/stats` 对齐 |
| `/api/admin/overview` | GET | ❌ | ❌ | ❌ | ✅ | P1 | 已实现 |
| `/api/admin/users` | GET | ❌ | ❌ | ❌ | ✅ | P1 | 已实现 |
| `/api/admin/billing` | GET | ❌ | ❌ | ❌ | ✅ | P1 | 已实现 |
| `/api/admin/keys/status` | GET | ❌ | ❌ | ❌ | ✅ | P2 | 已实现 |
| `/api/admin/login` | POST | ❌ | ❌ | ❌ | ✅ | P1 | 已实现 |
| `/api/admin/session` | GET | ❌ | ❌ | ❌ | ✅ | P1 | 已实现 |

**对齐说明：**
1. Rust billing-system 管理接口较完整
2. main 分支的统计接口需要迁移到 Rust

---

## 九、计费与充值

| 端点 | 方法 | main | pyDogeAI | rust-simple | rust-billing | 优先级 | 对齐目标 |
|------|------|------|----------|-------------|--------------|--------|----------|
| `/api/user/history` | GET | ❌ | ❌ | ❌ | ✅ | P2 | 可选 |
| `/api/recharge/redeem` | POST | ❌ | ❌ | ❌ | ✅ | P2 | 可选 |
| `/api/recharge/create` | POST | ❌ | ❌ | ❌ | ✅ | P2 | 可选 |
| `/v1/token/query` | POST | ❌ | ❌ | ✅ | ❌ | P2 | 可选 |

**对齐说明：**
1. 计费和充值是扩展功能，不需要在所有分支实现

---

## 十、健康与监控

| 端点 | 方法 | main | pyDogeAI | rust-simple | rust-billing | 优先级 | 对齐目标 |
|------|------|------|----------|-------------|--------------|--------|----------|
| `/health` | GET | ✅ | ✅ | ✅ | ✅ | P0 | ✅ 已对齐 |
| `/api/health/model` | GET | ❌ | ❌ | ❌ | ✅ | P2 | 可选 |
| `/api/health/model/:model` | GET | ❌ | ❌ | ❌ | ✅ | P2 | 可选 |
| `/api/admin/models/stats` | GET | ❌ | ❌ | ❌ | ✅ | P2 | 可选 |

---

## 十一、组织管理

| 端点 | 方法 | main | pyDogeAI | rust-simple | rust-billing | 优先级 | 对齐目标 |
|------|------|------|----------|-------------|--------------|--------|----------|
| `/api/organizations` | GET | ❌ | ✅ | ❌ | ❌ | P1 | Rust 系统需补充 |
| `/v1/organizations/{org_id}` | GET | ❌ | ✅ | ❌ | ❌ | P1 | Rust 系统需补充 |
| `/v1/organizations/{org_id}/api_keys` | POST | ❌ | ✅ | ❌ | ❌ | P1 | Rust 系统需补充 |
| `/v1/dashboard/billing/usage` | GET | ❌ | ✅ | ❌ | ❌ | P2 | 可选 |
| `/v1/usage` | GET | ❌ | ✅ | ❌ | ❌ | P2 | 可选 |

---

## 实现优先级总结

### P0 - 必须立即实现
- ✅ `/v1/messages` (Anthropic 格式)
- ✅ `/health`
- ✅ `/apply` (API Key 申请)
- ✅ `/v1/models`

### P1 - 高优先级 (Claude Code 兼容)
- `/api/bootstrap`
- `/api/auth` & `/api/auth/session`
- `/api/account`
- `/api/claude_code/settings`
- `/api/claude_code/policy_limits`
- `/oauth/authorize` & `/oauth/token`
- `/v1/organizations/{org_id}/api_keys`
- `/api/stats` 或 `/admin/stats`

### P2 - 中优先级 (增强功能)
- `/v1/messages/count_tokens`
- `/v1/complete` (兼容)
- `/.well-known/openid-configuration`
- `/admin/keys`
- `/admin/recent-calls`
- 充值相关接口
- 健康监控详细接口

### P3 - 低优先级
- 其他辅助端点

---

**最后更新:** 2026-02-24
**版本:** 1.0
