# Anthropic API 中转代理 - 客户端使用指南

## 🎯 服务器信息

- **服务器IP**: `59.110.40.73`
- **代理端口**: `8081` (Nginx) / `8080` (直连)
- **状态**: ✅ 运行中

## 📝 服务器测试Token

```
sk-test-token-c226abf5a0b69e4a50d4fa7a55f3acbe
```

⚠️ **注意**: 这是默认测试token，请在生产环境中联系管理员获取专用token。

---

## 🔧 客户端配置方法

### 方法1: Claude Code (推荐)

在你的**客户端机器**上配置环境变量：

```bash
# 临时配置（当前终端会话）
export ANTHROPIC_API_KEY="sk-test-token-c226abf5a0b69e4a50d4fa7a55f3acbe"
export ANTHROPIC_BASE_URL="http://59.110.40.73:8081"

# 永久配置（添加到 ~/.bashrc 或 ~/.zshrc）
echo 'export ANTHROPIC_API_KEY="sk-test-token-c226abf5a0b69e4a50d4fa7a55f3acbe"' >> ~/.bashrc
echo 'export ANTHROPIC_BASE_URL="http://59.110.40.73:8081"' >> ~/.bashrc
source ~/.bashrc
```

### 方法2: Python 应用

```python
import os
from anthropic import Anthropic

# 配置API
client = Anthropic(
    api_key="sk-test-token-c226abf5a0b69e4a50d4fa7a55f3acbe",
    base_url="http://59.110.40.73:8081"
)

# 使用示例
message = client.messages.create(
    model="claude-3-5-sonnet-20241022",
    max_tokens=1024,
    messages=[
        {"role": "user", "content": "你好，请介绍一下你自己"}
    ]
)

print(message.content)
```

### 方法3: cURL 测试

```bash
curl -X POST http://59.110.40.73:8081/v1/messages \
  -H "x-api-key: sk-test-token-c226abf5a0b69e4a50d4fa7a55f3acbe" \
  -H "anthropic-version: 2023-06-01" \
  -H "content-type: application/json" \
  -d '{
    "model": "claude-3-5-sonnet-20241022",
    "max_tokens": 1024,
    "messages": [
      {"role": "user", "content": "你好"}
    ]
  }'
```

### 方法4: Node.js 应用

```javascript
const Anthropic = require('@anthropic-ai/sdk');

const client = new Anthropic({
  apiKey: 'sk-test-token-c226abf5a0b69e4a50d4fa7a55f3acbe',
  baseURL: 'http://59.110.40.73:8081',
});

async function main() {
  const message = await client.messages.create({
    model: 'claude-3-5-sonnet-20241022',
    max_tokens: 1024,
    messages: [{ role: 'user', content: 'Hello!' }],
  });

  console.log(message);
}

main();
```

---

## ✅ 快速测试

### 1. 测试连接

```bash
curl http://59.110.40.73:8081/health
```

**预期输出**:
```json
{
  "status": "healthy",
  "service": "Anthropic API Proxy",
  "upstream": "https://open.bigmodel.cn/api/anthropic"
}
```

### 2. 测试API调用

```bash
curl -X POST http://59.110.40.73:8081/v1/messages \
  -H "x-api-key: sk-test-token-c226abf5a0b69e4a50d4fa7a55f3acbe" \
  -H "content-type: application/json" \
  -d '{
    "model": "claude-3-5-sonnet-20241022",
    "max_tokens": 100,
    "messages": [{"role": "user", "content": "Say hi"}]
  }'
```

### 3. 在Claude Code中使用

```bash
# 配置环境变量后，直接使用claude命令
export ANTHROPIC_API_KEY="sk-test-token-c226abf5a0b69e4a50d4fa7a55f3acbe"
export ANTHROPIC_BASE_URL="http://59.110.40.73:8081"

# 启动Claude Code
claude
```

---

## 🔒 获取专用Token

如果你需要长期使用或更高的限额，请联系服务器管理员获取专用token。

管理员可以通过以下命令生成新token：

```bash
curl -X POST "http://59.110.40.73:8081/admin/tokens/generate?user=your_name&limit=1000000&days=365&admin_key=admin-change-this-key"
```

---

## 📊 支持的模型

- `claude-3-5-sonnet-20241022` - Claude 3.5 Sonnet（推荐）
- `claude-3-5-haiku-20241022` - Claude 3.5 Haiku
- `claude-3-opus-20240229` - Claude 3 Opus

查看完整模型列表：

```bash
curl http://59.110.40.73:8081/v1/models \
  -H "x-api-key: sk-test-token-c226abf5a0b69e4a50d4fa7a55f3acbe"
```

---

## 🛠️ 故障排查

### 问题1: 连接超时

```bash
# 检查服务器是否可达
ping 59.110.40.73

# 检查端口是否开放
telnet 59.110.40.73 8081
# 或
nc -zv 59.110.40.73 8081
```

### 问题2: 401 Unauthorized

**原因**: API密钥无效

**解决**:
- 检查 `ANTHROPIC_API_KEY` 是否正确
- 联系管理员确认token是否有效

### 问题3: 429 Too Many Requests

**原因**: 超过每日限额

**解决**:
- 等待第二天重置
- 或联系管理员提高限额

### 问题4: 流式响应中断

**原因**: 代理连接超时

**解决**:
- 确保网络稳定
- 检查防火墙设置

---

## 📖 完整配置示例

### macOS/Linux 用户

创建 `~/.anthropic_proxy_config`:

```bash
#!/bin/bash
export ANTHROPIC_API_KEY="sk-test-token-c226abf5a0b69e4a50d4fa7a55f3acbe"
export ANTHROPIC_BASE_URL="http://59.110.40.73:8081"
```

添加到 `~/.bashrc` 或 `~/.zshrc`:

```bash
source ~/.anthropic_proxy_config
```

### Windows PowerShell 用户

```powershell
# 设置环境变量
$env:ANTHROPIC_API_KEY="sk-test-token-c226abf5a0b69e4a50d4fa7a55f3acbe"
$env:ANTHROPIC_BASE_URL="http://59.110.40.73:8081"

# 永久设置
[System.Environment]::SetEnvironmentVariable('ANTHROPIC_API_KEY', 'sk-test-token-c226abf5a0b69e4a50d4fa7a55f3acbe', 'User')
[System.Environment]::SetEnvironmentVariable('ANTHROPIC_BASE_URL', 'http://59.110.40.73:8081', 'User')
```

---

## 🔧 高级配置

### 使用HTTPS（推荐）

如果你有域名，建议配置HTTPS：

```bash
# 安装certbot
yum install -y certbot

# 申请证书
certbot certonly --standalone -d your-domain.com

# 修改Nginx配置使用SSL
```

然后客户端配置为：

```bash
export ANTHROPIC_BASE_URL="https://your-domain.com"
```

### 配置代理缓存

服务器已配置Redis缓存，相同请求会更快响应。

### 监控使用情况

联系管理员查看你的API使用统计。

---

## 📞 技术支持

如有问题，请联系服务器管理员。

---

## ⚠️ 安全提醒

1. **不要分享你的API密钥**
2. **定期更换token**
3. **监控使用量**
4. **生产环境请使用专用token**

---

**文档更新时间**: 2025-01-29
**服务器版本**: v1.0.0
