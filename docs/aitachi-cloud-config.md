# 🌐 aitachi.cloud API代理服务 - 配置文档

## 🎉 配置完成摘要

**域名**: aitachi.cloud
**服务器**: 59.110.40.73
**新API Key**: `sk-JvflJvETSAx-50xJ_aAXdXESwfWgQX6bLZrjpJSbeFg`
**安全级别**: ✅ 高（API Key验证 + 访问控制）

---

## 🔑 新的API密钥信息

```
API Key: sk-JvflJvETSAx-50xJ_aAXdXESwfWgQX6bLZrjpJSbeFg
用户: aitachi_cloud
每日限额: 10,000,000 tokens
有效期: 2027-01-29
状态: ✅ 已激活
```

---

## 🌐 访问地址

### 主域名（推荐）
```
Base URL: http://aitachi.cloud
```

### 子域名
```
主站: http://aitachi.cloud
API:  http://api.aitachi.cloud
WWW:  http://www.aitachi.cloud
```

### IP访问（自动重定向）
```
http://59.110.40.73 → http://aitachi.cloud
http://59.110.40.73:8081 → http://aitachi.cloud
```

---

## 🔒 安全特性

### 1. API Key验证
- ✅ 所有API请求必须提供有效的API Key
- ✅ 错误的Key将被拒绝
- ✅ 缺少Key将被拒绝

### 2. 访问控制
- ✅ 管理接口仅允许内网访问
- ✅ 健康检查接口公开访问
- ✅ IP访问自动重定向到域名

### 3. 限额管理
- ✅ 每日Token限额：10,000,000
- ✅ 超出限额自动拒绝
- ✅ 详细的使用统计

---

## 📦 客户端配置

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

### 环境变量
```bash
# Linux/macOS
export ANTHROPIC_API_KEY="sk-JvflJvETSAx-50xJ_aAXdXESwfWgQX6bLZrjpJSbeFg"
export ANTHROPIC_BASE_URL="http://aitachi.cloud"

# Windows PowerShell
[System.Environment]::SetEnvironmentVariable('ANTHROPIC_API_KEY', 'sk-JvflJvETSAx-50xJ_aAXdXESwfWgQX6bLZrjpJSbeFg', 'User')
[System.Environment]::SetEnvironmentVariable('ANTHROPIC_BASE_URL', 'http://aitachi.cloud', 'User')
```

### cURL
```bash
curl -X POST http://aitachi.cloud/v1/messages \
  -H "x-api-key: sk-JvflJvETSAx-50xJ_aAXdXESwfWgQX6bLZrjpJSbeFg" \
  -H "anthropic-version: 2023-06-01" \
  -H "content-type: application/json" \
  -d '{
    "model": "claude-sonnet-4.5",
    "max_tokens": 1024,
    "messages": [{"role": "user", "content": "你好"}]
  }'
```

---

## 📊 可用模型

- ✅ `claude-sonnet-4.5` - Claude Sonnet 4.5 (推荐)
- ✅ `claude-opus-4.5` - Claude Opus 4.5 (最新)
- ✅ `claude-3.5-sonnet` - Claude 3.5 Sonnet
- ✅ `claude-3.5-haiku` - Claude 3.5 Haiku
- ✅ `claude-3-opus` - Claude 3 Opus

---

## 🧪 验证配置

### 1. 健康检查
```bash
curl http://aitachi.cloud/health
```

### 2. 模型列表
```bash
curl http://aitachi.cloud/v1/models \
  -H "x-api-key: sk-JvflJvETSAx-50xJ_aAXdXESwfWgQX6bLZrjpJSbeFg"
```

### 3. API测试
```bash
curl -X POST http://aitachi.cloud/v1/messages \
  -H "x-api-key: sk-JvflJvETSAx-50xJ_aAXdXESwfWgQX6bLZrjpJSbeFg" \
  -H "content-type: application/json" \
  -d '{"model":"claude-sonnet-4.5","max_tokens":50,"messages":[{"role":"user","content":"Hi"}]}'
```

---

## 🔧 管理功能

### 查看API使用统计
```bash
curl "http://aitachi.cloud/admin/stats?api_key=sk-JvflJvETSAx-50xJ_aAXdXESwfWgQX6bLZrjpJSbeFg&days=7" \
  -H "x-admin-key: admin-change-this-key"
```

### 查看所有API密钥
```bash
curl "http://aitachi.cloud/admin/keys" \
  -H "x-admin-key: admin-change-this-key"
```

### 查看实时日志
```bash
journalctl -u anthropic-proxy -f
tail -f /var/log/anthropic-proxy/api-proxy.log
```

---

## ⚠️ 安全提示

1. **不要分享API密钥**
2. **定期监控使用情况**
3. **及时撤销未使用的密钥**
4. **管理接口仅允许内网访问**

---

## 🚨 错误处理

### 401 Unauthorized
```
原因：API密钥无效或缺失
解决：检查x-api-key头部是否正确设置
```

### 403 Forbidden
```
原因：API密钥已被禁用
解决：联系管理员重新激活
```

### 429 Too Many Requests
```
原因：超出每日限额
解决：等待第二天或联系管理员增加限额
```

---

## 📁 相关文件

- **服务代码**: `/root/anthropic_proxy_v2.py`
- **数据库**: `/var/lib/anthropic-proxy/stats.db`
- **日志**: `/var/log/anthropic-proxy/api-proxy.log`
- **Nginx配置**: `/etc/nginx/conf.d/anthropic-proxy-v2.conf`

---

## 🔄 迁移指南

### 从旧配置迁移

**旧配置:**
```python
api_key="sk-dMUtFZ9IWedPSgSd0WMkH7XhP0idZVtAVnp48MYgNLo"
base_url="http://59.110.40.73"
```

**新配置:**
```python
api_key="sk-JvflJvETSAx-50xJ_aAXdXESwfWgQX6bLZrjpJSbeFg"
base_url="http://aitachi.cloud"
```

### 环境变量迁移
```bash
# 删除旧的
unset ANTHROPIC_API_KEY
unset ANTHROPIC_BASE_URL

# 设置新的
export ANTHROPIC_API_KEY="sk-JvflJvETSAx-50xJ_aAXdXESwfWgQX6bLZrjpJSbeFg"
export ANTHROPIC_BASE_URL="http://aitachi.cloud"
```

---

## 📞 技术支持

- **服务状态**: http://aitachi.cloud/health
- **API文档**: http://aitachi.cloud/docs
- **管理工具**: `/root/api-proxy-admin.sh`

---

**配置完成时间**: 2026-01-29
**版本**: v2.0.0
**域名**: aitachi.cloud
