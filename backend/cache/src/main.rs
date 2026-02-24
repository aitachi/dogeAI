//! 两级缓存系统服务器
//!
//! 运行:
//!   - 服务器模式: cargo run --bin cache-server
//!   - 基准测试: cargo run --bin cache-bench

use clap::{Parser, Subcommand};
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::sleep;

use two_level_cache::{CacheEntry, TwoLevelCache, TwoLevelCacheConfig};

/// 用户信息示例类型
#[derive(Debug, Clone, Serialize, Deserialize)]
struct UserInfo {
    user_id: u64,
    username: String,
    email: String,
    data: Vec<u8>, // 用于控制内存占用
}

impl UserInfo {
    fn new(user_id: u64, data_size: usize) -> Self {
        Self {
            user_id,
            username: format!("user_{}", user_id),
            email: format!("user_{}@example.com", user_id),
            data: vec![0; data_size],
        }
    }

    fn approx_size_bytes(&self) -> usize {
        self.username.len() + self.email.len() + self.data.len() + 16
    }
}

#[derive(Parser)]
#[command(name = "cache-server")]
#[command(about = "两级缓存系统 (LRU + Redis)", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 启动缓存服务器
    Server {
        /// Redis URL
        #[arg(short, long, default_value = "redis://127.0.0.1:6379")]
        redis: String,

        /// L1 缓存容量
        #[arg(short, long, default_value = "10000")]
        capacity: usize,

        /// L1 TTL (秒)
        #[arg(short = '1', long, default_value = "60")]
        l1_ttl: u64,

        /// L2 TTL (秒)
        #[arg(short = '2', long, default_value = "300")]
        l2_ttl: u64,
    },

    /// 运行性能测试
    Bench {
        /// Redis URL
        #[arg(short, long, default_value = "redis://127.0.0.1:6379")]
        redis: String,

        /// L1 缓存容量
        #[arg(short, long, default_value = "10000")]
        capacity: usize,

        /// 测试数据集大小
        #[arg(short, long, default_value = "100000")]
        dataset_size: usize,

        /// 单条数据大小 (字节)
        #[arg(short, long, default_value = "1024")]
        data_size: usize,

        /// 请求总数
        #[arg(short, long, default_value = "1000000")]
        requests: usize,

        /// 并发数
        #[arg(short = 'c', long, default_value = "100")]
        concurrency: usize,

        /// 读取比例 (0-100, 例如 80 表示 80% 读)
        #[arg(short, long, default_value = "80")]
        read_ratio: u8,

        /// Zipf 参数 (0 = 均匀分布, 越大约倾斜)
        #[arg(short = 'z', long, default_value = "0.9")]
        zipf: f64,
    },

    /// 验证配置
    Verify {
        /// Redis URL
        #[arg(short, long, default_value = "redis://127.0.0.1:6379")]
        redis: String,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Server { redis, capacity, l1_ttl, l2_ttl } => {
            run_server(redis, capacity, l1_ttl, l2_ttl).await?;
        }
        Commands::Bench {
            redis,
            capacity,
            dataset_size,
            data_size,
            requests,
            concurrency,
            read_ratio,
            zipf,
        } => {
            run_bench(
                redis,
                capacity,
                dataset_size,
                data_size,
                requests,
                concurrency,
                read_ratio,
                zipf,
            )
            .await?;
        }
        Commands::Verify { redis } => {
            verify_config(redis).await?;
        }
    }

    Ok(())
}

async fn run_server(
    redis_url: String,
    capacity: usize,
    l1_ttl: u64,
    l2_ttl: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = TwoLevelCacheConfig {
        l1_capacity: capacity,
        l1_default_ttl: l1_ttl,
        l2_default_ttl: l2_ttl,
        redis_url: redis_url.clone(),
        ..Default::default()
    };

    let cache = TwoLevelCache::<UserInfo>::new(config).await?;

    println!("🚀 两级缓存服务器已启动");
    println!("   L1 容量: {} 条目", capacity);
    println!("   L1 TTL: {} 秒", l1_ttl);
    println!("   L2 TTL: {} 秒", l2_ttl);
    println!("   Redis: {}", redis_url);

    // 定期打印统计
    let mut interval = tokio::time::interval(Duration::from_secs(10));
    loop {
        interval.tick().await;
        let stats = cache.stats().await?;
        print_stats(&stats);
    }
}

