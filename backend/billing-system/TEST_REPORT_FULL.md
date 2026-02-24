# 系统测试与问题修复报告

生成时间: 2026-02-08

## 一、已修复的问题 ✅

| 问题 | 修复方案 | 状态 |
|------|----------|------|
| tier大小写不一致 | 统一为小写 | ✅ |
| haiku模型未配置 | 添加到GLM的models列表 | ✅ |
| sonnet模型未配置 | 添加到GLM的models列表 | ✅ |
| glm-4-flashx模型未配置 | 添加到GLM的models列表 | ✅ |
| Qwen base_url未配置 | 设置为 https://dashscope.aliyuncs.com/compatible-mode/v1 | ✅ |
| Deepseek base_url未配置 | 设置为 https://api.deepseek.com | ✅ |
| 用户等级限制模型访问 | 改为所有用户都可访问所有模型 | ✅ |
| 请求日志表缺失 | 创建 request_log 表 | ✅ |

---

## 二、当前系统状态

### 2.1 API Keys 配置

```
┌─────┬───────────────────────┬───────────┬─────────────────────────────────────┬──────────┬─────────┐
│ ID  │ Key Name              │ Provider  │ Base URL                            │ Protocol │ Models  │
├─────┼───────────────────────┼───────────┼─────────────────────────────────────┼──────────┼─────────┤
│ 1   │ GLM-ClaudeCode-Main   │ glm       │ open.bigmodel.cn/api/anthropic      │ claude   │ 9 models│
│ 2   │ Qwen-Main             │ qwen      │ dashscope.aliyuncs.com/compatible   │ openai   │ 5 models│
│ 3   │ Deepseek-Aitachi      │ deepseek  │ api.deepseek.com                    │ openai   │ 2 models│
│ 4   │ Deepseek-Zhangjiaqiang│ deepseek  │ api.deepseek.com                    │ openai   │ 2 models│
│ 5   │ Deepseek-Shiyibing    │ deepseek  │ api.deepseek.com                    │ openai   │ 2 models│
└─────┴───────────────────────┴───────────┴─────────────────────────────────────┴──────────┴─────────┘
```

### 2.2 用户等级分布

| Tier | 用户数 | 平均余额 | 模型访问权限 |
|------|--------|----------|-------------|
| base | 17 | 82000 | 全部模型 |
| pro | 1 | 1000 | 全部模型 |

### 2.3 计费规则

| 模型 | 费用 |
|------|------|
| opus / glm-4-plus | 7分 |
| sonnet / glm-4-flashx | 4分 |
| haiku / glm-4-flash | 1分 |
| codex / qwen-coder | 2分 |
| gemini / qwen-turbo | 2分 |

---

## 三、待修复/待测试的问题

### 3.1 🔴 网络连接问题 (严重)

**现象**: 所有上游API连接失败
```
HTTP request failed: client error (Connect)
```

**影响**: 无法完成实际的模型调用

**建议**:
- [ ] 配置HTTP代理
- [ ] 检查防火墙出站规则
- [ ] 使用VPN或专线

### 3.2 ⚠️ 旧用户API Key格式 (中)

**现象**: 部分用户使用 `sk_xxx` 格式而非 `sk-ant-xxx`

**影响**: 新旧格式混用，但不影响功能

**建议**:
- [ ] 保持兼容（当前状态）
- [ ] 或逐步迁移到新格式

### 3.3 ⚠️ 余额与计费 (中)

**现象**: `billing_records` 表为空

**原因**: 由于网络问题，没有成功的请求

**建议**:
- [ ] 网络修复后验证计费功能
- [ ] 测试余额扣减

### 3.4 ℹ️ 故障转移测试 (待验证)

**需要测试**:
- [ ] GLM失败 → Qwen
- [ ] Qwen失败 → Deepseek
- [ ] Deepseek储备池轮询

### 3.5 ℹ️ 并发限制测试 (待验证)

**需要测试**:
- [ ] GLM max_tasks=10
- [ ] Qwen max_tasks=50
- [ ] Deepseek max_tasks=100

---

## 四、完整测试清单

### 4.1 基础功能测试

