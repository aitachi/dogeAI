//! 性能基准测试
//!
//! 测试各核心组件的性能表现

use std::time::Instant;

#[cfg(test)]
mod benchmarks {
    use super::*;

    /// 基准测试: 计费规则查询
    #[test]
    fn benchmark_billing_rule_lookup() {
        use billing_gateway::models::BillingRule;

        const ITERATIONS: u32 = 1_000_000;
        let start = Instant::now();

        for i in 0..ITERATIONS {
            let model = match i % 5 {
                0 => "opus",
                1 => "sonnet",
                2 => "haiku",
                3 => "codex",
                _ => "gemini",
            };
            let _ = BillingRule::get(model);
        }

        let elapsed = start.elapsed();
        let ops_per_sec = ITERATIONS as f64 / elapsed.as_secs_f64();

        println!("\n=== 计费规则查询性能 ===");
        println!("迭代次数: {}", ITERATIONS);
        println!("总耗时: {:?}", elapsed);
        println!("吞吐量: {:.2} ops/sec", ops_per_sec);
        println!("平均延迟: {:.2} ns/op", elapsed.as_nanos() as f64 / ITERATIONS as f64);

        assert!(ops_per_sec > 100_000.0, "性能低于预期");
    }

    /// 基准测试: Token限流器
    #[test]
    fn benchmark_rate_limiter() {
        use billing_gateway::ratelimit::TokenRateLimiter;

        const ITERATIONS: u32 = 100_000;
        let limiter = TokenRateLimiter::new();
        let start = Instant::now();

        for i in 0..ITERATIONS {
            let model = match i % 3 {
                0 => "opus",
                1 => "sonnet",
                _ => "haiku",
            };
            let _ = limiter.check_tokens(model, 10);
        }

        let elapsed = start.elapsed();
        let ops_per_sec = ITERATIONS as f64 / elapsed.as_secs_f64();

        println!("\n=== Token限流器性能 ===");
        println!("迭代次数: {}", ITERATIONS);
        println!("总耗时: {:?}", elapsed);
        println!("吞吐量: {:.2} ops/sec", ops_per_sec);
        println!("平均延迟: {:.2} ns/op", elapsed.as_nanos() as f64 / ITERATIONS as f64);

        assert!(ops_per_sec > 50_000.0, "限流器性能低于预期");
    }

    /// 基准测试: 序列化/反序列化
    #[test]
    fn benchmark_json_serialization() {
        use billing_gateway::models::{ChatRequest, ChatMessage};
        use serde_json;

        let request = ChatRequest {
            model: "opus".to_string(),
            messages: vec![
                ChatMessage { role: "user".to_string(), content: "Hello".to_string() },
                ChatMessage { role: "assistant".to_string(), content: "Hi there!".to_string() },
            ],
            stream: false,
            max_tokens: Some(1000),
            temperature: Some(0.7),
        };

        const ITERATIONS: u32 = 10_000;
        let start = Instant::now();

        for _ in 0..ITERATIONS {
            let json = serde_json::to_string(&request).unwrap();
            let _ = serde_json::from_str::<ChatRequest>(&json).unwrap();
        }

        let elapsed = start.elapsed();
        let ops_per_sec = ITERATIONS as f64 / elapsed.as_secs_f64();

        println!("\n=== JSON序列化性能 ===");
        println!("迭代次数: {}", ITERATIONS);
        println!("总耗时: {:?}", elapsed);
        println!("吞吐量: {:.2} ops/sec", ops_per_sec);
        println!("平均延迟: {:.2} μs/op", elapsed.as_micros() as f64 / ITERATIONS as f64);

        assert!(ops_per_sec > 10_000.0, "序列化性能低于预期");
    }

