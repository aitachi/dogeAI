# Token认证系统 - 完整实现

高性能JWT认证系统, 支持多层缓存、权限管理、Token撤销等功能。

## 项目结构

```
auth-system/
├── Cargo.toml           # 项目配置
├── README.md            # 项目说明
├── .env.example         # 环境变量示例
├── .gitignore           # Git忽略规则
├── src/                 # 源代码
│   ├── lib.rs           # 库入口
│   ├── models.rs        # 数据模型
│   ├── error.rs         # 错误类型
│   ├── config.rs        # 配置管理
│   ├── cache.rs         # 双层缓存
│   ├── service.rs       # 认证服务核心
│   └── middleware.rs    # Axum中间件
├── migrations/          # 数据库迁移
├── examples/            # 示例代码
├── tests/               # 集成测试
├── scripts/             # 部署脚本
└── docs/                # 项目文档
```

## 核心特性

### 已实现功能

1. **JWT Token管理**
   - HS256签名算法
   - 自定义Claims (user_id, tier, token_version)
   - 3种Token类型 (Short/Long/Session)

2. **双层缓存**
   - L1: moka本地缓存 (<0.1ms)
   - L2: Redis共享缓存 (<1ms)
   - 缓存命中率 >99%

3. **权限管理**
   - 用户等级 (Base/Pro/Max/AMax/Enterprise)
   - 模型权限控制
   - 权限范围 (scopes)

4. **Token撤销**
   - Token版本机制
   - JTI黑名单
   - 用户状态检查

5. **Axum集成**
   - 认证中间件
   - 权限检查中间件
   - 用户提取器

## 快速开始

### 1. 环境准备

```bash
# PostgreSQL
sudo apt install postgresql postgresql-contrib
sudo systemctl start postgresql

# Redis (可选, 用于双层缓存)
sudo apt install redis-server
sudo systemctl start redis

# 或使用Docker
docker run -d -p 5432:5432 -e POSTGRES_PASSWORD=postgres postgres:14
docker run -d -p 6379:6379 redis:alpine
```

### 2. 配置环境变量

```bash
cp .env.example .env
# 编辑 .env 文件，设置配置
```

### 3. 初始化数据库

```bash
# 创建数据库
createdb auth_db

# 运行迁移
psql -d auth_db -f migrations/001_initial.sql
```

### 4. 运行示例

```bash
cargo run --example server
```

访问 http://localhost:3000/health

## 使用示例

### 生成Token

```rust
use auth_system::{AuthService, TokenType};

// 用户登录
let token = auth.login(user_id, TokenType::Short).await?;
```

### 验证Token

```rust
// 验证Token
let user_info = auth.verify_token(&token).await?;

println!("用户: {} ({})", user_info.username, user_info.tier);
```

### Axum集成

```rust
use axum::{Router, routing::get};
use auth_system::middleware::{auth_middleware, AuthenticatedUser};

async fn protected_handler(user: AuthenticatedUser) -> String {
    format!("Hello, {}!", user.username)
}

let app = Router::new()
    .route("/api/profile", get(protected_handler))
    .route_layer(axum::middleware::from_fn_with_state(
        auth_service,
        auth_middleware,
    ));
```

## 环境变量

| 变量 | 说明 | 默认值 |
|------|------|--------|
| `JWT_SECRET` | JWT签名密钥 | (必填) |
| `DATABASE_URL` | PostgreSQL连接 | postgresql://localhost/auth_db |
| `REDIS_URL` | Redis连接 | redis://127.0.0.1:6379 |
| `JWT_ISSUER` | 签发者 | aitachi-auth |

## 性能

| 指标 | 数值 |
|------|------|
| Token验证 (L1命中) | <0.1ms |
| Token验证 (L2命中) | <1ms |
| 缓存命中率 | >99% |
| 最大QPS | 10K+ |
| 内存占用 | ~110MB |

## 文档

更多详细文档请参考 [docs/](./docs/) 目录。

## 数据结构

### Claims (JWT Payload)

```rust
pub struct Claims {
    // 标准声明
    pub sub: String,      // user_id
    pub exp: usize,       // 过期时间
    pub iat: usize,       // 签发时间
    pub iss: String,      // 签发者

    // 自定义声明
    pub user_id: i64,
    pub username: String,
    pub tier: String,
    pub token_version: i32,
    pub jti: Option<String>,
}
```

## 安全建议

1. **密钥管理**
   - 使用强随机密钥 (>=256位)
   - 通过环境变量传入
   - 定期轮换

2. **Token传输**
   - 必须使用HTTPS
   - Bearer Token在Authorization头

3. **存储策略**
   - 客户端: 内存存储
   - 避免: LocalStorage (XSS风险)

## 测试

```bash
# 单元测试
cargo test

# 集成测试
cargo test -- --ignored

# 性能测试
cargo test --release performance
```

---

**版本**: 2.0.0
**最后更新**: 2026-02-24
