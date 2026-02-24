# 系统测试用例

生成时间: 2026-02-08

## 测试环境

- 服务地址: http://127.0.0.1:3000
- 测试用户API Key: sk-ant-VQTTPA6SU4BGYQGMEG4TVY3VJYJSUYAG2L5C5GNVQYEB65NB (user_id=20)

---

## 测试用例 1: 基础认证测试

**目的**: 验证API Key认证功能

**请求**:
```bash
curl -s -X POST 'http://127.0.0.1:3000/api/chat' \
  -H 'Content-Type: application/json' \
  -H 'Authorization: Bearer sk-ant-VQTTPA6SU4BGYQGMEG4TVY3VJYJSUYAG2L5C5GNVQYEB65NB' \
  -d '{
    "model": "opus",
    "messages": [{"role": "user", "content": "Hello"}],
    "max_tokens": 100
  }'
```

**预期结果**:
- HTTP 200
- 返回格式符合OpenAI Chat Completions规范
- 包含 `X-Balance` 响应头

**检查点**:
- [ ] 认证成功
- [ ] 返回余额信息
- [ ] response包含 `id`, `choices`, `usage`

---

## 测试用例 2: GLM Claude Code协议调用

**目的**: 验证GLM使用Claude Code协议调用

**请求**:
```bash
curl -s -X POST 'http://127.0.0.1:3000/api/chat' \
  -H 'Content-Type: application/json' \
  -H 'Authorization: Bearer sk-ant-VQTTPA6SU4BGYQGMEG4TVY3VJYJSUYAG2L5C5GNVQYEB65NB' \
  -d '{
    "model": "glm-4-plus",
    "messages": [{"role": "user", "content": "1+1=?"}],
    "max_tokens": 100
  }'
```

**预期结果**:
- 使用 protocol=claude_code
- 请求发送到 `https://open.bigmodel.cn/api/anthropic/v1/messages`
- Header包含 `x-api-key: b8e22e2565834b0d9ce54dbb723fab34.NrRrzUS6ArwnsO2D`

**检查点**:
- [ ] 使用正确的协议
- [ ] 模型映射到 claude-opus-4
- [ ] API Key正确

---

## 测试用例 3: Qwen模型调用

**目的**: 验证千问API调用

**请求**:
```bash
curl -s -X POST 'http://127.0.0.1:3000/api/chat' \
  -H 'Content-Type: application/json' \
  -H 'Authorization: Bearer sk-ant-VQTTPA6SU4BGYQGMEG4TVY3VJYJSUYAG2L5C5GNVQYEB65NB' \
  -d '{
    "model": "codex",
    "messages": [{"role": "user", "content": "print hello"}],
    "max_tokens": 100
  }'
```

**预期结果**:
- 使用 Qwen provider
- 实际调用 qwen-coder-latest
- 协议使用 openai compatible

**检查点**:
- [ ] Provider = qwen
- [ ] Base URL = https://dashscope.aliyuncs.com/compatible-mode/v1
- [ ] 模型正确映射

---

## 测试用例 4: 故障转移 - GLM到Qwen

**目的**: 验证GLM失败后自动切换到Qwen

**前置条件**: 临时禁用GLM key或修改key为无效值

**请求**:
```bash
# 先禁用GLM key
psql -h localhost -U postgres -d api_gateway -c "UPDATE api_keys SET enabled = false WHERE provider = 'glm';"

# 然后请求
curl -s -X POST 'http://127.0.0.1:3000/api/chat' \
  -H 'Content-Type: application/json' \
  -H 'Authorization: Bearer sk-ant-VQTTPA6SU4BGYQGMEG4TVY3VJYJSUYAG2L5C5GNVQYEB65NB' \
  -d '{
    "model": "opus",
    "messages": [{"role": "user", "content": "test"}],
    "max_tokens": 50
  }'

# 恢复GLM key
psql -h localhost -U postgres -d api_gateway -c "UPDATE api_keys SET enabled = true WHERE provider = 'glm';"
```

**预期结果**:
- GLM失败后自动切换到Qwen
- 日志中有 "Failover triggered" 记录

**检查点**:
- [ ] 故障转移发生
- [ ] 最终请求成功
- [ ] 使用了Qwen provider

---

## 测试用例 5: 故障转移 - Qwen到Deepseek

**目的**: 验证Qwen失败后自动切换到Deepseek储备池

**前置条件**: 临时禁用Qwen key

