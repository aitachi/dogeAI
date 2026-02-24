# 狼狗AI API 代理系统 - 完整配置总结

## 一、问题解答

### Q: 需要购买 Claude Code 官方订阅吗？

**A: 不需要！**

我们有两种使用方式：

| 方式 | 是否需要官方账号 | 说明 |
|------|------------------|------|
| **命令行模式** (`claude -p`) | ❌ 不需要 | 直接使用我们生成的 API Key |
| **交互模式** (`claude`) | ✅ 需要 | Claude Code CLI 2.x 的交互模式强制官方登录 |

**推荐使用命令行模式**：
```bash
claude -p "你的问题"
```

完全免费，使用我们自己生成的 API Key！

---

## 二、系统架构

```
┌─────────────────────────────────────────────────────────────┐
│                        客户端机器                            │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐        │
│  │ Claude CLI  │  │  Python SDK │  │  Node.js    │        │
│  │ (命令行模式) │  │             │  │             │        │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘        │
│         │                 │                 │               │
│         └─────────────────┴─────────────────┘               │
│                           │                                  │
│                    x-api-key: sk-ant-api03-xxx...           │
└───────────────────────────┼──────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                    115.190.62.87 (服务器)                    │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              Nginx (443/80)                         │   │
│  │  SSL: 自签名证书 | 路由: /v1/* -> 8080              │   │
│  └────────────────────┬────────────────────────────────┘   │
│                       │                                      │
│                       ▼                                      │
│  ┌─────────────────────────────────────────────────────┐   │
│  │         Rust API Gateway (8080)                     │   │
│  │  ├─ API Key 验证 (sk-ant-api03-xxx)                │   │
│  │  ├─ JWT 认证                                        │   │
│  │  ├─ 请求路由 & 负载均衡                             │   │
│  │  └─ 模型资源池管理                                  │   │
│  └────────────────────┬────────────────────────────────┘   │
│                       │                                      │
│        ┌──────────────┼──────────────┐                      │
│        ▼              ▼              ▼                      │
│  ┌─────────┐   ┌─────────┐   ┌─────────┐                   │
│  │ Sonnet  │   │  Opus   │   │  Haiku  │                   │
│  │  Pool   │   │  Pool   │   │  Pool   │                   │
│  └────┬────┘   └────┬────┘   └────┬────┘                   │
│       │             │             │                         │
│       └─────────────┴─────────────┘                         │
│                     │                                       │
│                     ▼                                       │
│            ┌─────────────────┐                               │
│            │  AI Provider    │                               │
│            │  (智谱/其他)    │                               │
│            └─────────────────┘                               │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                    PostgreSQL 数据库                         │
│  ┌─────────────────────────────────────────────────────┐   │
│  │  users              - 用户信息                       │   │
│  │  user_api_keys       - API Key (sk-ant-api03-xxx)   │   │
│  │  model_providers     - 模型提供商                     │   │
│  │  request_logs       - 请求日志                       │   │
│  │  recharge_cards     - 充值卡                         │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

---

## 三、API Key 格式说明

### 旧格式（已弃用）
```
sk-ABC12345
长度: 约12字符
```

### 新格式（当前使用）
```
sk-ant-api03-9WQIGVGKX4AY31LQZV9STVJG439Y2NH67HNO1K3GLGD4YOIBHZQC
长度: 65字符
格式: sk-ant-api03- + 52个大写字母/数字
```

---

## 四、数据库更新（服务器端执行）

### 1. Ubuntu 命令

```bash
# 方式一：直接执行 SQL
sudo -u postgres psql -d api_gateway << 'EOF'
ALTER TABLE user_api_keys ALTER COLUMN api_key TYPE VARCHAR(128);
EOF

# 方式二：使用 SQL 文件
sudo -u postgres psql -d api_gateway -f /root/database-update.sql
```

### 2. 验证修改

```bash
sudo -u postgres psql -d api_gateway -c "
SELECT column_name, character_maximum_length
FROM information_schema.columns
WHERE table_name = 'user_api_keys' AND column_name = 'api_key';
"
```

预期输出：
```
column_name | character_maximum_length
-------------+--------------------------
api_key     |                      128
```

---

## 五、后端更新（服务器端执行）

```bash
cd /root/api-gateway-simple

# 1. 重新编译
cargo build --release

# 2. 重启服务
pkill -f api-gateway-simple
sleep 1
nohup ./target/release/api-gateway-simple > /tmp/api-gateway.log 2>&1 &

# 3. 验证服务
sleep 3
curl -s http://127.0.0.1:8080/health | jq '.'
```

---

## 六、前端更新（服务器端执行）

```bash
cd /root/frontend

# 重新构建
npm run build

# 自动部署到 /var/www/aitachi.top/new/
```

---

## 七、客户端配置（在客户端机器执行）

### 方式一：使用一键脚本

```bash
# 下载脚本
curl -O https://115.190.62.87/ubuntu-setup.sh

# 执行配置
bash ubuntu-setup.sh
```

### 方式二：手动配置

```bash
# 1. 配置环境变量
cat >> ~/.bashrc << 'EOF'
export ANTHROPIC_API_KEY="你的API_KEY"
export ANTHROPIC_BASE_URL="https://115.190.62.87/v1"
export NODE_TLS_REJECT_UNAUTHORIZED=0
EOF

source ~/.bashrc

# 2. 配置 Claude Code
mkdir -p ~/.config/claude
cat > ~/.config/claude/settings.json << 'EOF'
{
  "apiKey": "你的API_KEY",
  "baseUrl": "https://115.190.62.87/v1"
}
EOF

