# Claude Code 连接指南

## 🔑 您的访问密钥

```
API Key: sk-jU4cYDIDYoUgL0_2tQqdL1dCxrN2q-T9Qd3sGk4dMVw
```

**配置信息：**
- 每日限额: 5,000,000 tokens
- 有效期: 2027-01-29
- 服务器: http://59.110.40.73

---

## 🚀 快速开始

### 方法1：环境变量配置（推荐）

**Linux/macOS:**
```bash
# 添加到 ~/.bashrc 或 ~/.zshrc
export ANTHROPIC_API_KEY="sk-jU4cYDIDYoUgL0_2tQqdL1dCxrN2q-T9Qd3sGk4dMVw"
export ANTHROPIC_BASE_URL="http://59.110.40.73"

# 立即生效
source ~/.bashrc  # 或 source ~/.zshrc
```

**Windows PowerShell:**
```powershell
# 设置环境变量（当前会话）
$env:ANTHROPIC_API_KEY="sk-jU4cYDIDYoUgL0_2tQqdL1dCxrN2q-T9Qd3sGk4dMVw"
$env:ANTHROPIC_BASE_URL="http://59.110.40.73"

# 永久设置
[System.Environment]::SetEnvironmentVariable('ANTHROPIC_API_KEY', 'sk-jU4cYDIDYoUgL0_2tQqdL1dCxrN2q-T9Qd3sGk4dMVw', 'User')
[System.Environment]::SetEnvironmentVariable('ANTHROPIC_BASE_URL', 'http://59.110.40.73', 'User')
```

### 方法2：Claude Code 配置文件

创建或编辑配置文件 `~/.claude/config.json`:

```json
{
  "api_key": "sk-jU4cYDIDYoUgL0_2tQqdL1dCxrN2q-T9Qd3sGk4dMVw",
  "base_url": "http://59.110.40.73"
}
```

### 方法3：命令行参数

```bash
claude --api-key sk-jU4cYDIDYoUgL0_2tQqdL1dCxrN2q-T9Qd3sGk4dMVw --base-url http://59.110.40.73
```

---

## ✅ 验证连接

运行以下命令验证配置是否正确：

```bash
# 方法1：使用curl
curl -X POST http://59.110.40.73/v1/messages \
  -H "x-api-key: sk-jU4cYDIDYoUgL0_2tQqdL1dCxrN2q-T9Qd3sGk4dMVw" \
  -H "content-type: application/json" \
  -d '{"model":"claude-sonnet-4.5","max_tokens":100,"messages":[{"role":"user","content":"你好"}]}'

# 方法2：使用Python
python3 << 'EOF'
import os
from anthropic import Anthropic

client = Anthropic(
    api_key="sk-jU4cYDIDYoUgL0_2tQqdL1dCxrN2q-T9Qd3sGk4dMVw",
    base_url="http://59.110.40.73"
)

message = client.messages.create(
    model="claude-sonnet-4.5",
    max_tokens=100,
    messages=[{"role": "user", "content": "你好，请介绍一下你自己"}]
)

print(message.content[0].text)
EOF
```

---

## 📦 可用模型

- `claude-sonnet-4.5` - Claude Sonnet 4.5 (最新，推荐)
- `claude-opus-4.5` - Claude Opus 4.5 (最新)
- `claude-3.5-sonnet` - Claude 3.5 Sonnet
- `claude-3.5-haiku` - Claude 3.5 Haiku
- `claude-3-opus` - Claude 3 Opus

---

## 💡 使用示例

### Python 代码
```python
from anthropic import Anthropic

# 初始化客户端
client = Anthropic(
    api_key="sk-jU4cYDIDYoUgL0_2tQqdL1dCxrN2q-T9Qd3sGk4dMVw",
    base_url="http://59.110.40.73"
)

# 创建消息
response = client.messages.create(
    model="claude-sonnet-4.5",
    max_tokens=1024,
    messages=[
        {"role": "user", "content": "帮我写一个Python函数计算斐波那契数列"}
    ]
)

print(response.content[0].text)
```

### JavaScript/Node.js
```javascript
const Anthropic = require('@anthropic-ai/sdk');

const client = new Anthropic({
  apiKey: 'sk-jU4cYDIDYoUgL0_2tQqdL1dCxrN2q-T9Qd3sGk4dMVw',
  baseURL: 'http://59.110.40.73',
});

async function main() {
  const message = await client.messages.create({
    model: 'claude-sonnet-4.5',
    max_tokens: 1024,
    messages: [{ role: 'user', content: '你好！' }],
  });

  console.log(message.content[0].text);
}

main();
```

---

## 🔧 故障排除

### 问题1：连接超时
**解决方案：**
- 检查网络连接
- 确认服务器地址正确: `http://59.110.40.73`
- 尝试使用备用端口: `http://59.110.40.73:8081`

### 问题2：认证失败
**解决方案：**
- 确认API密钥正确: `sk-jU4cYDIDYoUgL0_2tQqdL1dCxrN2q-T9Qd3sGk4dMVw`
- 检查环境变量是否正确设置

### 问题3：达到限额
**解决方案：**
- 联系管理员增加限额
- 每日限额: 5,000,000 tokens
- 使用 `curl http://59.110.40.73/health` 检查服务状态

---

## 📊 查看使用统计

```bash
# 查看当前密钥的使用情况
curl "http://59.110.40.73/admin/stats?api_key=sk-jU4cYDIDYoUgL0_2tQqdL1dCxrN2q-T9Qd3sGk4dMVw&days=7" \
  -H "x-admin-key: admin-change-this-key"
```

---

## 🔐 安全提示

1. **不要分享您的API密钥**
2. **定期轮换密钥**
3. **监控使用情况**
4. **如果密钥泄露，立即联系管理员撤销**

---

## 📞 技术支持

- **服务状态**: http://59.110.40.73/health
- **API文档**: http://59.110.40.73/docs
- **模型列表**: http://59.110.40.73/v1/models

---

**生成时间**: 2026-01-29
**有效期至**: 2027-01-29
**版本**: v2.0.0
