# 认证系统版本对比

## 完整版 vs 简化版

### 架构对比

| 特性 | 完整版 | 简化版 |
|------|----------------|-------------|
| **缓存策略** | L1(moka) + L2(Redis) | L1(moka) |
| **数据库** | PostgreSQL | SQLite |
| **Token撤销** | Token版本 + JTI黑名单 | Token版本 (内存) |
| **审计日志** | ✅ 完整审计日志 | ❌ 无 |
| **会话管理** | ✅ 会话表 | ❌ 无 |
| **API密钥** | ✅ 支持 | ❌ 无 |
| **权限范围** | 细粒度 (scopes数组) | 简化 (tier映射) |
| **Token类型** | 3种 (Short/Long/Session) | 2种 (Short/Session) |
| **依赖数** | ~20个 | ~10个 |
| **编译时间** | ~30秒 | ~10秒 |
| **内存占用** | ~100MB | ~30MB |
| **部署复杂度** | 高 (需PostgreSQL + Redis) | 低 (仅需SQLite) |

### 性能对比 (单机)

| 场景 | 完整版 | 简化版 |
|------|----------------|-------------|
| **Token验证 (缓存命中)** | <0.1ms | <0.1ms |
| **Token验证 (缓存未命中)** | 1-2ms (Redis) | 5-10ms (SQLite) |
| **缓存命中率** | >99% (双层) | >95% (单层) |
| **最大QPS** | 10K+ | 1K+ |
| **并发支持** | 高 | 中 |

### 适用场景

#### 完整版适用场景

- ✅ 生产环境部署
- ✅ 高并发应用 (>1000 QPS)
- ✅ 多实例部署 (需要共享缓存)
- ✅ 需要详细审计日志
- ✅ 需要API密钥管理
- ✅ 需要细粒度权限控制

#### 简化版适用场景

- ✅ 本地开发环境
- ✅ 小型应用 (<500 用户)
- ✅ 单实例部署
- ✅ 快速原型开发
- ✅ 学习和测试
- ✅ 资源受限环境

### 代码对比

#### 完整版

```rust
// 需要配置Redis + PostgreSQL
let db = PgPool::connect(&DATABASE_URL).await?;
let redis = RedisConfig::new(REDIS_URL);
let cache = CacheManager::new(redis, 1000, 3600);
let auth = AuthService::new(db, cache, config);
```

#### 简化版

```rust
// 仅需SQLite
let db = SqlitePool::connect("sqlite:auth.db").await?;
let auth = AuthService::new(db, "secret");
```

### 迁移指南

从简化版迁移到完整版:

1. **数据库迁移**
   ```bash
   # SQLite -> PostgreSQL
   pgloader sqlite:auth.db postgresql:///auth_db
   ```

2. **添加Redis**
   ```bash
   docker run -d -p 6379:6379 redis:alpine
   ```

3. **更新代码**
   ```rust
   // 从简化版
   - use auth_system_lite::AuthService;
   + use auth_system::AuthService;

   // 添加Redis配置
   + let redis = RedisConfig::from_env()?;
   + let cache = CacheManager::new(redis, 1000, 3600);
   - let cache = None; // 移除

   + let auth = AuthService::new(db, cache, config);
   ```

### 性能优化建议

#### 简化版优化

如果简化版性能不足, 可以优先优化以下方面:

1. **增加moka缓存容量**
   ```rust
   let cache = Cache::builder()
       .max_capacity(10000)  // 从1000增加到10000
       .time_to_live(Duration::from_secs(7200))  // 从1小时增加到2小时
       .build();
   ```

2. **使用内存数据库替代SQLite**
   ```rust
   let db = SqlitePool::connect("sqlite::memory:").await?;
   ```

3. **预加载用户信息**
   ```rust
   // 启动时预加载常用用户到缓存
   for user_id in [1, 2, 3] {
       if let Ok(info) = auth.get_user_info(user_id).await {
           cache.insert(&format!("user:{}", user_id), info);
       }
   }
   ```

### 推荐选择

| 项目规模 | 推荐版本 |
|---------|---------|
| 个人项目/学习 | 简化版 |
| MVP/原型 | 简化版 |
| 小型SaaS (<500用户) | 简化版 |
| 中型SaaS (500-5000用户) | 完整版 (可只用Redis, 不用审计日志) |
| 大型SaaS (>5000用户) | 完整版 |

### 总结

- **简化版**: 开发效率 > 性能, 适合快速迭代
- **完整版**: 性能 > 开发效率, 适合生产环境

建议: **先从简化版开始, 根据实际需求逐步升级到完整版**
