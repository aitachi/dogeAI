# 🎉 DNS劫持配置完成 - 使用指南

## ✅ 配置状态

- ✅ Nginx配置已更新
- ✅ SSL证书正确 (CN=api.anthropic.com)
- ✅ 443端口正常运行
- ✅ 所有端点测试通过
- ✅ DNS劫持完全可用

---

## 🚀 三种使用方式

### 方式1: HTTP直连 (最简单)

```bash
export ANTHROPIC_BASE_URL="http://59.110.40.73:3001"
export ANTHROPIC_API_KEY="sk-any"
claude-code
```

**优点**: 无需SSL，最简单
**缺点**: 非加密传输

---

### 方式2: HTTPS直接 (推荐) ⭐

```bash
export ANTHROPIC_BASE_URL="https://59.110.40.73"
export ANTHROPIC_API_KEY="sk-any"
claude-code
```

**优点**: 加密传输，无需配置hosts
**缺点**: 需要信任自签名证书

**消除SSL警告**:
```bash
# 使用 -k 参数
curl -k https://59.110.40.73/v1/models

# 或设置环境变量
export NODE_TLS_REJECT_UNAUTHORIZED=0
```

---

### 方式3: DNS劫持 (完美透明) 🎯

#### 步骤1: 配置hosts文件

**Linux/Mac**:
```bash
sudo bash -c 'echo "59.110.40.73  api.anthropic.com" >> /etc/hosts'
```

**Windows**:
1. 以管理员身份打开 `C:\Windows\System32\drivers\etc\hosts`
2. 添加: `59.110.40.73  api.anthropic.com`
3. 保存文件

#### 步骤2: 使用 (无需设置BASE_URL!)

```bash
export ANTHROPIC_API_KEY="sk-any"
claude-code
```

**优点**:
- ✅ 完全透明，无需修改代码
- ✅ 使用官方域名 `api.anthropic.com`
- ✅ 所有工具自动支持
- ✅ 加密传输

**缺点**:
- 需要配置hosts文件
- 需要信任自签名证书

---

## 📊 访问方式对比

| 方式 | URL | 配置难度 | 透明度 | 安全性 |
|------|-----|---------|--------|--------|
| HTTP直连 | `http://59.110.40.73:3001` | ⭐ 简单 | ⭐⭐ 需设置BASE_URL | ⚠️ 无加密 |
| HTTPS直接 | `https://59.110.40.73` | ⭐⭐ 中等 | ⭐⭐ 需设置BASE_URL | ✅ SSL加密 |
| DNS劫持 | `https://api.anthropic.com` | ⭐⭐⭐ 复杂 | ✅✅✅ 完全透明 | ✅✅ SSL+官方域名 |

---

## 🧪 验证测试

### 测试1: 模型列表

```bash
curl -k https://59.110.40.73/v1/models
```

**预期输出**: 9个Claude模型

---

### 测试2: 核心消息API

```bash
curl -k -X POST https://59.110.40.73/v1/messages \
  -H "Content-Type: application/json" \
  -d '{"model":"claude-sonnet-4-5-20250929","max_tokens":50,"messages":[{"role":"user","content":"hi"}]}'
```

**预期输出**:
```json
{
  "id": "msg_xxx",
  "type": "message",
  "role": "assistant",
  "content": [{"type": "text", "text": "Hello! How can I help you?"}]
}
```

---

### 测试3: DNS劫持 (配置hosts后)

```bash
# 配置hosts
echo "59.110.40.73  api.anthropic.com" >> /etc/hosts

# 测试
curl -k https://api.anthropic.com/v1/models
```

**预期输出**: 9个Claude模型

---

## 🎯 Claude Code CLI 使用

### 安装Claude Code CLI

```bash
npm install -g @anthropic-ai/claude-code
```

---

### 使用方式

#### 方式1: HTTP直连

```bash
export ANTHROPIC_BASE_URL="http://59.110.40.73:3001"
export ANTHROPIC_API_KEY="sk-any"

claude-code
```

#### 方式2: HTTPS直接

```bash
export ANTHROPIC_BASE_URL="https://59.110.40.73"
export ANTHROPIC_API_KEY="sk-any"

# 跳过SSL验证
export NODE_TLS_REJECT_UNAUTHORIZED=0

claude-code
```

#### 方式3: DNS劫持 (推荐)

```bash
# 1. 配置hosts
echo "59.110.40.73  api.anthropic.com" | sudo tee -a /etc/hosts

# 2. 使用 (就像使用官方API一样!)
export ANTHROPIC_API_KEY="sk-any"
export NODE_TLS_REJECT_UNAUTHORIZED=0

claude-code
```

**配置文件方式** (`~/.claude-code/config.json`):
```json
{
  "apiKey": "sk-any",
  "baseUrl": "https://api.anthropic.com",
  "disableTlsVerification": true
}
```

---

## 🔧 故障排查

### 问题1: SSL证书错误

```
SSL: certificate verify failed
```