**请求**:
```bash
# 禁用Qwen key
psql -h localhost -U postgres -d api_gateway -c "UPDATE api_keys SET enabled = false WHERE provider = 'qwen';"

# 请求
curl -s -X POST 'http://127.0.0.1:3000/api/chat' \
  -H 'Content-Type: application/json' \
  -H 'Authorization: Bearer sk-ant-VQTTPA6SU4BGYQGMEG4TVY3VJYJSUYAG2L5C5GNVQYEB65NB' \
  -d '{
    "model": "codex",
    "messages": [{"role": "user", "content": "test"}],
    "max_tokens": 50
  }'

# 恢复Qwen key
psql -h localhost -U postgres -d api_gateway -c "UPDATE api_keys SET enabled = true WHERE provider = 'qwen';"
```

**预期结果**:
- 自动切换到Deepseek储备池
- 3个Deepseek key按负载均衡选择

**检查点**:
- [ ] 使用了Deepseek provider
- [ ] 请求成功
- [ ] 负载均衡正常

---

## 测试用例 6: 计费系统测试

**目的**: 验证计费是否正确

**请求**:
```bash
# 获取初始余额
curl -s -X GET 'http://127.0.0.1:3000/api/user/balance' \
  -H 'Authorization: Bearer sk-ant-VQTTPA6SU4BGYQGMEG4TVY3VJYJSUYAG2L5C5GNVQYEB65NB'

# 发送opus请求 (费用7分)
curl -s -X POST 'http://127.0.0.1:3000/api/chat' \
  -H 'Content-Type: application/json' \
  -H 'Authorization: Bearer sk-ant-VQTTPA6SU4BGYQGMEG4TVY3VJYJSUYAG2L5C5GNVQYEB65NB' \
  -d '{
    "model": "opus",
    "messages": [{"role": "user", "content": "test"}],
    "max_tokens": 100
  }'

# 检查余额
curl -s -X GET 'http://127.0.0.1:3000/api/user/balance' \
  -H 'Authorization: Bearer sk-ant-VQTTPA6SU4BGYQGMEG4TVY3VJYJSUYAG2L5C5GNVQYEB65NB'

# 检查计费记录
psql -h localhost -U postgres -d api_gateway -c "SELECT * FROM billing_records ORDER BY created_at DESC LIMIT 1;"
```

**预期结果**:
- Opus费用: 7分
- 余额正确减少
- billing_records表有新记录

**检查点**:
- [ ] 余额减少7分
- [ ] 计费记录正确
- [ ] request_id存在

---

## 测试用例 7: 不同模型计费标准

**目的**: 验证不同模型的计费标准

**请求**:
```bash
# Haiku (1分)
curl -s -X POST 'http://127.0.0.1:3000/api/chat' \
  -H 'Content-Type: application/json' \
  -H 'Authorization: Bearer sk-ant-VQTTPA6SU4BGYQGMEG4TVY3VJYJSUYAG2L5C5GNVQYEB65NB' \
  -d '{"model": "haiku", "messages": [{"role": "user", "content": "hi"}], "max_tokens": 50}'

# Sonnet (4分)
curl -s -X POST 'http://127.0.0.1:3000/api/chat' \
  -H 'Content-Type: application/json' \
  -H 'Authorization: Bearer sk-ant-VQTTPA6SU4BGYQGMEG4TVY3VJYJSUYAG2L5C5GNVQYEB65NB' \
  -d '{"model": "sonnet", "messages": [{"role": "user", "content": "hi"}], "max_tokens": 50}'

# Codex (2分)
curl -s -X POST 'http://127.0.0.1:3000/api/chat' \
  -H 'Content-Type: application/json' \
  -H 'Authorization: Bearer sk-ant-VQTTPA6SU4BGYQGMEG4TVY3VJYJSUYAG2L5C5GNVQYEB65NB' \
  -d '{"model": "codex", "messages": [{"role": "user", "content": "hi"}], "max_tokens": 50}'
```

**预期结果**:
- Haiku: 1分
- Sonnet: 4分
- Codex: 2分

**检查点**:
- [ ] 各模型计费正确
- [ ] billing_records记录正确

---

## 测试用例 8: 并发限制测试

**目的**: 验证并发限制功能

**请求**:
```bash
# 发送5个并发请求
for i in {1..5}; do
  curl -s -X POST 'http://127.0.0.1:3000/api/chat' \
    -H 'Content-Type: application/json' \
    -H 'Authorization: Bearer sk-ant-VQTTPA6SU4BGYQGMEG4TVY3VJYJSUYAG2L5C5GNVQYEB65NB' \
    -d '{"model": "haiku", "messages": [{"role": "user", "content": "test '$i'"}], "max_tokens": 50}' &
done
wait

# 检查key_usage表
psql -h localhost -U postgres -d api_gateway -c "SELECT * FROM key_usage;"
```

