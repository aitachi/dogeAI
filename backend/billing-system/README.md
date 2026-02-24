# Billing System (计费网关系统)

完整的 API 计费和管理系统，支持用户认证、API Key 管理、实时计费、余额充值等功能。

## 项目结构

```
billing-system/
├── Cargo.toml           # 项目配置
├── README.md            # 项目说明
├── .env.example         # 环境变量示例
├── .gitignore           # Git忽略规则
├── src/                 # 源代码
│   ├── main.rs          # 服务入口
│   ├── api/             # API 路由
│   ├── auth/            # 认证模块
│   ├── billing/         # 计费模块
│   ├── cache/           # 缓存模块
│   ├── config/          # 配置管理
│   ├── database/        # 数据库操作
│   ├── key_manager/     # Key 管理
│   ├── models/          # 数据模型
│   ├── proxy/           # 代理转发
│   ├── ratelimit/       # 限流
│   ├── recharge/        # 充值模块
│   └── scheduler/       # 定时任务
├── migrations/          # 数据库迁移
├── static/              # 静态文件
├── tests/               # 测试
├── scripts/             # 部署脚本
├── examples/            # 示例代码
└── docs/                # 项目文档
```

## 功能特性

- **用户认证**: JWT Token 认证，用户注册/登录
- **API Key 管理**: 创建、查询、删除 API Key
- **实时计费**: 基于 Token 使用的计费系统
- **余额充值**: 充值码系统，支持批量生成
- **使用统计**: 详细的调用统计和历史记录
- **限流控制**: 用户级和 Key 级限流
- **静态文件服务**: 内置用户/管理后台界面

## 技术栈

- Axum 0.7 - Web 框架
- SQLx - 数据库 ORM
- Redis - 缓存和会话
- Tokio-Cron - 定时任务
- PostgreSQL - 主数据库

## 快速开始

### 1. 配置环境变量

```bash
cp .env.example .env
# 编辑 .env 文件，设置数据库连接等配置
```

### 2. 初始化数据库

```bash
psql -U postgres -c "CREATE DATABASE billing_db;"
psql -U postgres -d billing_db -f migrations/001_initial.sql
psql -U postgres -d billing_db -f migrations/002_add_reconciliation.sql
```

### 3. 构建和运行

```bash
cargo build --release
cargo run --release
```

服务将在 `http://localhost:8082` 启动。

## API 端点

### 认证

| 方法 | 路径 | 说明 |
|------|------|------|
| POST | /auth/register | 用户注册 |
| POST | /auth/login | 用户登录 |
| POST | /auth/logout | 用户登出 |

### API Key 管理

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | /api/keys | 获取 API Key 列表 |
| POST | /api/keys | 创建新 API Key |
| DELETE | /api/keys/:id | 删除 API Key |

### 计费

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | /billing/balance | 查询余额 |
| GET | /billing/history | 查询消费历史 |
| GET | /billing/stats | 查询统计信息 |

### 充值

| 方法 | 路径 | 说明 |
|------|------|------|
| POST | /recharge/redeem | 使用充值码充值 |
| POST | /recharge/generate | 生成充值码 (管理员) |

### 代理

| 方法 | 路径 | 说明 |
|------|------|------|
| POST | /v1/chat/completions | 聊天完成 (代理到上游) |
| GET | /v1/models | 模型列表 |

## 计费规则

| 模型 | 输入价格 | 输出价格 |
|------|---------|---------|
| Claude Opus | 15 积分/百万 tokens | 75 积分/百万 tokens |
| Claude Sonnet | 3 积分/百万 tokens | 15 积分/百万 tokens |
| Claude Haiku | 0.25 积分/百万 tokens | 1.25 积分/百万 tokens |

## 测试

```bash
# 运行所有测试
cargo test

# 运行集成测试
cargo test --test integration_tests

# 运行性能测试
cargo test --test performance_tests
```

## 部署

### Docker

```bash
docker build -t billing-system .
docker run -p 8082:8082 --env-file .env billing-system
```

### Systemd

```bash
cp billing-system.service /etc/systemd/system/
systemctl daemon-reload
systemctl enable billing-system
systemctl start billing-system
```

## 文档

更多详细文档请参考 [docs/](./docs/) 目录。
