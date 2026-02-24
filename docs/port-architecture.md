# DogeAI 中转系统 - 端口架构说明

## 当前架构

### 端口分配

| 端口 | 服务 | 用途 | 状态 |
|------|------|------|------|
| 80 | Nginx (HTTP) | 对外 HTTP 入口 | ✅ 配置 |
| 443 | Nginx (HTTPS) | 对外 HTTPS 入口 | ⚠️ 待配置 SSL |
| 8080 | Python 代理服务 (main) | 原 Python 中转服务 | ✅ 运行 |
| 8081 | Rust API 网关 | Rust 中转服务 | ✅ 配置 |

### 1. Nginx (端口 80/443)

**配置文件**:
- `/etc/nginx/nginx.conf` - 主配置
- `/etc/nginx/sites-available/api-gateway` - API 网关配置
- `/etc/nginx/sites-available/aitachi.top` - 主域名配置

**功能**:
- 反向代理到后端服务
- SSL/TLS 终止 (443)
- CORS 处理
- SSE 流式响应支持
- 请求头传递

**关键路由**:

```nginx
# API Gateway (端口 80) -> 后端 8080
location /v1/messages      -> 127.0.0.1:8080 (Python)
location /v1/             -> 127.0.0.1:8080
location /api/            -> 127.0.0.1:8080

# 主域名 (端口 80) -> Rust 后端 8081
location /api/admin/      -> 127.0.0.1:8081 (Rust)
location /api/user/       -> 127.0.0.1:8081
location /v1/             -> 127.0.0.1:8081
```

### 2. Python 代理服务 (端口 8080)

**配置** (来自 `main` 分支):
```python
CONFIG = {
    "PROXY_PORT": 8080,
    "PROXY_HOST": "0.0.0.0",
    "UPSTREAM_BASE_URL": "https://open.bigmodel.cn/api/anthropic",
}
```

**端点**:
- `POST /v1/messages` - Anthropic 消息 API
- `GET /v1/models` - 模型列表
- `GET /health` - 健康检查
- `POST /apply` - 申请 API 密钥

### 3. Rust API 网关 (端口 8081)

**配置** (来自 `rust` 分支):
```rust
server_addr: "0.0.0.0:8081"
```

**端点**:
- `POST /v1/messages` - Anthropic 消息 API
- `GET /v1/models` - 模型列表
- `GET /health` - 健康检查
- `/api/*` - Claude Code 专用端点
- `/api/admin/*` - 管理端点
- `/api/user/*` - 用户端点

---

## 推荐的统一架构

### 方案 A: 单一 Rust 服务 (推荐)

```
┌─────────────┐
│   Nginx     │
│   :80/:443  │
└──────┬──────┘
       │
       ▼
┌─────────────────────────────────┐
│     Rust API Gateway            │
│     (统一中转服务)               │
│     :8081                       │
├─────────────────────────────────┤
│ • POST /v1/messages             │
│ • GET /v1/models                │
│ • /api/* (Claude Code)          │
│ • /api/admin/* (管理)           │
│ • /api/user/* (用户)            │
└─────────────────────────────────┘
```

**步骤**:
1. 将 Nginx 配置统一指向 8081
2. 停用 Python 服务 (8080)
3. 确保 Rust 实现所有功能

### 方案 B: 渐进式迁移 (当前)

```
┌─────────────┐
│   Nginx     │
│   :80/:443  │
└──────┬──────┘
       │
       ├──► :8080 (Python) - 传统 API
       │     • /v1/messages
       │     • /v1/models
       │     • /apply
       │
       └──► :8081 (Rust) - 新 API
             • /api/* (Claude Code)
             • /api/admin/*
             • /api/user/*
```

---

## SSL/HTTPS 配置 (443 端口)

当前 443 端口未配置 SSL。建议使用 Let's Encrypt:

```bash
# 安装 certbot
apt install certbot python3-certbot-nginx

# 获取证书
certbot --nginx -d aitachi.top -d www.aitachi.top

# 自动续期
certbot renew --dry-run
```

---

## 端口检查命令

```bash
# 检查端口占用
netstat -tlnp | grep -E '80|443|8080|8081'

# 检查服务状态
systemctl status nginx
systemctl status python-proxy  # 如有
systemctl status rust-gateway  # 如有

# 测试端点
curl http://localhost:8080/health
curl http://localhost:8081/health
curl http://localhost/health
```

---

## 配置文件修改

### 修改 Nginx 统一指向 Rust 服务

编辑 `/etc/nginx/sites-available/api-gateway`:

```nginx
upstream api_backend {
    server 127.0.0.1:8081;  # 改为 8081
    keepalive 64;
}
```

编辑 `/etc/nginx/sites-available/aitachi.top`:

```nginx
# 所有 location 已经正确指向 8081，无需修改
```

---

生成时间: 2026-02-24
