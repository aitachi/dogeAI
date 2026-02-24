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

## 项目结构

```
api-gateway-simple/
├── Cargo.toml           # 项目配置
├── README.md            # 项目说明
├── .env.example         # 环境变量示例
├── .gitignore           # Git忽略规则
├── Dockerfile           # 容器化
├── docker-compose.yml   # Docker编排
├── config/              # 配置文件目录
├── src/                 # 源代码
│   ├── main.rs          # 入口
│   ├── config/          # 配置模块
│   ├── models/          # 数据模型
│   ├── handlers/        # 请求处理器
│   ├── services/        # 业务服务
│   ├── middleware/      # 中间件
│   ├── jwt.rs           # JWT认证
│   ├── provider.rs      # 提供商管理
│   └── anthropic.rs     # Anthropic API兼容
├── migrations/          # 数据库迁移
├── scripts/             # 部署脚本
└── docs/                # 项目文档
```

## 快速启动

### 使用 Docker Compose (推荐)

```bash
docker-compose up -d
```

### 本地开发

1. 复制环境变量配置
```bash
cp .env.example .env
```

2. 安装 PostgreSQL 和 Redis

3. 运行
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
| JWT_SECRET   | (必须设置)                                      |
| UPSTREAM_API_URL | https://api.anthropic.com                   |
| UPSTREAM_API_KEY | (必须设置)                                   |

## API端点

| 方法 | 路径                      | 说明         |
|------|---------------------------|-------------|
| GET  | /health                   | 健康检查     |
| GET  | /v1/models                | 模型列表     |
| POST | /v1/token/query           | 查询费用     |
| POST | /v1/chat/completions      | 聊天完成     |
| POST | /v1/messages              | Anthropic消息API |

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

## 文档

更多详细文档请参考 [docs/](./docs/) 目录。

## 测试Token

```
sk_test_1234567890abcdef
```

初始余额: 1亿积分 (pro等级)
