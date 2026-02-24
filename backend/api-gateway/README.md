# Token API Gateway

基于 Rust + Axum + Tower 构建的高性能 API 网关系统。

## 功能特性

### 核心功能
- ✅ **请求路由**: 智能分发请求到对应的处理器
- ✅ **Token验证**: 基于 Token 的用户认证和授权
- ✅ **限流控制**: 三层限流架构（用户级、IP级、全局）
- ✅ **计费检查**: 实时余额检查和费用计算
- ✅ **请求转发**: 代理请求到上游服务（Claude API）
- ✅ **流式响应**: 支持 Server-Sent Events (SSE)
- ✅ **错误处理**: 统一的错误响应格式
- ✅ **响应增强**: 自动添加追踪ID、性能指标等响应头

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

### 1. 安装依赖

```bash
# 克隆项目
cd /root/api-gateway

# 安装Rust依赖
cargo build --release
```

### 2. 配置数据库

```bash
# 创建数据库
createdb api_gateway

# 运行迁移
psql -d api_gateway -f migrations/001_initial.sql
```

### 3. 配置Redis

```bash
# 启动Redis
redis-server

# 测试连接
redis-cli ping
```

### 4. 配置应用

编辑 `config.toml` 文件：

```toml
[server]
port = 8080

[database]
url = "postgresql://postgres:password@localhost/api_gateway"

[cache]
redis_url = "redis://127.0.0.1:6379"
```

### 5. 启动服务

```bash
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

## 项目结构

```
api-gateway/
├── src/
│   ├── main.rs              # 应用入口
│   ├── models/              # 数据模型
│   │   ├── mod.rs
│   │   └── error.rs
│   ├── middleware/          # 中间件
│   │   ├── mod.rs
│   │   ├── auth.rs
│   │   └── rate_limit.rs
│   ├── services/            # 服务层
│   │   ├── mod.rs
│   │   ├── redis.rs
│   │   └── database.rs
│   ├── handlers/            # HTTP处理器
│   │   ├── mod.rs
│   │   └── token.rs
│   └── config/              # 配置
│       └── mod.rs
├── migrations/              # 数据库迁移
│   └── 001_initial.sql
├── config.toml              # 配置文件
├── Cargo.toml               # Rust依赖
└── README.md
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

## 测试

### 健康检查
```bash
curl http://localhost:8080/health
```

### Token查询
```bash
curl -X POST http://localhost:8080/v1/token/query \
  -H "Authorization: Bearer sk_test_xxx" \
  -H "Content-Type: application/json" \
  -d '{"model":"opus","input_tokens":100,"output_tokens":50}'
```

### 聊天（非流式）
```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Authorization: Bearer sk_test_xxx" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "opus",
    "messages": [{"role": "user", "content": "Hello"}],
    "stream": false
  }'
```

### 聊天（流式）
```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Authorization: Bearer sk_test_xxx" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "opus",
    "messages": [{"role": "user", "content": "Hello"}],
    "stream": true
  }'
```

## 性能优化

### 缓存策略
- moka本地缓存: Token信息（1小时TTL）
- Redis缓存: 用户会话（1天TTL）

### 连接池配置
- PostgreSQL: 30个最大连接
- Redis: 50个最大连接
- 上游服务: 100个最大连接

### 异步处理
- 计费记录: 异步写入
- 日志记录: 后台任务

## 部署

### Docker部署（可选）

```dockerfile
FROM rust:1.70 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates
COPY --from=builder /app/target/release/api-gateway /usr/local/bin/
EXPOSE 8080
CMD ["api-gateway"]
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

## 故障排查

### 常见问题

1. **数据库连接失败**
   - 检查PostgreSQL是否运行
   - 验证连接字符串

2. **Redis连接失败**
   - 检查Redis是否运行
   - 检查防火墙设置

3. **Token验证失败**
   - 确认Token格式: `Bearer sk_xxx`
   - 检查Token是否在数据库中

## License

MIT

## 版本

v2.0.0

## 作者

API Gateway Team
