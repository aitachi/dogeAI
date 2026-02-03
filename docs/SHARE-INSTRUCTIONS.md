# 📤 分发给其他用户的配置包

## 🔑 公开API密钥信息

```
API Key: sk-jU4cYDIDYoUgL0_2tQqdL1dCxrN2q-T9Qd3sGk4dMVw
服务器: http://59.110.40.73
每日限额: 5,000,000 tokens
有效期: 2027-01-29
```

---

## 📦 分发方式

### 方式1：发送配置脚本（推荐）

将 `/root/setup-client.sh` 发送给用户，让他们运行：

```bash
# 下载并运行
bash setup-client.sh
```

这个脚本会自动：
- 检测操作系统
- 配置环境变量
- 生成测试脚本
- 验证连接

### 方式2：发送完整指南

发送 `/root/CLIENT-SETUP-GUIDE.md` 文件给用户

### 方式3：快速配置命令

直接发送以下命令给用户：

**Linux/macOS:**
```bash
# 添加到 ~/.bashrc 或 ~/.zshrc
echo 'export ANTHROPIC_API_KEY="sk-jU4cYDIDYoUgL0_2tQqdL1dCxrN2q-T9Qd3sGk4dMVw"' >> ~/.bashrc
echo 'export ANTHROPIC_BASE_URL="http://59.110.40.73"' >> ~/.bashrc
source ~/.bashrc

# 验证
curl http://59.110.40.73/health
```

**Windows PowerShell:**
```powershell
[System.Environment]::SetEnvironmentVariable('ANTHROPIC_API_KEY', 'sk-jU4cYDIDYoUgL0_2tQqdL1dCxrN2q-T9Qd3sGk4dMVw', 'User')
[System.Environment]::SetEnvironmentVariable('ANTHROPIC_BASE_URL', 'http://59.110.40.73', 'User')
```

---

## 📧 邮件/消息模板

### 给用户的快速配置说明

```
你好！

我已经为你配置好了Claude AI API代理服务。以下是配置信息：

【服务器信息】
服务器地址: http://59.110.40.73
API密钥: sk-jU4cYDIDYoUgL0_2tQqdL1dCxrN2q-T9Qd3sGk4dMVw

【快速配置】
Linux/Mac用户在终端运行：
  export ANTHROPIC_API_KEY="sk-jU4cYDIDYoUgL0_2tQqdL1dCxrN2q-T9Qd3sGk4dMVw"
  export ANTHROPIC_BASE_URL="http://59.110.40.73"

Windows用户在PowerShell运行：
  [System.Environment]::SetEnvironmentVariable('ANTHROPIC_API_KEY', 'sk-jU4cYDIDYoUgL0_2tQqdL1dCxrN2q-T9Qd3sGk4dMVw', 'User')
  [System.Environment]::SetEnvironmentVariable('ANTHROPIC_BASE_URL', 'http://59.110.40.73', 'User')

【验证配置】
运行: curl http://59.110.40.73/health

【可用模型】
- claude-sonnet-4.5 (推荐)
- claude-opus-4.5
- claude-3.5-sonnet
- claude-3.5-haiku
- claude-3-opus

【使用示例】
Python代码：
  from anthropic import Anthropic
  client = Anthropic(
      api_key="sk-jU4cYDIDYoUgL0_2tQqdL1dCxrN2q-T9Qd3sGk4dMVw",
      base_url="http://59.110.40.73"
  )
  message = client.messages.create(
      model="claude-sonnet-4.5",
      max_tokens=1024,
      messages=[{"role": "user", "content": "你好"}]
  )

【技术支持】
- 服务状态: http://59.110.40.73/health
- API文档: http://59.110.40.73/docs

每日限额: 5,000,000 tokens
有效期至: 2027-01-29

如有问题，请随时联系！
```

---

## 📊 监控此密钥使用情况

```bash
curl "http://59.110.40.73/admin/stats?api_key=sk-jU4cYDIDYoUgL0_2tQqdL1dCxrN2q-T9Qd3sGk4dMVw&days=7" \
  -H "x-admin-key: admin-change-this-key"
```

---

## 🔐 管理此密钥

### 查看密钥详情
```bash
curl "http://59.110.40.73/admin/keys" \
  -H "x-admin-key: admin-change-this-key" | grep claude_code_public
```

### 撤销密钥（如需要）
```bash
sqlite3 /var/lib/anthropic-proxy/stats.db "UPDATE api_keys SET is_active=0 WHERE user='claude_code_public';"
```

### 修改限额
```bash
sqlite3 /var/lib/anthropic-proxy/stats.db "UPDATE api_keys SET daily_limit=10000000 WHERE user='claude_code_public';"
```

---

## 📁 相关文件

- `/root/CLIENT-SETUP-GUIDE.md` - 完整客户端配置指南
- `/root/setup-client.sh` - 自动配置脚本
- `/root/SHARE-INSTRUCTIONS.md` - 此文件

---

## ⚠️ 安全提醒

1. 此密钥已公开，任何人都可以使用
2. 监控使用情况，防止滥用
3. 定期轮换密钥
4. 如发现异常，立即撤销

---

**生成时间**: 2026-01-29
**密钥用户**: claude_code_public
**状态**: ✅ 激活
