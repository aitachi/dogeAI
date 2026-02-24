# Rust 中转系统架构文档

> 生成时间: 2026-02-24
> 系统概述: 本机部署的 Rust 构建的 API 中转、网关、计费系统

---

## 目录

1. [系统概览](#系统概览)
2. [项目详细结构](#项目详细结构)
3. [架构关系图](#架构关系图)
4. [技术栈总览](#技术栈总览)
5. [配置文件清单](#配置文件清单)
6. [部署目录](#部署目录)

---

## 系统概览

本系统由 **5 个 Rust 项目** 组成，构成完整的 API 服务中转生态系统：

| 项目 | 路径 | 功能描述 |
|------|------|----------|
| **auth-system** | `/root/auth-system/` | JWT认证系统，支持多层缓存、权限管理 |
| **api-gateway-simple** | `/root/api-gateway-simple/` | 简化版API网关，多提供商故障转移 |
| **two-level-cache** | `/root/two-level-cache/` | L1+L2两级缓存系统 |
| **api-gateway** | `/root/api-gateway/` | 完整版API网关，限流+计费 |
| **billing-gateway** | `/root/aitachi/billing-system/` | API计费网关，密钥管理 |

### 系统能力

- **API代理与路由**: 统一入口，智能路由分发
- **认证与授权**: JWT Token验证、权限控制、Token撤销
- **限流控制**: 多维度限流策略
- **多层缓存**: L1本地缓存 + L2 Redis缓存
- **计费管理**: API调用计费、余额管理
- **密钥管理**: API Key生成、验证、撤销
- **故障转移**: 多提供商自动切换

---

## 项目详细结构

### 1. auth-system - 认证系统

**路径**: `/root/auth-system/`

**版本**: 2.0.0

**核心功能**: 高性能JWT认证系统

#### 目录结构

```
auth-system/
├── Cargo.toml                 # 项目配置
├── README.md                  # 项目文档
├── src/
│   ├── lib.rs                 # 主库：认证核心功能
│   ├── middleware.rs          # 认证中间件和提取器
│   ├── config.rs              # 配置管理
│   ├── service.rs             # 认证服务实现
│   ├── cache.rs               # 多层缓存管理
│   ├── models.rs              # 数据模型
│   └── error.rs               # 错误处理
├── examples/
│   └── server.rs              # 示例服务器
└── migrations/                # 数据库迁移文件
```

#### 关键文件说明

| 文件 | 功能 |
|------|------|
| `src/lib.rs` | JWT Token验证、多层缓存(L1 moka + L2 Redis)，验证延迟<0.5ms |
| `src/middleware.rs` | Axum认证中间件，开箱即用 |
| `src/service.rs` | 认证服务核心逻辑 |
| `src/cache.rs` | 双重缓存策略实现 |
| `src/config.rs` | 配置加载和管理 |
| `src/models.rs` | Token、User、Permission等数据模型 |
| `src/error.rs` | 统一错误类型定义 |

#### 主要依赖

```toml
jsonwebtoken = "9.2"      # JWT处理
axum = "0.7"              # Web框架
tower = "0.4"             # 中间件框架
sqlx = { version = "0.7", features = ["postgres", "chrono", "migrate"] }  # 数据库
redis = "0.24"            # Redis L2缓存
moka = { version = "0.12", features = ["future"] }  # L1本地缓存
```

---

### 2. api-gateway-simple - 简化版API网关

**路径**: `/root/api-gateway-simple/`

**版本**: 2.0.0

**核心功能**: 轻量级API网关，支持多提供商

#### 目录结构

```
api-gateway-simple/
├── Cargo.toml                    # 项目配置
├── README.md                     # 项目文档
├── Dockerfile                    # 容器化配置
├── src/
│   ├── main.rs                   # 主服务器
│   ├── main_claude_compatible.rs # Claude兼容版本
│   ├── jwt.rs                    # JWT认证模块
│   ├── provider.rs               # 多提供商管理
│   └── anthropic.rs              # Anthropic API处理
└── scripts/                      # 部署脚本
```

#### 关键文件说明

| 文件 | 功能 |
|------|------|
| `src/main.rs` | JWT验证、密钥轮换、Token撤销、本地缓存 |
| `src/jwt.rs` | JWT认证服务实现 |
| `src/provider.rs` | 多提供商管理(GLM、千问、DeepSeek)自动故障转移 |
| `src/anthropic.rs` | Anthropic API代理实现 |

#### 支持的提供商

- **GLM 智谱AI**: 智谱AI大语言模型
- **千问**: 阿里云千问系列
- **DeepSeek**: DeepSeek AI模型
- **Anthropic**: Claude系列模型

---

### 3. two-level-cache - 两级缓存系统

**路径**: `/root/two-level-cache/`

**版本**: 0.1.0

**核心功能**: 高性能两级缓存架构

#### 目录结构

```
two-level-cache/
├── Cargo.toml          # 项目配置
├── src/
│   ├── lib.rs          # 两级缓存核心实现
│   └── main.rs         # 缓存服务器主程序
├── .git/               # Git仓库
└── target/             # 编译输出
```

#### 关键文件说明

| 文件 | 功能 |
|------|------|
| `src/lib.rs` | 两级缓存架构实现 |
| `src/main.rs` | 独立缓存服务器程序 |

#### 缓存策略

| 层级 | 类型 | 容量 | TTL |
|------|------|------|-----|
| L1 | LRU本地内存缓存 | <200MB | 1-5分钟 |
| L2 | Redis缓存 | 256MB | 可配置 |

#### 性能目标

- L1命中率: ≥70%
- L2命中率: ≥90%
- 命中时延迟: <1ms

---

### 4. api-gateway - 完整版API网关

**路径**: `/root/api-gateway/`

**版本**: 2.0.0

**核心功能**: 功能完整的Token API网关

#### 目录结构

```
api-gateway/
├── Cargo.toml                 # 项目配置
├── config.toml                # 网关配置文件
├── src/
│   ├── main.rs                # 主服务器入口
│   ├── config/
│   │   └── mod.rs             # 配置管理
│   ├── models/
│   │   ├── mod.rs             # 模型聚合
│   │   └── error.rs           # 错误模型
│   ├── handlers/              # 请求处理器
│   │   ├── mod.rs
│   │   ├── token.rs           # Token处理
│   │   └── anthropic.rs       # Anthropic API代理
│   ├── services/              # 业务服务
│   │   ├── mod.rs
│   │   ├── database.rs        # 数据库服务
│   │   ├── redis.rs           # Redis缓存服务
│   │   └── upstream.rs        # 上游API客户端
│   └── middleware/            # 中间件
│       ├── mod.rs
│       ├── auth.rs            # 认证中间件
│       └── rate_limit.rs      # 限流中间件
└── migrations/                # 数据库迁移文件
```

#### 关键文件说明

| 文件 | 功能 |
|------|------|
| `src/main.rs` | 网关启动入口，初始化所有组件 |
| `src/handlers/token.rs` | Token生成、验证、刷新 |
| `src/handlers/anthropic.rs` | Anthropic API代理处理 |
| `src/services/upstream.rs` | 上游API客户端池 |
| `src/middleware/rate_limit.rs` | Governor限流中间件 |
| `src/middleware/auth.rs` | JWT认证中间件 |

#### 主要依赖

```toml
axum = { version = "0.7", features = ["multipart", "ws"] }  # Web框架
governor = "0.6"                # 限流
redis = "0.24"                  # Redis
moka = { version = "0.12", features = ["future"] }  # 本地缓存
sqlx = { version = "0.7", features = ["postgres", "chrono", "uuid", "json"] }
```

---

### 5. billing-gateway - 计费网关

**路径**: `/root/aitachi/billing-system/`

**版本**: 0.1.0

**核心功能**: API计费、密钥管理、充值系统

#### 目录结构

```
billing-system/
├── Cargo.toml                 # 项目配置
├── src/
│   ├── main.rs                # 主服务器入口
│   ├── lib.rs                 # 库文件
│   ├── config/
│   │   └── mod.rs             # 配置管理
│   ├── models/
│   │   └── mod.rs             # 数据模型
│   ├── database/
│   │   └── mod.rs             # 数据库服务
│   ├── cache/
│   │   └── mod.rs             # 缓存服务
│   ├── auth/
│   │   └── mod.rs             # 认证模块
│   ├── billing/
│   │   └── mod.rs             # 计费模块
│   ├── proxy/
│   │   └── mod.rs             # 代理服务
│   ├── key_manager/
│   │   └── mod.rs             # 密钥管理
│   ├── recharge/
│   │   └── mod.rs             # 充值模块
│   ├── ratelimit/
│   │   └── mod.rs             # 限流模块
│   ├── scheduler/
│   │   └── mod.rs             # 定时任务
│   ├── api/
│   │   └── mod.rs             # API接口
│   └── bin/
│       └── check_keys.rs      # 密钥检查工具
├── migrations/                # 数据库迁移
├── tests/                     # 测试文件
└── static/                    # 静态文件（管理面板）
```

#### 关键文件说明

| 文件 | 功能 |
|------|------|
| `src/main.rs` | 计费网关启动入口 |
| `src/billing/mod.rs` | API调用计费逻辑、扣费 |
| `src/proxy/mod.rs` | 请求代理转发 |
| `src/key_manager/mod.rs` | API Key生成、验证、撤销 |
| `src/scheduler/mod.rs` | 定时任务（账单、统计） |
| `src/ratelimit/mod.rs` | 用户级限流 |
| `src/recharge/mod.rs` | 充值接口 |
| `src/bin/check_keys.rs` | 密钥状态检查工具 |

#### 主要功能模块

- **认证**: 用户登录、Token管理
- **计费**: 按Token/调用计费、余额管理
- **密钥**: API Key CRUD、权限控制
- **代理**: 多上游代理、故障转移
- **限流**: 用户/密钥级别限流
- **充值**: 余额充值、消费记录
- **调度**: 定时统计、账单生成

---

## 架构关系图

```
┌─────────────────────────────────────────────────────────────────┐
│                         客户端请求                               │
└────────────────────────────┬────────────────────────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────────┐
│                      API 网关层                                  │
├─────────────────────────────────────────────────────────────────┤
│  ┌──────────────────┐  ┌──────────────────┐  ┌───────────────┐ │
│  │ api-gateway      │  │ api-gateway-simple│ │ billing-gateway│ │
│  │ (完整版)         │  │ (简化版)          │ │ (计费版)      │ │
│  │                  │  │                  │ │               │ │
│  │ • 限流           │  │ • 多提供商        │ │ • 计费        │ │
│  │ • 缓存           │  │ • 故障转移        │ │ • 密钥管理    │ │
│  │ • 认证           │  │ • JWT认证         │ │ • 充值        │ │
│  └──────────────────┘  └──────────────────┘  └───────────────┘ │
└────────────────────────────┬────────────────────────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────────┐
│                      服务层                                      │
├─────────────────────────────────────────────────────────────────┤
│  ┌──────────────────┐  ┌──────────────────┐                     │
│  │ auth-system      │  │ two-level-cache  │                     │
│  │ (认证系统)        │  │ (两级缓存)        │                     │
│  │                  │  │                  │                     │
│  │ • JWT验证        │  │ • L1: LRU本地    │                     │
│  │ • 权限管理       │  │ • L2: Redis      │                     │
│  │ • Token撤销      │  │ • 高性能命中     │                     │
│  └──────────────────┘  └──────────────────┘                     │
└────────────────────────────┬────────────────────────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────────┐
│                      数据层                                      │
├─────────────────────────────────────────────────────────────────┤
│  ┌──────────────────┐  ┌──────────────────┐  ┌───────────────┐ │
│  │ PostgreSQL       │  │ Redis            │  │ 上游API       │ │
│  │ • 用户数据       │  │ • L2缓存         │  │ • Anthropic   │ │
│  │ • 计费记录       │  │ • Token存储      │  │ • GLM         │ │
│  │ • 密钥存储       │  │ • 会话管理       │  │ • DeepSeek    │ │
│  └──────────────────┘  └──────────────────┘  └───────────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

---

## 技术栈总览

### 核心框架

| 框架 | 版本 | 用途 |
|------|------|------|
| **Axum** | 0.7 | Web服务器框架 |
| **Tower** | 0.4 | 中间件框架 |
| **Tokio** | Latest | 异步运行时 |

### 数据存储

| 组件 | 版本 | 用途 |
|------|------|------|
| **PostgreSQL (SQLx)** | 0.7 | 主数据库 |
| **Redis** | 0.24/0.27 | L2缓存、会话存储 |

### 认证与安全

| 库 | 版本 | 用途 |
|----|------|------|
| **jsonwebtoken** | 9.2 | JWT Token处理 |
| **governor** | 0.6 | 限流 |

### 缓存

| 库 | 版本 | 用途 |
|----|------|------|
| **moka** | 0.12 | L1本地缓存 |
| **lru** | 0.12 | LRU缓存算法 |

### 其他工具

| 库 | 版本 | 用途 |
|----|------|------|
| **sqlx** | 0.7 | 数据库ORM |
| **clap** | 4 | CLI参数解析 |
| **tokio-cron-scheduler** | 0.9 | 定时任务 |

---

## 配置文件清单

| 项目 | 配置文件 | 路径 |
|------|----------|------|
| api-gateway | config.toml | `/root/api-gateway/config.toml` |
| api-gateway-simple | 环境变量 | `/root/api-gateway-simple/.env` |
| billing-gateway | 环境变量 | `/root/aitachi/billing-system/.env` |
| auth-system | 环境变量 | `/root/auth-system/.env` |

### 环境变量配置项

```bash
# 数据库配置
DATABASE_URL=postgresql://user:pass@localhost/dbname

# Redis配置
REDIS_URL=redis://localhost:6379

# JWT配置
JWT_SECRET=your-secret-key
JWT_EXPIRATION=3600

# API配置
API_PORT=8080
API_HOST=0.0.0.0

# 上游API配置
ANTHROPIC_API_KEY=sk-xxx
GLM_API_KEY=xxx
DEEPSEEK_API_KEY=sk-xxx
```

---

## 部署目录

### 源代码目录

```
/root/
├── auth-system/              # 认证系统
├── api-gateway-simple/       # 简化网关
├── two-level-cache/          # 两级缓存
├── api-gateway/              # 完整网关
└── aitachi/
    └── billing-system/       # 计费网关
```

### 编译输出目录

```
/root/
├── auth-system/target/release/auth-system
├── api-gateway-simple/target/release/api-gateway-simple
├── two-level-cache/target/release/two-level-cache
├── api-gateway/target/release/api-gateway
└── aitachi/billing-system/target/release/billing-gateway
```

### 服务部署

所有项目均支持：
- **直接运行**: `cargo run --release`
- **Docker部署**: 提供Dockerfile
- **systemd服务**: 可配置为系统服务

---

## 系统启动顺序

1. **启动数据层**
   ```bash
   # PostgreSQL
   systemctl start postgresql
   # Redis
   systemctl start redis
   ```

2. **启动基础服务**
   ```bash
   # auth-system
   cd /root/auth-system && cargo run --release
   # two-level-cache
   cd /root/two-level-cache && cargo run --release
   ```

3. **启动网关服务**
   ```bash
   # api-gateway
   cd /root/api-gateway && cargo run --release
   # api-gateway-simple
   cd /root/api-gateway-simple && cargo run --release
   # billing-gateway
   cd /root/aitachi/billing-system && cargo run --release
   ```

---

## 文档版本

- **创建日期**: 2026-02-24
- **版本**: 1.0
- **状态**: 当前系统架构
