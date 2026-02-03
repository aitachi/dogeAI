# 📤 分发给用户的快速配置指南

## 🌐 aitachi.cloud API代理服务

### 🔑 您的访问密钥

```
API Key: sk-JvflJvETSAx-50xJ_aAXdXESwfWgQX6bLZrjpJSbeFg
服务器地址: http://aitachi.cloud
```

---

## ⚡ 快速配置（30秒完成）

### Linux/Mac 用户
```bash
export ANTHROPIC_API_KEY="sk-JvflJvETSAx-50xJ_aAXdXESwfWgQX6bLZrjpJSbeFg"
export ANTHROPIC_BASE_URL="http://aitachi.cloud"
```

### Windows PowerShell 用户
```powershell
[System.Environment]::SetEnvironmentVariable('ANTHROPIC_API_KEY', 'sk-JvflJvETSAx-50xJ_aAXdXESwfWgQX6bLZrjpJSbeFg', 'User')
[System.Environment]::SetEnvironmentVariable('ANTHROPIC_BASE_URL', 'http://aitachi.cloud', 'User')
```

---

## ✅ 验证配置

```bash
curl http://aitachi.cloud/health
```

---

## 💻 使用示例

### Python
```python
from anthropic import Anthropic

client = Anthropic(
    api_key="sk-JvflJvETSAx-50xJ_aAXdXESwfWgQX6bLZrjpJSbeFg",
    base_url="http://aitachi.cloud"
)

message = client.messages.create(
    model="claude-sonnet-4.5",
    max_tokens=1024,
    messages=[{"role": "user", "content": "你好"}]
)

print(message.content[0].text)
```

---

## 📊 可用模型

- `claude-sonnet-4.5` (推荐)
- `claude-opus-4.5`
- `claude-3.5-sonnet`
- `claude-3.5-haiku`
- `claude-3-opus`

---

## 🔒 安全说明

⚠️ **重要**: 所有API请求必须提供有效的API Key，否则将被拒绝。

---

**服务地址**: http://aitachi.cloud
**健康检查**: http://aitachi.cloud/health