    /// 基准测试: 错误处理
    #[test]
    fn benchmark_error_handling() {
        use billing_gateway::models::{AppError, Result};

        const ITERATIONS: u32 = 100_000;
        let start = Instant::now();

        for i in 0..ITERATIONS {
            let result: Result<()> = match i % 4 {
                0 => Err(AppError::Unauthorized),
                1 => Err(AppError::RateLimitExceeded),
                2 => Err(AppError::UserNotFound),
                _ => Ok(()),
            };
            let _ = result;
        }

        let elapsed = start.elapsed();
        let ops_per_sec = ITERATIONS as f64 / elapsed.as_secs_f64();

        println!("\n=== 错误处理性能 ===");
        println!("迭代次数: {}", ITERATIONS);
        println!("总耗时: {:?}", elapsed);
        println!("吞吐量: {:.2} ops/sec", ops_per_sec);
        println!("平均延迟: {:.2} ns/op", elapsed.as_nanos() as f64 / ITERATIONS as f64);
    }

    /// 基准测试: 模型名称解析
    #[test]
    fn benchmark_model_resolution() {
        use billing_gateway::models::BillingRule;

        let models = vec![
            "opus", "glm-4-plus", "sonnet", "glm-4-flashx",
            "haiku", "glm-4-flash", "codex", "qwen-coder",
            "gemini", "qwen-turbo"
        ];

        const ITERATIONS: u32 = 1_000_000;
        let start = Instant::now();

        for i in 0..ITERATIONS {
            let model = models[i as usize % models.len()];
            let _ = BillingRule::get(model);
        }

        let elapsed = start.elapsed();
        let ops_per_sec = ITERATIONS as f64 / elapsed.as_secs_f64();

        println!("\n=== 模型解析性能 ===");
        println!("迭代次数: {}", ITERATIONS);
        println!("模型数量: {}", models.len());
        println!("总耗时: {:?}", elapsed);
        println!("吞吐量: {:.2} ops/sec", ops_per_sec);
        println!("平均延迟: {:.2} ns/op", elapsed.as_nanos() as f64 / ITERATIONS as f64);

        assert!(ops_per_sec > 200_000.0, "模型解析性能低于预期");
    }

    /// 内存使用分析
    #[test]
    fn analyze_memory_footprint() {
        use billing_gateway::ratelimit::TokenRateLimiter;
        use std::mem;

        let limiter = TokenRateLimiter::new();
        let limiter_size = mem::size_of_val(&limiter);

        println!("\n=== 内存占用分析 ===");
        println!("TokenRateLimiter 结构体大小: {} bytes", limiter_size);
        println!("RateLimitConfig 大小: {} bytes", mem::size_of::<billing_gateway::ratelimit::RateLimitConfig>());
        println!("ChatRequest 大小: {} bytes", mem::size_of::<billing_gateway::models::ChatRequest>());
        println!("AppError 大小: {} bytes", mem::size_of::<billing_gateway::models::AppError>());

        // 验证内存占用合理
        assert!(limiter_size < 200, "限流器内存占用过大");
    }

    /// 并发性能测试 (模拟)
    #[test]
    fn benchmark_concurrent_access() {
        use std::sync::atomic::{AtomicU64, Ordering};
        use std::sync::Arc;
        use std::thread;

        let counter = Arc::new(AtomicU64::new(0));
        const THREADS: u32 = 4;
        const ITERATIONS_PER_THREAD: u32 = 25_000;

        let start = Instant::now();
        let mut handles = vec![];

        for _ in 0..THREADS {
            let counter = counter.clone();
            let handle = thread::spawn(move || {
                for _ in 0..ITERATIONS_PER_THREAD {
                    counter.fetch_add(1, Ordering::Relaxed);
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        let elapsed = start.elapsed();
        let total_ops = THREADS as u64 * ITERATIONS_PER_THREAD as u64;
        let ops_per_sec = total_ops as f64 / elapsed.as_secs_f64();

        println!("\n=== 并发性能测试 ===");
        println!("线程数: {}", THREADS);
        println!("总操作数: {}", total_ops);
        println!("总耗时: {:?}", elapsed);
        println!("吞吐量: {:.2} ops/sec", ops_per_sec);

        assert_eq!(counter.load(Ordering::Relaxed), total_ops);
    }
}
