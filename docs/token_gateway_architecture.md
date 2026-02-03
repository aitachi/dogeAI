# Token中转平台 - 高可用架构设计

## 📋 系统架构

```
┌─────────────────────────────────────────────────────────────────┐
│                         用户请求层                               │
│  用户Token A, Token B, Token C... (对外提供的统一Token)          │
└────────────────────────┬────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────────┐
│                    API网关层 (FastAPI)                           │
│  • 请求验证                                                      │
│  • 限流控制                                                      │
│  • 会话管理                                                      │
└────────────────────────┬────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────────┐
│                   负载均衡层                                     │
│  • Key池管理                                                     │
│  • 健康检查                                                      │
│  • 智能路由                                                      │
│  • 故障切换                                                      │
└─────────────┬─────────────┬─────────────┬────────────┬─────────┘
              │             │             │            │
              ▼             ▼             ▼            ▼
         ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐
         │ API Key │  │ API Key │  │ API Key │  │ API Key │
         │   Pool  │  │   Pool  │  │   Pool  │  │   Pool  │
         │    #1   │  │    #2   │  │    #3   │  │    #4   │
         └─────────┘  └─────────┘  └─────────┘  └─────────┘
              │             │             │            │
              └─────────────┴─────────────┴────────────┘
                            │
                            ▼
                   ┌──────────────────┐
                   │  Anthropic API   │
                   │  (或其它LLM)     │
                   └──────────────────┘
```

## 🎯 核心功能模块

### 1. Key池管理 (Key Pool Manager)
- 存储多个实际的API Keys
- 每个Key包含状态、权重、健康度
- 支持动态添加/删除Keys

### 2. 健康检查系统 (Health Check System)
- 定期检测每个Key的响应时间
- 检测Key是否失效/限流
- 自动标记不健康的Key

### 3. 负载均衡器 (Load Balancer)
- **策略1: 轮询 (Round Robin)** - 平均分配
- **策略2: 加权轮询** - 根据性能分配
- **策略3: 最少连接** - 选择当前请求最少的Key
- **策略4: 响应时间优先** - 优先使用响应快的Key

### 4. 会话管理 (Session Management)
- 同一会话/对话保持使用相同的Key
- 确保上下文连贯性
- 会话超时机制

### 5. 故障切换 (Failover)
- 检测到Key卡顿/失败自动切换
- 降级策略：优先级Key池
- 熔断机制：连续失败暂时禁用

### 6. 监控统计 (Monitoring)
- 每个Key的使用统计
- 响应时间监控
- 错误率统计
- 实时告警

## 🗄️ 数据库设计

### 表1: backend_keys (后端实际API Keys)
```sql
CREATE TABLE backend_keys (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    key_name VARCHAR(100) UNIQUE,           -- Key名称/标识
    api_key VARCHAR(255) NOT NULL,          -- 实际的API Key
    provider VARCHAR(50),                   -- 提供商 (anthropic, openai等)
    is_active BOOLEAN DEFAULT 1,            -- 是否启用
    weight INTEGER DEFAULT 100,             -- 权重 (用于负载均衡)
    priority INTEGER DEFAULT 0,             -- 优先级 (数字越大优先级越高)
    health_score INTEGER DEFAULT 100,       -- 健康分数 (0-100)
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
```

### 表2: key_health_stats (Key健康统计)
```sql
CREATE TABLE key_health_stats (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    backend_key_id INTEGER,
    response_time FLOAT,                    -- 响应时间(ms)
    is_success BOOLEAN,                     -- 是否成功
    error_type VARCHAR(50),                 -- 错误类型
    checked_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (backend_key_id) REFERENCES backend_keys(id)
);
```

### 表3: sessions (会话管理)
```sql
CREATE TABLE sessions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id VARCHAR(100) UNIQUE NOT NULL,-- 会话ID
    user_token VARCHAR(255) NOT NULL,       -- 用户Token
    backend_key_id INTEGER,                 -- 绑定的后端Key
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    last_used_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    expires_at TIMESTAMP,                   -- 过期时间
    FOREIGN KEY (backend_key_id) REFERENCES backend_keys(id)
);
```

