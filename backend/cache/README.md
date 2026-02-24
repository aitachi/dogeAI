# Two-Level Cache (两级缓存系统)

高性能的两级缓存实现，结合本地 LRU 缓存和 Redis 分布式缓存。

## 架构

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│  Application│────▶│  L1 (LRU)   │────▶│  L2 (Redis) │
└─────────────┘     └─────────────┘     └─────────────┘
```

## 特性

- **L1 缓存**: 本地内存 LRU 缓存，容量 10000 条目，默认 TTL 60 秒
- **L2 缓存**: Redis 分布式缓存，默认 TTL 300 秒
- **原子操作**: 使用原子操作收集指标，无锁性能
- **缓存回写**: L2 命中时自动回写 L1
- **统计指标**: L1/L2 命中率、容量使用等

## 性能目标

- L1 命中率 >= 70%
- L2 命中率 >= 90%
- 整体命中率 >= 95%

## 依赖

- `lru` - LRU 缓存实现
- `redis` - Redis 客户端
- `tokio` - 异步运行时
- `serde` - 序列化

## 使用示例

```rust
use two_level_cache::{TwoLevelCache, TwoLevelCacheConfig};

// 创建缓存
let config = TwoLevelCacheConfig::default();
let cache = TwoLevelCache::<String>::new(config).await?;

// 设置值
cache.set("key1", "value1", Some(60)).await?;

// 获取值
if let Some(value) = cache.get("key1").await? {
    println!("Found: {}", value);
}

// 获取统计
let stats = cache.stats().await?;
println!("L1 命中率: {:.2}%", stats.l1_hit_rate() * 100.0);
```

## 构建和运行

```bash
# 构建库
cargo build --release

# 运行示例
cargo run --bin cache-server
```

## 配置

| 参数 | 默认值 | 说明 |
|------|--------|------|
| l1_capacity | 10000 | L1 缓存容量 |
| l1_default_ttl | 60 | L1 默认 TTL (秒) |
| l2_default_ttl | 300 | L2 默认 TTL (秒) |
| redis_url | redis://127.0.0.1:6379 | Redis 连接 URL |
| key_prefix | cache2: | Redis key 前缀 |
