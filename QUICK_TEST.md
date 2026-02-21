# 🧪 快速测试命令

## ⚡ 30秒快速验证

```bash
cd /root/dogeAI && bash quick_test.sh
```

---

## 📋 分类测试命令

### 1️⃣ 基础测试

```bash
# 健康检查
curl -k https://127.0.0.1/api/hello

# 服务状态
curl -k https://127.0.0.1/

# 模型列表
curl -k https://127.0.0.1/v1/models | python3.11 -m json.tool
```

### 2️⃣ 核心API测试

```bash
# 简单对话
curl -k -X POST https://127.0.0.1/v1/messages \
  -H "Content-Type: application/json" \
  -d '{"model":"claude-sonnet-4-5-20250929","max_tokens":50,"messages":[{"role":"user","content":"1+1=?"}]}'

# 流式响应
curl -k -X POST https://127.0.0.1/v1/messages \
  -H "Content-Type: application/json" \
  -d '{"model":"claude-sonnet-4-5-20250929","max_tokens":50,"stream":true,"messages":[{"role":"user","content":"say hello"}]}'
```

### 3️⃣ Claude Code端点

```bash
# 设置
curl -k https://127.0.0.1/api/claude_code/settings

# 策略
curl -k https://127.0.0.1/api/claude_code/policy_limits

# Penguin模式
curl -k https://127.0.0.1/api/claude_code/penguin_mode
```

### 4️⃣ 外网测试

```bash
# HTTP 3001端口
curl http://59.110.40.73:3001/api/hello

# HTTPS 443端口
curl -k https://59.110.40.73/api/hello

# HTTPS 443端口 - 核心API
curl -k -X POST https://59.110.40.73/v1/messages \
  -H "Content-Type: application/json" \
  -d '{"model":"claude-sonnet-4-5-20250929","max_tokens":50,"messages":[{"role":"user","content":"hi"}]}'
```

---

## 🚀 完整自动化测试

```bash
cd /root/dogeAI

# Bash版本 (推荐，无依赖)
bash test_api.sh

# Python版本 (需要安装requests)
pip3.11 install requests
python3.11 test_api.py
```

**预期输出**:
```
总测试数: 23
通过: 23
失败: 0
🎉 所有测试通过！
```

---

## 🔍 验证检查清单

运行测试前检查：

```bash
# 1. 服务状态
systemctl status claude-proxy
systemctl status nginx

# 2. 端口监听
ss -tlnp | grep -E ":(3001|443)"

# 3. 运行测试
bash quick_test.sh
```

---

## 📱 Claude Code CLI测试

```bash
# 安装
npm install -g @anthropic-ai/claude-code

# 配置
export ANTHROPIC_BASE_URL="https://59.110.40.73"
export ANTHROPIC_API_KEY="sk-any"
export NODE_TLS_REJECT_UNAUTHORIZED=0

# 测试
claude-code "what is 2+2?"
```

---

## 🎯 测试结果解读

### ✅ 成功指标

- 所有测试返回200状态码
- 响应包含预期字段
- 模型映射正确 (claude-sonnet-4-5 → qwen-plus)
- 格式转换正确 (Anthropic ↔ OpenAI)

### ❌ 失败诊断

如果测试失败：

1. **查看服务状态**
   ```bash
   systemctl status claude-proxy
   systemctl status nginx
   ```

2. **查看日志**
   ```bash
   journalctl -u claude-proxy -n 50
   tail -f /root/dogeAI/logs/proxy.log
   ```

3. **检查端口**
   ```bash
   ss -tlnp | grep -E ":(3001|443)"
   ```

---

## 📞 需要帮助？

- **完整测试指南**: `cat TEST_GUIDE.md`
- **API兼容性**: `cat API_COMPARISON.md`
- **DNS劫持配置**: `cat DNS_HIJACK_GUIDE.md`
- **服务管理**: `./service-manage.sh status`
