# 🧪 Claude Code Proxy - API测试完整指南

## 📋 测试脚本说明

我已创建了两个自动化测试脚本：

1. **test_api.sh** - Bash脚本（推荐）
2. **test_api.py** - Python脚本（需要requests库）

---

## 🚀 快速测试

### 方式1: Bash脚本（推荐）

```bash
cd /root/dogeAI
bash test_api.sh
```

**输出示例**:
```
========================================
🧪 Claude Code Proxy - API测试
========================================
测试目标: 127.0.0.1
测试端口: 443 (HTTPS)

✅ 测试 1: 健康检查 - /api/hello
✅ 测试 2: 健康检查 - 根路径
✅ 测试 3: 模型列表
...
总计: 23
通过: 23
失败: 0
🎉 所有测试通过！
```

---

### 方式2: Python脚本（需要安装requests）

```bash
cd /root/dogeAI

# 安装依赖
pip3.11 install requests

# 运行测试
python3.11 test_api.py
```

---

## 📊 测试覆盖范围

### 1️⃣ 基础健康检查 (2个测试)
- ✅ GET /api/hello
- ✅ GET /

### 2️⃣ 模型管理 (2个测试)
- ✅ GET /v1/models
- ✅ GET /v1/models/{id}

### 3️⃣ Claude Code端点 (3个测试)
- ✅ GET /api/claude_code/settings
- ✅ GET /api/claude_code/policy_limits
- ✅ GET /api/claude_code/penguin_mode

### 4️⃣ 平台API (5个测试)
- ✅ GET /api/bootstrap
- ✅ GET /api/auth
- ✅ GET /api/account
- ✅ GET /api/organizations
- ✅ GET /v1/organizations

### 5️⃣ OAuth认证 (2个测试)
- ✅ GET /v1/oauth/hello
- ✅ GET /.well-known/openid-configuration

### 6️⃣ 核心消息API (2个测试)
- ✅ POST /v1/messages (1+1=?)
- ✅ POST /v1/messages/count_tokens

### 7️⃣ 用户信息 (3个测试)
- ✅ GET /api/me
- ✅ GET /v1/me
- ✅ GET /userinfo

### 8️⃣ Usage统计 (2个测试)
- ✅ GET /v1/dashboard/billing/usage
- ✅ GET /v1/usage

### 9️⃣ 工具调用 (1个测试)
- ✅ POST /v1/messages (带tools)

### 🔟 系统提示 (1个测试)
- ✅ POST /v1/messages (带system)

**总计**: 23个测试用例

---

## 🎯 手动测试

如果需要手动测试特定功能，可以使用以下命令：

### 基础测试

```bash
# 健康检查
curl -k https://127.0.0.1/api/hello

# 服务状态
curl -k https://127.0.0.1/

# 模型列表
curl -k https://127.0.0.1/v1/models
```

### 核心API测试

```bash
# 简单对话
curl -k -X POST https://127.0.0.1/v1/messages \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-sonnet-4-5-20250929",
    "max_tokens": 50,
    "messages": [{"role": "user", "content": "1+1=?"}]
  }'
```

### 流式响应测试

```bash
curl -k -X POST https://127.0.0.1/v1/messages \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-sonnet-4-5-20250929",
    "max_tokens": 50,
    "stream": true,
    "messages": [{"role": "user", "content": "say hello"}]
  }'
```

### 工具调用测试

```bash
curl -k -X POST https://127.0.0.1/v1/messages \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-sonnet-4-5-20250929",
    "max_tokens": 100,
    "messages": [{"role": "user", "content": "What'\''s the weather?"}],
    "tools": [{
      "name": "get_weather",
      "description": "Get weather info",
      "input_schema": {
        "type": "object",
        "properties": {
          "location": {"type": "string"}
        }
      }
    }]
  }'
```

### DNS劫持测试

```bash
# 1. 配置hosts
echo "59.110.40.73  api.anthropic.com" | sudo tee -a /etc/hosts

# 2. 测试
curl -k https://api.anthropic.com/v1/models

# 3. 清理
sudo sed -i '/api.anthropic.com/d' /etc/hosts
```

---

## 🌐 外网测试

测试公网访问是否正常：

```bash
# HTTP 3001端口
curl http://59.110.40.73:3001/api/hello

# HTTPS 443端口
curl -k https://59.110.40.73/api/hello

# HTTPS 443端口 - 直接访问核心API
curl -k https://59.110.40.73/v1/models

# HTTPS 443端口 - 核心消息API
curl -k -X POST https://59.110.40.73/v1/messages \
  -H "Content-Type: application/json" \
  -d '{"model":"claude-sonnet-4-5-20250929","max_tokens":50,"messages":[{"role":"user","content":"hi"}]}'
```

---

## 📱 Claude Code CLI 测试

### 安装CLI

```bash
npm install -g @anthropic-ai/claude-code
```

### 测试配置