# 3. 测试
claude -p "Hello"
```

---

## 八、使用示例

### Claude Code CLI

```bash
# 命令行模式（推荐）
claude -p "解释这段代码"

# 管道输入
echo "2+2=?" | claude -p

# 多行输入
cat <<EOF | claude -p
请写一个 Python 函数
计算斐波那契数列
EOF
```

### Python SDK

```python
import anthropic
import httpx

# 禁用 SSL 验证
http_client = httpx.Client(verify=False)

client = anthropic.Anthropic(
    api_key="你的API_KEY",
    base_url="https://115.190.62.87/v1",
    http_client=http_client
)

response = client.messages.create(
    model="claude-sonnet-4-20250514",
    max_tokens=1000,
    messages=[{"role": "user", "content": "Hello!"}]
)

print(response.content[0].text)
```

### curl 直接调用

```bash
curl -k -X POST "https://115.190.62.87/v1/messages" \
  -H "x-api-key: 你的API_KEY" \
  -H "anthropic-version: 2023-06-01" \
  -H "content-type: application/json" \
  -d '{
    "model": "claude-sonnet-4-20250514",
    "max_tokens": 100,
    "messages": [{"role": "user", "content": "Hello!"}]
  }'
```

---

## 九、前端访问地址

| 页面 | URL |
|------|-----|
| 首页 | https://115.190.62.87/new/ |
| 客户端登录/注册 | https://115.190.62.87/new/client |
| Claude 文档 | https://115.190.62.87/new/docs/claude |
| GPT 文档 | https://115.190.62.87/new/docs/gpt |
| 管理后台登录 | https://115.190.62.87/new/admin/login |
| 管理后台首页 | https://115.190.62.87/new/admin/dashboard |

---

## 十、常见错误处理

### 错误 1: "缺少Authorization头或x-api-key头"

**原因**: Header 格式错误
**解决**: 使用 `-H "x-api-key: sk-xxx"` (不是 `-H "sk-xxx"`)

### 错误 2: "无法解析JWT"

**原因**: 后端没有更新到最新版本
**解决**: 重新编译并重启后端

### 错误 3: SSL 证书验证失败

**原因**: 使用自签名证书
**解决**: 使用 `-k` 参数或设置 `NODE_TLS_REJECT_UNAUTHORIZED=0`

### 错误 4: Claude Code 交互模式无法登录

**原因**: 新版 CLI 的交互模式需要官方账号
**解决**: 使用命令行模式 `claude -p "问题"`

---

## 十一、/v1/models 端点格式修复 (2026-02-09)

### 问题发现

原 `/v1/models` 返回格式与 Anthropic 官方 API 不一致：

```json
// ❌ 旧格式 (不兼容)
{
  "object": "list",        // 多余字段
  "data": [
    {
      "id": "claude-opus-4-6",
      "object": "model",    // 应该是 "type"
      "owned_by": "anthropic"  // 多余字段
    }
  ]
}
```

### 修复后格式

```json
// ✅ 新格式 (符合官方规范)
{
  "data": [
    {
      "id": "claude-opus-4-6",
      "created_at": "2026-02-04T00:00:00Z",  // RFC 3339 格式
      "display_name": "Claude Opus 4.6",      // 人类可读名称
      "type": "model"                          // 正确字段名
    }
  ],
  "first_id": "claude-opus-4-6",  // 分页支持
  "has_more": false,
  "last_id": "claude-3-opus-20240229"
}
```

### 修复内容

| 字段 | 官方要求 | 修复状态 |
|------|----------|----------|
| `type` | `"model"` | ✅ 从 `object` 改为 `type` |
| `created_at` | RFC 3339 日期时间 | ✅ 新增字段 |
| `display_name` | 人类可读名称 | ✅ 新增字段 |
| `first_id` | 分页用 | ✅ 新增字段 |
| `has_more` | `false` | ✅ 新增字段 |
| `last_id` | 分页用 | ✅ 新增字段 |
| `object` | 无 | ✅ 移除多余字段 |
| `owned_by` | 无 | ✅ 移除多余字段 |

### 验证命令

```bash
curl -k -s "https://115.190.62.87/v1/models" \
  -H "x-api-key: sk-ant-api03-CG0BG1GN0D4Q3CRWCREL0DIT2BHZCWSMQUF9SYOCU8V6K6A6Q5GA" | jq '.'
```

---

## 十二、文件清单

| 文件 | 位置 | 说明 |
|------|------|------|
| 后端源码 | /root/api-gateway-simple/src/ | Rust API Gateway |
| 前端源码 | /root/frontend/src/ | Vue 3 前端 |
| 前端部署 | /var/www/aitachi.top/new/ | 构建输出 |
| Nginx 配置 | /etc/nginx/sites-available/default-ip | 反向代理 |
| 配置脚本 | /root/ubuntu-setup.sh | 客户端配置脚本 |
| 数据库脚本 | /root/database-update.sql | 数据库更新 |
| 完整文档 | /root/FULL_SETUP_GUIDE.md | 详细配置指南 |
| 本文档 | /root/COMPLETE_SUMMARY.md | 完整总结 |

---

## 十三、快速检查清单

- [x] 数据库 api_key 列扩展到 128 字符
- [x] 后端代码更新并重新编译
- [x] 后端服务重启
- [x] 前端重新构建
- [x] 新用户注册生成 65 字符 Key
- [x] API 调用测试成功
- [x] 客户端配置脚本可用
- [x] 文档已更新
- [x] `/v1/models` 端点格式修复为 Anthropic 官方格式

---

**更新日期**: 2026-02-09
**版本**: 3.1.0 (修复 /v1/models 端点格式)