```bash
# [1] 健康检查
curl http://127.0.0.1:3000/health

# [2] 获取余额
curl -X GET http://127.0.0.1:3000/api/user/balance \
  -H "Authorization: Bearer YOUR_API_KEY"

# [3] 模型健康状态
curl http://127.0.0.1:3000/api/model/health

# [4] API Key状态
curl http://127.0.0.1:3000/api/admin/keys/status
```

### 4.2 模型访问测试 (网络修复后)

```bash
# [5] Base用户访问opus (7分)
curl -X POST http://127.0.0.1:3000/v1/chat/completions \
  -H "Authorization: Bearer YOUR_API_KEY" \
  -d '{"model": "opus", "messages": [{"role": "user", "content": "Hi"}]}'

# [6] Base用户访问sonnet (4分)
curl -X POST http://127.0.0.1:3000/v1/chat/completions \
  -H "Authorization: Bearer YOUR_API_KEY" \
  -d '{"model": "sonnet", "messages": [{"role": "user", "content": "Hi"}]}'

# [7] Base用户访问haiku (1分)
curl -X POST http://127.0.0.1:3000/v1/chat/completions \
  -H "Authorization: Bearer YOUR_API_KEY" \
  -d '{"model": "haiku", "messages": [{"role": "user", "content": "Hi"}]}'

# [8] Codex模型 (2分)
curl -X POST http://127.0.0.1:3000/v1/chat/completions \
  -H "Authorization: Bearer YOUR_API_KEY" \
  -d '{"model": "codex", "messages": [{"role": "user", "content": "Hi"}]}'
```

### 4.3 故障转移测试

```bash
# [9] 禁用GLM，测试故障转移到Qwen
psql -h localhost -U postgres -d api_gateway -c "UPDATE api_keys SET enabled = false WHERE provider = 'glm';"
# 然后发送opus请求，应该转移到Qwen

# [10] 禁用Qwen，测试故障转移到Deepseek
psql -h localhost -U postgres -d api_gateway -c "UPDATE api_keys SET enabled = false WHERE provider = 'qwen';"
# 然后发送codex请求，应该转移到Deepseek

# 恢复
UPDATE api_keys SET enabled = true WHERE provider IN ('glm', 'qwen');
```

### 4.4 计费测试

```bash
# [11] 检查余额变化
# 发送请求前记录余额
# 发送请求
# 检查余额是否正确扣减

# [12] 检查计费记录
psql -h localhost -U postgres -d api_gateway -c "SELECT * FROM billing_records ORDER BY created_at DESC LIMIT 5;"
```

### 4.5 请求日志测试

```bash
# [13] 检查请求日志
psql -h localhost -U postgres -d api_gateway -c "
SELECT request_id, model_requested, status, cost, created_at
FROM request_log
ORDER BY created_at DESC
LIMIT 10;"
```

---

## 五、快速验证脚本

```bash
#!/bin/bash
# 快速验证所有API端点

BASE_URL="http://127.0.0.1:3000"
API_KEY="sk-ant-VQTTPA6SU4BGYQGMEG4TVY3VJYJSUYAG2L5C5GNVQYEB65NB"

echo "=== API端点测试 ==="
curl -s ${BASE_URL}/health | jq .status
curl -s ${BASE_URL}/api/user/balance -H "Authorization: Bearer ${API_KEY}" | jq .
curl -s ${BASE_URL}/api/model/health | jq .
curl -s ${BASE_URL}/api/admin/keys/status | jq '.keys | length'

echo -e "\n=== 数据库检查 ==="
psql -h localhost -U postgres -d api_gateway -c "
SELECT 'users' as table_name, COUNT(*) as count FROM users
UNION ALL
SELECT 'api_keys', COUNT(*) FROM api_keys
UNION ALL
SELECT 'billing_records', COUNT(*) FROM billing_records
UNION ALL
SELECT 'request_log', COUNT(*) FROM request_log;"
```

---

## 六、修复优先级

| 优先级 | 问题 | 建议操作 |
|--------|------|----------|
| P0 | 网络连接 | 配置代理/VPN |
| P1 | 验证计费功能 | 网络修复后测试 |
| P2 | 验证故障转移 | 网络修复后测试 |
| P3 | 统一API Key格式 | 可选 |