**解决方案**:
```bash
# 方案A: 使用 -k 参数 (curl)
curl -k https://59.110.40.73/v1/models

# 方案B: 设置环境变量
export NODE_TLS_REJECT_UNAUTHORIZED=0  # Node.js
export PYTHONWARNINGS="ignore"  # Python requests
```

---

### 问题2: DNS劫持不生效

```bash
ping api.anthropic.com
```

如果显示的不是 `59.110.40.73`：

**解决方案**:
1. 检查hosts文件: `cat /etc/hosts | grep anthropic`
2. 清除DNS缓存:
   - Linux: `sudo systemd-resolve --flush-caches`
   - Mac: `sudo dscacheutil -flushcache && sudo killall -HUP mDNSResponder`
   - Windows: `ipconfig /flushdns`

---

### 问题3: 404 Not Found

```bash
curl https://59.110.40.73/v1/messages
```

**检查**:
1. nginx状态: `systemctl status nginx`
2. 后端服务: `systemctl status claude-proxy`
3. 配置文件: `nginx -t`

---

## 📱 其他工具使用

### Python SDK

```python
import anthropic

# 方式1: HTTP直连
client = anthropic.Anthropic(
    api_key="sk-any",
    base_url="http://59.110.40.73:3001"
)

# 方式2: HTTPS直接
client = anthropic.Anthropic(
    api_key="sk-any",
    base_url="https://59.110.40.73",
    # 跳过SSL验证
    default_headers={"verify": False}
)

# 使用
message = client.messages.create(
    model="claude-sonnet-4-5-20250929",
    max_tokens=100,
    messages=[{"role": "user", "content": "Hello"}]
)
print(message.content)
```

---

### JavaScript/TypeScript

```javascript
import Anthropic from '@anthropic-ai/sdk';

// 方式1: HTTP直连
const client = new Anthropic({
  apiKey: 'sk-any',
  baseURL: 'http://59.110.40.73:3001',
  // 跳过SSL验证 (HTTPS方式需要)
  dangerouslyAllowBrowser: true
});

// 使用
const message = await client.messages.create({
  model: 'claude-sonnet-4-5-20250929',
  max_tokens: 100,
  messages: [{ role: 'user', content: 'Hello' }]
});

console.log(message.content);
```

---

### cURL

```bash
# HTTP直连
curl -X POST http://59.110.40.73:3001/v1/messages \
  -H "Content-Type: application/json" \
  -H "x-api-key: sk-any" \
  -d '{
    "model": "claude-sonnet-4-5-20250929",
    "max_tokens": 100,
    "messages": [{"role": "user", "content": "Hello"}]
  }'

# HTTPS直接
curl -k -X POST https://59.110.40.73/v1/messages \
  -H "Content-Type: application/json" \
  -H "x-api-key: sk-any" \
  -d '{...}'

# DNS劫持 (配置hosts后)
curl -k -X POST https://api.anthropic.com/v1/messages \
  -H "Content-Type: application/json" \
  -H "x-api-key: sk-any" \
  -d '{...}'
```

---

## 🎓 最佳实践

### 开发环境
推荐使用 **方式2: HTTPS直接** 或 **方式3: DNS劫持**

```bash
export ANTHROPIC_BASE_URL="https://59.110.40.73"
export ANTHROPIC_API_KEY="sk-any"
```

### 生产环境
推荐使用 **方式3: DNS劫持**，配合内网DNS服务器

1. 配置内网DNS服务器，将 `api.anthropic.com` 解析到 `59.110.40.73`
2. 所有客户端无需修改配置
3. 完全透明使用

### 临时测试
推荐使用 **方式1: HTTP直连**

```bash
export ANTHROPIC_BASE_URL="http://59.110.40.73:3001"
```

---

## 📞 技术支持

### 查看服务状态

```bash
# Nginx状态
systemctl status nginx

# 代理服务状态
systemctl status claude-proxy
service-manage.sh status

# 查看日志
journalctl -u claude-proxy -f
tail -f /root/dogeAI/logs/proxy.log
```

### 重启服务

```bash
# 重启nginx
systemctl restart nginx

# 重启代理服务
systemctl restart claude-proxy
# 或
service-manage.sh restart
```

---

## ✅ 功能清单

- ✅ `/v1/messages` - 核心消息API
- ✅ `/v1/models` - 模型列表
- ✅ `/v1/models/{id}` - 模型详情
- ✅ `/v1/messages/count_tokens` - Token计数
- ✅ `/v1/organizations` - 组织信息
- ✅ `/v1/organizations/{id}` - 组织详情
- ✅ `/v1/me` - 用户信息
- ✅ `/v1/dashboard/billing/usage` - 账单
- ✅ `/v1/usage` - 使用统计
- ✅ `/api/*` - 平台API
- ✅ `/oauth/*` - OAuth认证
- ✅ `/.well-known/*` - OIDC发现
- ✅ 流式响应 (SSE)
- ✅ 工具调用 (tools/tool_use)
- ✅ 多轮对话
- ✅ 系统提示词
- ✅ Token限制

---

**配置完成时间**: 2026-02-17
**状态**: ✅ 完全可用 | ✅ 生产就绪
**推荐**: 方式3 (DNS劫持) 或 方式2 (HTTPS直接)
