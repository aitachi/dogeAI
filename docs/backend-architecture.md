# DogeAI 后端模块架构

## 目录结构

```
backend/
├── api-gateway-simple/    # 主 API 网关 (统一中转服务)
│   ├── src/
│   │   ├── main.rs       # 主入口
│   │   ├── handlers/     # 端点处理器
│   │   ├── middleware/   # 中间件
│   │   ├── models/       # 数据模型
│   │   ├── services/     # 业务服务
│   │   ├── jwt.rs        # JWT 认证
│   │   ├── provider.rs   # 提供商管理
│   │   └── anthropic.rs  # Anthropic API 兼容
│   └── Cargo.toml
│
├── auth-system/          # 认证系统 (独立模块)
│   ├── src/
│   │   ├── lib.rs        # 库入口
│   │   ├── service.rs    # 认证服务
│   │   ├── middleware.rs # 中间件
│   │   ├── cache.rs      # 缓存层
│   │   ├── models.rs     # 数据模型
│   │   └── error.rs      # 错误类型
│   └── Cargo.toml
│
├── billing-system/       # 计费系统 (独立模块)
│   ├── src/
│   │   ├── lib.rs        # 库入口
│   │   ├── billing/      # 计费逻辑
│   │   ├── key_manager/  # 密钥管理
│   │   ├── ratelimit/    # 速率限制
│   │   ├── recharge/     # 充值功能
│   │   ├── database/     # 数据库层
│   │   └── api/          # API 端点
│   └── Cargo.toml
│
└── cache/                # 缓存服务 (独立模块)
    ├── src/
    │   ├── lib.rs        # 库入口
    │   ├── config.rs     # 配置
    │   └── models.rs     # 数据模型
    └── Cargo.toml
```

## 模块职责

### 1. api-gateway-simple (主服务)

**职责**: 统一的 API 网关，处理所有外部请求

**端口**: 8080 (通过环境变量配置)

**主要功能**:
- Anthropic API 兼容 (`/v1/messages`, `/v1/models`)
- Claude Code 专用端点 (`/api/*`)
- 用户管理 (`/api/user/*`)
- 管理端点 (`/api/admin/*`)
- JWT 认证和验证
- 多提供商路由 (GLM, 千问, DeepSeek)

**依赖**:
- `auth-system`: JWT 认证
- `billing-system`: 计费和限流
- `redis`: 缓存和黑名单
- `postgresql`: 数据持久化

### 2. auth-system (认证库)

**职责**: 提供可复用的认证功能

**主要功能**:
- JWT 生成和验证
- 密钥轮换
- Token 撤销
- 本地缓存 (L1) + Redis (L2)
- 认证中间件

**使用方式**:
```rust
use auth_system::{JwtAuthService, AuthenticatedUser};

let auth = JwtAuthService::new(jwt_secret, redis_url);
let token = auth.create_token(&user_id, &username, &tier, scopes, version)?;
let user = auth.verify_token(&token)?;
```

### 3. billing-system (计费库)

**职责**: 处理计费、配额和充值

**主要功能**:
- 请求计费
- 配额管理
- 速率限制
- 充值卡管理
- 使用统计

**使用方式**:
```rust
use billing_system::{BillingService, QuotaCheck};

let billing = BillingService::new(pool, redis);
let check = billing.check_quota(&user_id, &model, estimated_tokens).await?;
if check.can_proceed {
    billing.record_usage(&user_id, &model, tokens, cost).await?;
}
```

### 4. cache (缓存服务)

**职责**: 高性能缓存服务

**主要功能**:
- Redis 缓存封装
- 内存缓存
- 缓存策略 (TTL, LRU)

## 数据流

```
┌─────────────┐
│   Nginx     │
│   :80/:443  │
└──────┬──────┘
       │
       ▼
┌─────────────────────────────────────┐
│      api-gateway-simple (:8080)     │
├─────────────────────────────────────┤
│  1. 请求验证 (auth-system)          │
│  2. 配额检查 (billing-system)       │
│  3. 路由到提供商 (provider)         │
│  4. 记录使用 (billing-system)       │
│  5. 返回响应                        │
└─────────────────────────────────────┘
       │
       ├──────────────┬──────────────┐
       ▼              ▼              ▼
┌─────────────┐ ┌──────────┐ ┌──────────────┐
│ PostgreSQL  │ │  Redis   │ │ 上游提供商    │
│ (用户数据)  │ │ (缓存)   │ │ (GLM/千问)   │
└─────────────┘ └──────────┘ └──────────────┘
```

## API 端点分类

### Anthropic 兼容端点
- `POST /v1/messages` - 消息 API
- `GET /v1/models` - 模型列表
- `GET /health` - 健康检查

### Claude Code 专用端点
- `GET /api/bootstrap` - 平台引导
- `GET /api/auth/session` - 会话信息
- `POST /oauth/token` - OAuth 令牌
- `POST /api/organizations/{id}/api_keys` - 创建 API 密钥

### 用户管理端点
- `POST /api/user/login` - 用户登录
- `POST /api/user/register` - 用户注册
- `GET /api/user/balance` - 余额查询

### 管理端点
- `GET /api/admin/stats` - 统计数据
- `GET /api/admin/keys` - API 密钥列表
- `POST /admin/keys/create` - 创建 API 密钥

## 环境变量

```bash
# 数据库
DATABASE_URL=postgresql://postgres:postgres@localhost/api_gateway

# Redis
REDIS_URL=redis://127.0.0.1:6379

# 服务端口
SERVER_ADDR=0.0.0.0:8080

# JWT 密钥
JWT_SECRET=your-secret-key-at-least-32-bytes-long

# 上游 API
UPSTREAM_API_URL=https://open.bigmodel.cn/api/anthropic
UPSTREAM_API_KEY=your-upstream-api-key

# Claude Code 伪装 (可选)
FAKE_ACCOUNT_UUID=3c90a1a6-8e4a-4d7a-9e5f-1a2b3c4d5e6f
FAKE_EMAIL=user@example.com
FAKE_NAME=Claude User
```

## 构建

```bash
# 开发版本
cd backend/api-gateway-simple
cargo build

# 发布版本
cargo build --release

# 运行
./target/release/api-gateway-simple
```

## 部署

```bash
# 停止旧服务
pkill -f api-gateway-simple

# 启动新服务
nohup ./target/release/api-gateway-simple > /var/log/api-gateway.log 2>&1 &

# 检查状态
curl http://localhost:8080/health
```

---

生成时间: 2026-02-24