```bash
# 方式1: HTTP直连
export ANTHROPIC_BASE_URL="http://59.110.40.73:3001"
export ANTHROPIC_API_KEY="sk-any"
claude-code "1+1=?"

# 方式2: HTTPS直接
export ANTHROPIC_BASE_URL="https://59.110.40.73"
export ANTHROPIC_API_KEY="sk-any"
export NODE_TLS_REJECT_UNAUTHORIZED=0
claude-code "say hello"

# 方式3: DNS劫持
echo "59.110.40.73  api.anthropic.com" | sudo tee -a /etc/hosts
export ANTHROPIC_API_KEY="sk-any"
export NODE_TLS_REJECT_UNAUTHORIZED=0
claude-code "what is 2+2?"
```

---

## 🔍 详细测试步骤

### 步骤1: 运行自动化测试

```bash
cd /root/dogeAI
bash test_api.sh
```

**预期输出**: 23个测试全部通过 ✅

---

### 步骤2: 验证核心功能

#### 2.1 模型映射

```bash
curl -k -X POST https://127.0.0.1/v1/messages \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-sonnet-4-5-20250929",
    "max_tokens": 50,
    "messages": [{"role": "user", "content": "hi"}]
  }' | python3.11 -m json.tool
```

**验证点**:
- ✅ 响应包含 `"model": "claude-sonnet-4-5-20250929"` (返回原始模型名)
- ✅ `"content"` 包含文本内容
- ✅ `"stop_reason"` 存在
- ✅ `"usage"` 包含token计数

#### 2.2 流式响应

```bash
curl -k -X POST https://127.0.0.1/v1/messages \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-sonnet-4-5-20250929",
    "max_tokens": 50,
    "stream": true,
    "messages": [{"role": "user", "content": "say hello"}]
  }' | grep -E "event:|data:" | head -20
```

**验证点**:
- ✅ 包含 `event: message_start`
- ✅ 包含 `event: content_block_delta`
- ✅ 包含 `event: message_delta`
- ✅ 包含 `event: message_stop`

#### 2.3 工具调用

```bash
curl -k -X POST https://127.0.0.1/v1/messages \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-sonnet-4-5-20250929",
    "max_tokens": 100,
    "messages": [{"role": "user", "content": "what is 2+2?"}],
    "tools": [{
      "name": "calculator",
      "description": "Calculate",
      "input_schema": {"type": "object", "properties": {}}
    }]
  }' | python3.11 -m json.tool
```

**验证点**:
- ✅ 响应包含工具调用或文本答案
- ✅ 格式符合Claude API规范

---

### 步骤3: 压力测试

```bash
# 并发测试 (使用ab工具)
ab -n 100 -c 10 -k -H "Content-Type: application/json" \
   -p test_request.json \
   https://127.0.0.1/api/hello

# 创建测试请求文件
cat > test_request.json << 'EOF'
{"model":"claude-sonnet-4-5-20250929","max_tokens":50,"messages":[{"role":"user","content":"hi"}]}
EOF
```

---

### 步骤4: 性能测试

```bash
# 测试响应时间
time curl -k -X POST https://127.0.0.1/v1/messages \
  -H "Content-Type: application/json" \
  -d '{"model":"claude-sonnet-4-5-20250929","max_tokens":50,"messages":[{"role":"user","content":"hi"}]}'

# 预期: < 1秒
```

---

## 🐛 故障排查

### 问题1: 测试失败

**检查服务状态**:
```bash
systemctl status claude-proxy
systemctl status nginx
```

**查看日志**:
```bash
journalctl -u claude-proxy -n 50
tail -f /root/dogeAI/logs/proxy.log
```

### 问题2: SSL证书错误

```bash
# 使用 -k 跳过SSL验证
curl -k https://127.0.0.1/api/hello
```

### 问题3: 连接超时

```bash
# 检查端口监听
ss -tlnp | grep -E ":(3001|443)"

# 检查防火墙
firewall-cmd --list-ports
```

---

## ✅ 测试检查清单

运行测试前请确认：

- [ ] 服务正在运行: `systemctl status claude-proxy`
- [ ] Nginx正在运行: `systemctl status nginx`
- [ ] 3001端口已监听: `ss -tlnp | grep 3001`
- [ ] 443端口已监听: `ss -tlnp | grep 443`
- [ ] 日志目录存在: `ls -la /root/dogeAI/logs/`

---

## 📊 测试报告

运行完所有测试后，您应该看到：

```
总测试数: 23
通过: 23
失败: 0
通过率: 100%

🎉 所有测试通过！API工作正常！
```

如果所有测试通过，说明：
- ✅ 所有端点响应正常
- ✅ API格式100%兼容Claude Code
- ✅ 模型映射正确工作
- ✅ 流式响应正常
- ✅ 工具调用正常
- ✅ DNS劫持可用

---

## 🎓 下一步

测试通过后，您可以：

1. **配置DNS劫持** (推荐)
   ```bash
   echo "59.110.40.73  api.anthropic.com" >> /etc/hosts
   ```

2. **使用Claude Code CLI**
   ```bash
   export ANTHROPIC_API_KEY="sk-any"
   export NODE_TLS_REJECT_UNAUTHORIZED=0
   claude-code
   ```

3. **集成到应用中**
   - Python: 使用 `anthropic` SDK
   - JavaScript: 使用 `@anthropic-ai/sdk`
   - 其他: 使用HTTP API

---

**测试文件位置**: `/root/dogeAI/test_api.sh`
**文档更新时间**: 2026-02-17
