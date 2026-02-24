# DogeAI - AI API Gateway Platform

DogeAI 是一个基于 Rust 构建的高性能 AI API 网关平台，提供完整的认证、计费、缓存和限流功能。

## 项目架构

```
dogeAI/
├── backend/                 # 后端服务
│   ├── api-gateway/        # 完整版 API 网关
│   ├── api-gateway-simple/ # 简化版 API 网关
│   ├── auth-system/        # JWT 认证系统
│   ├── billing-system/     # 计费网关系统
│   └── cache/              # 两级缓存实现
├── frontend/               # Vue 3 前端应用
├── docs/                   # 项目文档
└── scripts/                # 部署脚本
```

## 核心组件

### 1. API Gateway (完整版)

高性能 API 网关，支持：
- 请求路由与代理
- Token 认证与授权
- 三层限流（用户级、IP级、全局）
- 实时计费检查
- 流式响应支持 (SSE)

**技术栈**: Axum 0.7, Tokio, Redis, PostgreSQL, Moka

**端口**: 8080

**文档**: [backend/api-gateway/README.md](backend/api-gateway/README.md)

### 2. API Gateway Simple (简化版)

轻量级 API 网关，适用于快速部署：
- 核心 API 代理功能
- JWT 认证
- Redis + PostgreSQL 存储
- Claude API 兼容

**技术栈**: Axum 0.7, Tokio, Redis, PostgreSQL

**端口**: 8081

**文档**: [backend/api-gateway-simple/README.md](backend/api-gateway-simple/README.md)

### 3. Auth System

完整的 JWT 认证系统：
- HS256 签名算法
- 双层缓存 (Moka + Redis)
- Token 撤销机制
- 权限管理

**技术栈**: JWT, Redis, PostgreSQL, Moka, Axum

**使用方式**: 作为 Rust 库集成

**文档**: [backend/auth-system/README.md](backend/auth-system/README.md)

### 4. Billing System

计费网关系统：
- API Key 管理
- 实时计费
- 余额充值
- 使用统计
- 限流控制

**技术栈**: Axum 0.7, SQLx, Redis, Tokio-Cron

**端口**: 3000

**文档**: [backend/billing-system/README.md](backend/billing-system/README.md)

### 5. Two-Level Cache

高性能两级缓存实现：
- L1: 本地 LRU 缓存
- L2: Redis 分布式缓存
- 缓存一致性保证

**技术栈**: LRU, Redis, Tokio

**使用方式**: 作为 Rust 库集成

**文档**: [backend/cache/README.md](backend/cache/README.md)

### 6. Frontend

Vue 3 前端应用：
- 用户仪表板
- 管理后台
- API Key 管理
- 充值功能

**技术栈**: Vue 3, TypeScript, Vite, Pinia, Vue Router

**文档**: [frontend/README.md](frontend/README.md)

## 快速开始

### 环境要求

- Rust 1.70+
- Node.js 18+
- PostgreSQL 13+
- Redis 6+

### 使用 Docker Compose (推荐)

```bash
# 启动所有服务
docker-compose up -d

# 查看日志
docker-compose logs -f
```

### 手动部署

#### 1. 启动数据库服务

```bash
# PostgreSQL
docker run -d -p 5432:5432 \
  -e POSTGRES_PASSWORD=postgres \
  postgres:14

# Redis
docker run -d -p 6379:6379 redis:alpine
```

#### 2. 启动后端服务

```bash
# API Gateway (完整版)
cd backend/api-gateway
cargo build --release
cargo run --release

# API Gateway Simple
cd backend/api-gateway-simple
cargo build --release
cargo run --release

# Billing System
cd backend/billing-system
cargo build --release
cargo run --release
```

#### 3. 启动前端

```bash
cd frontend
npm install
npm run dev
```

## API 端点

### API Gateway

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | /health | 健康检查 |
| GET | /v1/models | 模型列表 |
| POST | /v1/chat/completions | 聊天完成 |
| POST | /v1/token/query | 查询费用 |

### Billing System

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | /health | 健康检查 |
| POST | /auth/register | 用户注册 |
| POST | /auth/login | 用户登录 |
| GET | /api/keys | API Key 列表 |
| POST | /api/keys | 创建 API Key |
| POST /recharge | 余额充值 |

## 计费规则

| 模型 | 输入价格 | 输出价格 |
|------|---------|---------|
| Claude Opus | 15 积分/百万 tokens | 75 积分/百万 tokens |
| Claude Sonnet | 3 积分/百万 tokens | 15 积分/百万 tokens |
| Claude Haiku | 0.25 积分/百万 tokens | 1.25 积分/百万 tokens |

## 性能指标

| 指标 | 目标值 |
|------|--------|
| 请求处理时间 | < 10ms |
| Token 验证 | < 1ms |
| 并发连接数 | 10000+ |
| QPS | 300-500 |
| 缓存命中率 | > 99% |

## 配置说明

每个服务都有对应的配置文件：

- `backend/api-gateway/config.toml` - API 网关配置
- `backend/api-gateway-simple/.env` - 简化网关环境变量
- `backend/billing-system/.env` - 计费系统环境变量

## 开发指南

### 添加新的后端服务

1. 在 `backend/` 目录下创建新项目
2. 更新主 README.md 添加服务说明
3. 添加对应的 Docker Compose 配置

### 前端开发

```bash
cd frontend
npm run dev          # 开发模式
npm run build        # 生产构建
npm run build:check  # 类型检查 + 构建
```

### 后端开发

```bash
cd backend/<service>
cargo build          # 构建
cargo test           # 测试
cargo run            # 运行
```

## 部署

### Docker 部署

每个服务都包含 Dockerfile：

```bash
cd backend/<service>
docker build -t dogeai/<service>:latest .
docker run -p <port>:<port> dogeai/<service>:latest
```

### Systemd 服务

参考各服务 README.md 中的 Systemd 配置示例。

## 监控和日志

- 日志格式: JSON (生产环境)
- 关键指标: 请求延迟、QPS、错误率、缓存命中率

## 测试

### 后端测试

```bash
cd backend/<service>
cargo test
```

### API 测试

```bash
# 健康检查
curl http://localhost:8080/health

# 聊天请求
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Authorization: Bearer sk_test_xxx" \
  -H "Content-Type: application/json" \
  -d '{"model":"opus","messages":[{"role":"user","content":"Hello"}]}'
```

## 故障排查

### 常见问题

1. **数据库连接失败**
   - 检查 PostgreSQL 是否运行
   - 验证连接字符串

2. **Redis 连接失败**
   - 检查 Redis 是否运行
   - 检查防火墙设置

3. **Token 验证失败**
   - 确认 Token 格式: `Bearer sk_xxx`
   - 检查 Token 是否在数据库中

## 版本

当前版本: v2.0.0

## 许可证

MIT License

## 贡献

欢迎提交 Issue 和 Pull Request。
