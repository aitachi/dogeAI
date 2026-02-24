# 系统功能检查报告

生成时间: 2026-02-08

## 一、功能模块列表

| 模块 | 文件位置 | 功能描述 | 状态 |
|------|----------|----------|------|
| API Key管理 | `src/key_manager/mod.rs` | 多Provider Key管理、负载均衡、故障转移 | ✅ |
| 模型池 | `src/proxy/mod.rs` | 请求转发、健康检查、Claude Code协议支持 | ✅ |
| 计费系统 | `src/billing/mod.rs` | 预扣费、异步计费、余额管理 | ✅ |
| 认证授权 | `src/auth/mod.rs` | API Key验证、用户认证 | ✅ |
| 限流控制 | `src/ratelimit/mod.rs` | Token Bucket限流 | ✅ |
| Redis缓存 | `src/cache/mod.rs` | 余额缓存、限流缓存 | ✅ |
| PostgreSQL | `src/database/mod.rs` | 用户数据、计费记录、充值管理 | ✅ |
| 调度器 | `src/scheduler/mod.rs` | 后台任务调度 | ✅ |
| 充值服务 | `src/recharge/mod.rs` | 充值码管理 | ✅ |

## 二、API Key 配置检查

### 2.1 当前数据库配置

| ID | Key名称 | Provider | 协议 | 最大任务 | 最大并发 | 状态 | 模型列表 |
|----|---------|----------|------|----------|----------|------|----------|
| 1 | GLM-ClaudeCode-Main | glm | claude_code | 10 | 10 | ✅ | {claude-opus,claude-sonnet,glm-4.7,glm-plus,opus} |
| 2 | Qwen-Main | qwen | openai | 50 | 20 | ✅ | {qwen-coder,qwen-turbo,qwen-plus,codex,gemini} |
| 3 | Deepseek-Aitachi | deepseek | openai | 100 | 50 | ✅ | {deepseek-chat,deepseek-coder} |
| 4 | Deepseek-Zhangjiaqiang | deepseek | openai | 100 | 50 | ✅ | {deepseek-chat,deepseek-coder} |
| 5 | Deepseek-Shiyibing | deepseek | openai | 100 | 50 | ✅ | {deepseek-chat,deepseek-coder} |

### 2.2 配置问题

| 问题 | 严重性 | 描述 |
|------|--------|------|
| GLM base_url | ⚠️ 中 | 数据库未配置base_url，代码使用硬编码 `https://open.bigmodel.cn/api/paas/v4/chat/completions`，但Claude Code协议应使用 `https://open.bigmodel.cn/api/anthropic` |
| Qwen base_url | ⚠️ 中 | 数据库未配置base_url，代码使用硬编码 `https://dashscope.aliyuncs.com/compatible-mode/v1` |
| Qwen协议 | ℹ️ | 当前使用openai协议，正确 |
| 模型映射 | ⚠️ 中 | 代码中的模型映射(`src/proxy/mod.rs:194-212`)与用户需求可能不完全匹配 |

## 三、故障转移机制检查

### 3.1 代码实现位置

- 故障转移逻辑: `src/key_manager/mod.rs:188-225`
- 转发调用: `src/proxy/mod.rs:73-94`

### 3.2 故障转移顺序

代码中的转移顺序:
```
glm → qwen → deepseek
qwen → deepseek → glm
deepseek → qwen → glm
```

### 3.3 用户需求 vs 实际配置

| 需求 | 实际配置 | 状态 |
|------|----------|------|
| GLM 报错 → 迅速转为 qwen-code-latest | glm → qwen → deepseek | ⚠️ 顺序正确，但需要确保qwen使用coder模型 |
| Qwen 报错 → 自动转向 deepseek | qwen → deepseek → glm | ✅ 正确 |
| Deepseek作为储备池 | 3个Deepseek key按负载均衡 | ✅ 正确 |

## 四、用户等级与模型映射

### 4.1 计费规则 (`src/models/mod.rs:174-203`)

| 请求模型 | 计费类型 | 费用(分) | 实际Provider | 映射模型 |
|----------|----------|----------|--------------|----------|
| opus | Opus | 7 | GLM | claude-opus-4 |
| glm-4-plus | Opus | 7 | GLM | claude-opus-4 |
| sonnet | Sonnet | 4 | GLM | claude-sonnet-4 |
| glm-4-flashx | Sonnet | 4 | - | - |
| haiku | Haiku | 1 | - | - |
| glm-4-flash | Haiku | 1 | - | - |
| codex | Codex | 2 | Qwen | qwen-coder-latest |
| qwen-coder | Codex | 2 | Qwen | qwen-coder-latest |
| gemini | Gemini | 2 | Qwen | qwen-turbo-latest |
| qwen-turbo | Gemini | 2 | Qwen | qwen-turbo-latest |