async fn verify_config(redis_url: String) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 验证 Redis 配置...\n");

    let client = redis::Client::open(redis_url)?;
    let mut conn = client.get_async_connection().await?;

    // 获取配置
    let maxmemory_vec: Vec<String> = redis::cmd("CONFIG").arg("GET").arg("maxmemory").query_async(&mut conn).await?;
    let policy_vec: Vec<String> = redis::cmd("CONFIG").arg("GET").arg("maxmemory-policy").query_async(&mut conn).await?;
    let maxmemory = maxmemory_vec.get(1).map(|s| s.as_str()).unwrap_or("unknown");
    let policy = policy_vec.get(1).map(|s| s.as_str()).unwrap_or("unknown");

    let info_stats: String = redis::cmd("INFO").arg("stats").query_async(&mut conn).await?;
    let info_memory: String = redis::cmd("INFO").arg("memory").query_async(&mut conn).await?;

    println!("Redis 配置:");
    println!("  maxmemory: {}", maxmemory);
    println!("  maxmemory-policy: {}", policy);

    // 解析统计
    for line in info_stats.lines() {
        if line.starts_with("evicted_keys:") || line.starts_with("keyspace_hits:") || line.starts_with("keyspace_misses:") {
            println!("  {}", line);
        }
    }

    for line in info_memory.lines() {
        if line.starts_with("used_memory_human:") || line.starts_with("maxmemory_human:") {
            println!("  {}", line);
        }
    }

    Ok(())
}

/// Zipf 分布生成器 (用于模拟真实访问模式)
struct ZipfGenerator {
    n: usize,
    s: f64,
}

impl ZipfGenerator {
    fn new(n: usize, s: f64) -> Self {
        Self { n, s }
    }

    fn gen(&self) -> usize {
        use rand::distributions::{Distribution, Uniform};
        let mut rng = thread_rng();
        let u: f64 = Uniform::new(0.0, 1.0).sample(&mut rng);

        if self.s <= 0.0 {
            return (u * self.n as f64) as usize;
        }

        // 简化的 Zipf 采样
        let alpha = 1.0 / (1.0 - self.s);
        let zeta_n = (1..=self.n).fold(0.0, |acc, i| acc + 1.0 / (i as f64).powf(self.s));

        let mut sum = 0.0;
        for k in 1..=self.n {
            sum += 1.0 / (k as f64).powf(self.s);
            if sum / zeta_n >= u {
                return k - 1;
            }
        }
        self.n - 1
    }
}