**预期结果**:
- 并发请求被正确计数
- concurrent_requests正确更新

**检查点**:
- [ ] 并发计数正确
- [ ] 请求全部成功
- [ ] key_usage正确更新

---

## 测试用例 9: Redis缓存一致性

**目的**: 验证Redis与PG数据一致性

**请求**:
```bash
# 检查Redis余额
redis-cli GET "balance:20"

# 检查PG余额
psql -h localhost -U postgres -d api_gateway -c "SELECT balance FROM users WHERE id = 20;"

# 发送请求
curl -s -X POST 'http://127.0.0.1:3000/api/chat' \
  -H 'Content-Type: application/json' \
  -H 'Authorization: Bearer sk-ant-VQTTPA6SU4BGYQGMEG4TVY3VJYJSUYAG2L5C5GNVQYEB65NB' \
  -d '{"model": "haiku", "messages": [{"role": "user", "content": "test"}], "max_tokens": 50}'

# 再次检查
redis-cli GET "balance:20"
psql -h localhost -U postgres -d api_gateway -c "SELECT balance FROM users WHERE id = 20;"
```

**预期结果**:
- Redis与PG余额一致
- 请求后两者同步减少

**检查点**:
- [ ] 初始余额一致
- [ ] 请求后同步更新
- [ ] Redis TTL正常

---

## 测试用例 10: 用户余额不足测试

**目的**: 验证余额不足时的处理

**前置条件**: 创建低余额用户或扣减余额

**请求**:
```bash
# 将用户余额设为5分
psql -h localhost -U postgres -d api_gateway -c "UPDATE users SET balance = 5 WHERE id = 20;"
redis-cli SET "balance:20" 5

# 尝试发送opus请求 (需要7分)
curl -s -X POST 'http://127.0.0.1:3000/api/chat' \
  -H 'Content-Type: application/json' \
  -H 'Authorization: Bearer sk-ant-VQTTPA6SU4BGYQGMEG4TVY3VJYJSUYAG2L5C5GNVQYEB65NB' \
  -d '{"model": "opus", "messages": [{"role": "user", "content": "test"}], "max_tokens": 100}'

# 恢复余额
psql -h localhost -U postgres -d api_gateway -c "UPDATE users SET balance = 200 WHERE id = 20;"
redis-cli SET "balance:20" 200
```

**预期结果**:
- 返回 402 Payment Required
- 错误信息: "insufficient_balance"
- 余额未扣减

**检查点**:
- [ ] 正确拒绝请求
- [ ] 返回合适的状态码
- [ ] 余额保持不变

---

## 测试执行记录

| 用例 | 执行时间 | 结果 | 问题描述 |
|------|----------|------|----------|
| 1 | - | - | - |
| 2 | - | - | - |
| 3 | - | - | - |
| 4 | - | - | - |
| 5 | - | - | - |
| 6 | - | - | - |
| 7 | - | - | - |
| 8 | - | - | - |
| 9 | - | - | - |
| 10 | - | - | - |

---

## 快速测试脚本

```bash
#!/bin/bash
# 快速执行所有测试用例

API_KEY="sk-ant-VQTTPA6SU4BGYQGMEG4TVY3VJYJSUYAG2L5C5GNVQYEB65NB"
BASE_URL="http://127.0.0.1:3000"

echo "=== 测试用例 1: 基础认证 ==="
curl -s -X POST "$BASE_URL/api/chat" \
  -H 'Content-Type: application/json' \
  -H "Authorization: Bearer $API_KEY" \
  -d '{"model": "haiku", "messages": [{"role": "user", "content": "Hello"}], "max_tokens": 50}' | jq .

echo -e "\n=== 测试用例 2: GLM模型 ==="
curl -s -X POST "$BASE_URL/api/chat" \
  -H 'Content-Type: application/json' \
  -H "Authorization: Bearer $API_KEY" \
  -d '{"model": "glm-4-plus", "messages": [{"role": "user", "content": "1+1=?"}], "max_tokens": 50}' | jq .

echo -e "\n=== 测试用例 3: Codex模型 ==="
curl -s -X POST "$BASE_URL/api/chat" \
  -H 'Content-Type: application/json' \
  -H "Authorization: Bearer $API_KEY" \
  -d '{"model": "codex", "messages": [{"role": "user", "content": "print hello"}], "max_tokens": 50}' | jq .

echo -e "\n=== 检查余额 ==="
curl -s -X GET "$BASE_URL/api/user/balance" \
  -H "Authorization: Bearer $API_KEY" | jq .
```
