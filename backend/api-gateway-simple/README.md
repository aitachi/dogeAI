# API网关 - Redis + PostgreSQL 生产级方案

## 架构

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   Client    │────▶│ API Gateway │────▶│   Redis     │
└─────────────┘     └─────────────┘     └─────────────┘
                          │
                          ▼
                   ┌─────────────┐
                   │ PostgreSQL  │
                   └─────────────┘
```

## 技术栈

| 组件    | 技术        |
|---------|-------------|
| 框架    | Axum 0.7    |
| 运行时  | Tokio       |
| 缓存    | Redis       |
| 数据库  | PostgreSQL  |
| 语言    | Rust        |

## 性能指标

- QPS: 10,000+
- 延迟: <10ms
- 并发: 支持150+用户

## 快速启动

### 使用 Docker Compose (推荐)

```bash
cd /root/api-gateway-simple
docker-compose up -d
```

### 本地开发

1. 安装 PostgreSQL 和 Redis
2. 设置环境变量或使用 .env 文件
3. 运行:

```bash
cargo build --release
./target/release/api-gateway-simple
```

## 环境变量

| 变量         | 默认值                                          |
|--------------|------------------------------------------------|
| DATABASE_URL | postgresql://postgres:postgres@localhost/api_gateway |
| REDIS_URL    | redis://127.0.0.1:6379                          |
| SERVER_ADDR  | 0.0.0.0:8081                                    |

## API端点

| 方法 | 路径                      | 说明         |
|------|---------------------------|-------------|
| GET  | /health                   | 健康检查     |
| GET  | /v1/models                | 模型列表     |
| POST | /v1/token/query           | 查询费用     |
| POST | /v1/chat/completions      | 聊天完成     |

## 测试

```bash
# 健康检查
curl http://localhost:8081/health

# 聊天请求
curl -X POST http://localhost:8081/v1/chat/completions \
  -H "Authorization: Bearer sk_test_1234567890abcdef" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "sonnet",
    "messages": [{"role": "user", "content": "Hello"}],
    "max_tokens": 100
  }'
```

## 测试Token

```
sk_test_1234567890abcdef
```

初始余额: 1亿积分 (pro等级)