async fn run_bench(
    redis_url: String,
    capacity: usize,
    dataset_size: usize,
    data_size: usize,
    total_requests: usize,
    concurrency: usize,
    read_ratio: u8,
    zipf_param: f64,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("🏁 两级缓存性能测试");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  数据集大小: {} 条目", dataset_size);
    println!("  单条数据: {} 字节", data_size);
    println!("  L1 容量: {} 条目", capacity);
    println!("  请求数: {}", total_requests);
    println!("  并发: {}", concurrency);
    println!("  读比例: {}%", read_ratio);
    println!("  Zipf 参数: {}", zipf_param);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // 清空 Redis
    let client = redis::Client::open(redis_url.clone())?;
    let mut conn = client.get_async_connection().await?;
    let _: () = redis::cmd("FLUSHDB").query_async(&mut conn).await?;
    println!("✓ Redis 已清空");

    // 配置 Redis
    let _: () = redis::cmd("CONFIG")
        .arg("SET")
        .arg("maxmemory")
        .arg("256mb")
        .query_async(&mut conn)
        .await?;
    let _: () = redis::cmd("CONFIG")
        .arg("SET")
        .arg("maxmemory-policy")
        .arg("volatile-lru")
        .query_async(&mut conn)
        .await?;
    println!("✓ Redis maxmemory=256MB, policy=volatile-lru\n");

    // 创建缓存
    let config = TwoLevelCacheConfig {
        l1_capacity: capacity,
        l1_default_ttl: 60,
        l2_default_ttl: 300,
        redis_url: redis_url.clone(),
        ..Default::default()
    };

    let cache = Arc::new(TwoLevelCache::<UserInfo>::new(config).await?);
    println!("✓ 缓存初始化完成\n");

    // 预填充数据
    println!("📦 预填充数据...");
    let warm_start = Instant::now();

    let mut batch = HashMap::new();
    for i in 0..dataset_size {
        let key = format!("key:{}", i);
        let value = UserInfo::new(i as u64, data_size);
        batch.insert(key, value);

        if batch.len() >= 1000 {
            cache.set_multi(batch.clone(), None).await?;
            batch.clear();
        }
    }
    if !batch.is_empty() {
        cache.set_multi(batch, None).await?;
    }

    let warm_duration = warm_start.elapsed();
    println!("✓ 预填充完成 ({:.2}s)\n", warm_duration.as_secs_f64());

    // Zipf 生成器
    let zipf = Arc::new(ZipfGenerator::new(dataset_size, zipf_param));

    // 运行基准测试
    println!("🚀 开始性能测试...\n");

    let start = Instant::now();
    let requests_per_thread = total_requests / concurrency;

    let mut handles = vec![];
    for _ in 0..concurrency {
        let cache_clone = cache.clone();
        let zipf_clone = zipf.clone();
        let rp = requests_per_thread;

        handles.push(tokio::spawn(async move {
            for _ in 0..rp {
                let key_idx = zipf_clone.gen();
                let key = format!("key:{}", key_idx);

                // 根据比例决定读或写
                let read = thread_rng().gen_range(0..100) < read_ratio;

                if read {
                    let _ = cache_clone.get(&key).await;
                } else {
                    let value = UserInfo::new(key_idx as u64, data_size);
                    let _ = cache_clone.set(&key, value, None).await;
                }
            }
        }));
    }

    for handle in handles {
        handle.await?;
    }

    let duration = start.elapsed();

    // 获取最终统计
    let stats = cache.stats().await?;

    // 打印结果
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📊 测试结果");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  总耗时: {:.2}s", duration.as_secs_f64());
    println!("  QPS: {:.0}", total_requests as f64 / duration.as_secs_f64());
    println!("  平均延迟: {:.2}μs", duration.as_micros() as f64 / total_requests as f64);
    println!();

    print_stats(&stats);

    // 检查是否达到目标
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("🎯 目标检查");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let l1_rate = stats.l1_hit_rate();
    let l2_rate = stats.l2_hit_rate();
    let overall_rate = stats.overall_hit_rate();

    println!(
        "  L1 命中率: {:.1}% ({})",
        l1_rate * 100.0,
        if l1_rate >= 0.7 { "✓ 达标" } else { "✗ 未达标" }
    );
    println!(
        "  L2 命中率: {:.1}% ({})",
        l2_rate * 100.0,
        if l2_rate >= 0.9 { "✓ 达标" } else { "✗ 未达标" }
    );
    println!(
        "  整体命中率: {:.1}%",
        overall_rate * 100.0
    );
    println!("  L2 驱逐数: {}", stats.l2_evicted_keys);
    println!("  Redis 内存: {} / {}",
        bytes_to_human(stats.redis_memory_used),
        bytes_to_human(stats.redis_memory_max)
    );

    // 估算本地缓存内存占用
    let estimated_l1_memory = stats.l1_size as u64 * (data_size as u64 + 200);
    println!("  L1 预估内存: {} ({})",
        bytes_to_human(estimated_l1_memory),
        if estimated_l1_memory < 200 * 1024 * 1024 { "✓ < 200MB" } else { "✗ 超标" }
    );

    Ok(())
}

fn print_stats(stats: &two_level_cache::CacheStats) {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📈 缓存统计");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  L1 大小: {} / {} 条目", stats.l1_size, stats.l1_capacity);
    println!("  L1 命中: {}", stats.l1_hits);
    println!("  L1 未命中: {}", stats.l1_misses);
    println!("  L1 命中率: {:.2}%", stats.l1_hit_rate() * 100.0);
    println!();
    println!("  L2 键数: {}", stats.l2_keys_count);
    println!("  L2 命中: {}", stats.l2_hits);
    println!("  L2 未命中: {}", stats.l2_misses);
    println!("  L2 命中率: {:.2}%", stats.l2_hit_rate() * 100.0);
    println!("  L2 驱逐: {}", stats.l2_evicted_keys);
    println!();
    println!("  整体命中率: {:.2}%", stats.overall_hit_rate() * 100.0);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
}

fn bytes_to_human(bytes: u64) -> String {
    if bytes == 0 {
        return "0B".to_string();
    }
    const units: &[&str] = &["B", "KB", "MB", "GB"];
    let mut size = bytes as f64;
    let mut unit_idx = 0;
    while size >= 1024.0 && unit_idx < units.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }
    format!("{:.1}{}", size, units[unit_idx])
}