### 4.2 用户等级检查

数据库中用户 `tier` 字段，但代码中**未发现按用户等级限制模型访问**的逻辑。

| 问题 | 严重性 |
|------|--------|
| 代码中无按tier限制模型访问的逻辑 | ⚠️ 中 |

## 五、日志存储检查

### 5.1 PostgreSQL存储

| 表名 | 用途 | 字段完整性 |
|------|------|-----------|
| users | 用户信息 | ✅ |
| billing_records | 计费记录 | ✅ 含request_id, model, tokens, cost |
| recharge_codes | 充值码 | ✅ |
| recharge_history | 充值历史 | ✅ |
| balance_reconciliation | 余额对账 | ✅ |
| api_keys | API Key配置 | ✅ |
| key_usage | Key使用状态 | ✅ |

### 5.2 Redis缓存

| Key格式 | 用途 | TTL |
|---------|------|-----|
| balance:{user_id} | 用户余额 | 3600s |
| rate:user:{user_id} | 限流计数 | 1s |
| limit_concurrent:{user_id} | 并发限制标记 | 300s |

### 5.3 日志问题

| 问题 | 严重性 | 位置 |
|------|--------|------|
| 无请求日志表 | ⚠️ 中 | 缺少request_log表 |
| 计费使用异步队列，可能丢失 | ⚠️ 高 | `src/billing/mod.rs:29-50` |
| 无故障转移日志 | ⚠️ 中 | - |

## 六、计费系统检查

### 6.1 计费流程

```
1. 用户请求 → pre_deduct() (预扣费，同步)
2. 转发请求到模型
3. 解析token使用量
4. record_billing() (异步写入队列)
5. 后台任务批量写入数据库
```

### 6.2 计费问题

| 问题 | 严重性 | 描述 |
|------|--------|------|
| 异步队列可能丢失数据 | 🔴 高 | 进程崩溃时队列中数据丢失 |
| 无请求ID追踪 | ⚠️ 中 | 无法追踪完整请求链路 |
| 余额对账机制未启用 | ⚠️ 中 | balance_reconciliation表存在但无定时对账任务 |

## 七、Claude Code协议检查

### 7.1 GLM配置

用户需求:
```json
{
  "ANTHROPIC_AUTH_TOKEN": "b8e22e2565834b0d9ce54dbb723fab34.NrRrzUS6ArwnsO2D",
  "ANTHROPIC_BASE_URL": "https://open.bigmodel.cn/api/anthropic"
}
```

代码实现 (`src/proxy/mod.rs:171`):
```rust
let url = format!("{}/v1/messages", key_mapping.base_url);
```

### 7.2 问题

| 问题 | 严重性 | 修复方案 |
|------|--------|----------|
| base_url未正确配置 | 🔴 高 | 需要在api_keys表设置 `https://open.bigmodel.cn/api/anthropic` |
| Header使用x-api-key而非Authorization | ✅ | 正确 |
| max_tasks=10符合需求 | ✅ | 正确 |

## 八、数据一致性检查

### 8.1 Redis vs PG余额

检查命令:
```bash
redis-cli GET "balance:1"  # 返回 100000
PG数据库 user_id=1 balance = 100000  # 一致
```

### 8.2 问题

| 问题 | 严重性 |
|------|--------|
| 无定时对账任务 | ⚠️ 中 |
| Redis余额TTL=3600s，可能miss | ℹ️ |

## 九、总结

### 9.1 关键问题

| 优先级 | 问题 |
|--------|------|
| 🔴 P0 | GLM base_url未配置为Claude Code协议URL |
| 🔴 P0 | 计费队列无持久化，进程崩溃会丢失 |
| ⚠️ P1 | 无用户等级模型访问控制 |
| ⚠️ P1 | 无请求日志表 |
| ⚠️ P2 | 无定时余额对账任务 |

### 9.2 正常功能

- ✅ API Key多Provider管理
- ✅ 故障转移机制
- ✅ Redis缓存
- ✅ PostgreSQL持久化
- ✅ 限流控制
- ✅ Deepseek储备池