### 表4: request_logs (请求日志)
```sql
CREATE TABLE request_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id VARCHAR(100),
    backend_key_id INTEGER,
    user_token VARCHAR(255),
    model VARCHAR(100),
    input_tokens INTEGER DEFAULT 0,
    output_tokens INTEGER DEFAULT 0,
    response_time FLOAT,
    status VARCHAR(20),                     -- success/failed
    error_message TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (backend_key_id) REFERENCES backend_keys(id)
);
```

## 🔄 负载均衡策略详解

### 策略1: 会话粘性 (Session Sticky) - **推荐**
```
逻辑：
1. 检查是否有该会话绑定的Key
2. 如果有且Key健康 → 使用该Key
3. 如果没有或Key不健康 → 选择新Key并绑定

优点：
✅ 确保对话连贯性
✅ 减少Key切换
✅ 提升用户体验
```

### 策略2: 智能加权
```
算法：
score = health_score * 0.6 + (100 / avg_response_time) * 0.4

选择score最高的Key

优点：
✅ 综合考虑健康度和响应时间
✅ 自动避障
✅ 性能最优
```

### 策略3: 分层降级
```
第一层：高优先级Key池 (高性能)
第二层：普通Key池 (备用)
第三层：低优先级Key池 (应急)

自动切换逻辑：
1. 优先使用第一层
2. 第一层全部不可用时 → 降级到第二层
3. 第二层不可用 → 使用第三层
4. 记录降级事件并告警
```

## ⚡ 故障切换机制

### 1. 实时检测
```
触发条件：
• 响应时间 > 10秒
• 连接超时 > 5秒
• 返回429/500/502/503错误
• 连续3次请求失败

动作：
• 立即标记Key为"不健康"
• 降低health_score
• 将现有请求重定向到其他Key
```

### 2. 定期健康检查
```
频率：每30秒一次
方法：发送测试请求
更新：health_score

恢复逻辑：
• 连续5次检查成功 → 恢复为健康
• 逐步恢复health_score
```

### 3. 熔断机制
```
状态转换：
健康 → 检测中 → 不健康 → 熔断 → 恢复中 → 健康

熔断条件：
• 1分钟内失败率 > 50%
• 连续失败 > 10次

恢复条件：
• 5分钟内无失败
• 连续成功 > 20次
```

## 📊 监控指标

### 实时监控
- 每个Key的QPS (每秒请求数)
- 响应时间 (P50, P95, P99)
- 错误率
- 并发连接数
- 健康度分数

### 告警规则
- Key不可用 > 1分钟
- 错误率 > 10%
- 响应时间 > 5秒
- 所有Key不可用

## 🔧 配置示例

```yaml
# config.yaml
key_pool:
  - name: "anthropic-main-1"
    api_key: "sk-ant-xxx1"
    provider: "anthropic"
    priority: 10
    weight: 100

  - name: "anthropic-backup-1"
    api_key: "sk-ant-xxx2"
    provider: "anthropic"
    priority: 5
    weight: 50

load_balancer:
  strategy: "session_sticky"  # session_sticky/weighted/least_connections
  health_check_interval: 30   # 秒
  max_response_time: 10000    # 毫秒
  circuit_breaker_threshold: 50  # 失败率百分比

session:
  timeout: 3600  # 会话超时时间(秒)
  cleanup_interval: 300  # 清理过期会话间隔(秒)

limits:
  max_concurrent_per_key: 10  # 每个Key最大并发
  max_retries: 3  # 最大重试次数
  retry_delay: 1  # 重试延迟(秒)
```

## 🚀 部署建议

### 1. 基础版 (单机)
```
FastAPI + SQLite + Redis
适用：< 1000 并发
```

### 2. 进阶版 (单机高可用)
```
FastAPI + MySQL + Redis + 健康检查
适用：< 5000 并发
```

### 3. 企业版 (分布式)
```
Nginx负载均衡 + 多个FastAPI实例 + MySQL集群 + Redis集群
适用：> 5000 并发
```

## 📈 性能优化建议

1. **连接池复用** - 复用HTTP连接
2. **异步处理** - 使用asyncio
3. **缓存策略** - Redis缓存健康状态
4. **批量请求** - 合并小请求
5. **限流降级** - 过载时自动降级
