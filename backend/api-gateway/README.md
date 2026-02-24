# Token API Gateway

基于 Rust + Axum + Tower 构建的高性能 API 网关系统。

## 项目结构

```
api-gateway/
├── Cargo.toml           # 项目配置
├── README.md            # 项目说明
├── .env.example         # 环境变量示例
├── .gitignore           # Git忽略规则
├── Dockerfile           # 容器化
├── config.toml          # 配置文件
├── src/                 # 源代码
│   ├── main.rs          # 应用入口
│   ├── models/          # 数据模型
│   │   ├── mod.rs
│   │   └── error.rs
│   ├── middleware/      # 中间件
│   │   ├── mod.rs
│   │   ├── auth.rs
│   │   └── rate_limit.rs
│   ├── services/        # 服务层
│   │   ├── mod.rs
│   │   ├── redis.rs
│   │   └── database.rs
│   ├── handlers/        # HTTP处理器
│   │   ├── mod.rs
│   │   └── token.rs
│   └── config/          # 配置
│       └── mod.rs
├── migrations/          # 数据库迁移
├── tests/               # 集成测试
├── examples/            # 示例代码
├── scripts/             # 部署脚本
└── docs/                # 项目文档
```

## 功能特性

### 核心功能
- **请求路由**: 智能分发请求到对应的处理器
- **Token验证**: 基于 Token 的用户认证和授权
- **限流控制**: 三层限流架构（用户级、IP级、全局）
- **计费检查**: 实时余额检查和费用计算
- **请求转发**: 代理请求到上游服务（Claude API）
- **流式响应**: 支持 Server-Sent Events (SSE)
- **错误处理**: 统一的错误响应格式
- **响应增强**: 自动添加追踪ID、性能指标等响应头

### 性能指标
| 指标 | 目标值 | 实现方式 |
|------|--------|----------|
| 请求处理时间 | <10ms | 同步处理+缓存查询 |
| Token验证 | <1ms | moka本地缓存 |
| 并发连接数 | 10000+ | Tokio轻量任务 |
| QPS | 300-500 | 异步IO+连接池 |

## 快速开始

### 前置要求
- Rust 1.70+
- PostgreSQL 13+
- Redis 6+

### 1. 配置环境变量

```bash
cp .env.example .env
# 编辑 .env 文件设置配置
```

### 2. 配置数据库

```bash
# 创建数据库
createdb api_gateway

# 运行迁移
psql -d api_gateway -f migrations/001_initial.sql
```

### 3. 构建和运行

```bash
cargo build --release
cargo run --release
```

服务将在 `http://localhost:8080` 启动。

## API端点

### 健康检查
```
GET /health
```

### Token查询
```
POST /v1/token/query
Authorization: Bearer {your_token}
Content-Type: application/json

{
  "model": "opus",
  "input_tokens": 100,
  "output_tokens": 50
}
```

### 聊天接口（非流式）
```
POST /v1/chat/completions
Authorization: Bearer {your_token}
Content-Type: application/json

{
  "model": "opus",
  "messages": [
    {"role": "user", "content": "Hello"}
  ],
  "stream": false
}
```

### 聊天接口（流式）
```
POST /v1/chat/completions
Authorization: Bearer {your_token}
Content-Type: application/json

{
  "model": "opus",
  "messages": [...],
  "stream": true
}
```

### 模型列表
```
GET /v1/models
Authorization: Bearer {your_token}
```

## 限流策略

### 用户级限流
- Base套餐: 10 QPS
- Pro套餐: 100 QPS
- Max套餐: 200 QPS

### IP级限流
- 每个IP: 500 QPS

### 全局限流
- 整个系统: 5000 QPS

## 计费规则

| 模型 | 输入价格 | 输出价格 |
|------|---------|---------|
| Opus | 15积分/百万tokens | 75积分/百万tokens |
| Sonnet | 3积分/百万tokens | 15积分/百万tokens |
| Haiku | 0.25积分/百万tokens | 1.25积分/百万tokens |

## 部署

### Docker部署

```bash
docker build -t api-gateway .
docker run -p 8080:8080 --env-file .env api-gateway
```

### Systemd服务

创建 `/etc/systemd/system/api-gateway.service`:

```ini
[Unit]
Description=API Gateway
After=network.target postgresql.service redis.service

[Service]
Type=simple
User=root
WorkingDirectory=/root/api-gateway
ExecStart=/root/api-gateway/target/release/api-gateway
Restart=always

[Install]
WantedBy=multi-user.target
```

启动服务：
```bash
systemctl daemon-reload
systemctl enable api-gateway
systemctl start api-gateway
```

## 监控和日志

### 日志格式
- 开发环境: 文本格式
- 生产环境: JSON格式

### 关键指标
- 请求延迟 (P50, P99, P999)
- QPS
- 错误率
- 缓存命中率
- 限流触发次数

## License

MIT

## 版本

v2.0.0
