//! 计费系统全面测试套件
//!
//! 运行方式: cargo test --test comprehensive_tests

use std::time::Duration;

// 测试结果结构
#[derive(Debug)]
pub struct TestResult {
    pub name: String,
    pub passed: bool,
    pub duration: Duration,
    pub message: String,
}

#[derive(Debug)]
pub struct TestSuite {
    pub name: String,
    pub results: Vec<TestResult>,
}

impl TestSuite {
    pub fn new(name: &str) -> Self {
        TestSuite {
            name: name.to_string(),
            results: Vec::new(),
        }
    }

    pub fn add(&mut self, result: TestResult) {
        self.results.push(result);
    }

    pub fn summary(&self) -> TestSummary {
        let passed = self.results.iter().filter(|r| r.passed).count();
        let failed = self.results.len() - passed;
        let total_duration: Duration = self.results.iter().map(|r| r.duration).sum();

        TestSummary {
            suite_name: self.name.clone(),
            total_tests: self.results.len(),
            passed,
            failed,
            total_duration,
        }
    }
}

#[derive(Debug)]
pub struct TestSummary {
    pub suite_name: String,
    pub total_tests: usize,
    pub passed: usize,
    pub failed: usize,
    pub total_duration: Duration,
}

pub fn print_report(suites: &[TestSuite]) {
    println!("\n╔════════════════════════════════════════════════════════════════╗");
    println!("║                    测试报告                                  ║");
    println!("╚════════════════════════════════════════════════════════════════╝");

    let mut total_passed = 0;
    let mut total_failed = 0;

    for suite in suites {
        let summary = suite.summary();
        total_passed += summary.passed;
        total_failed += summary.failed;

        println!("\n【{}】", suite.name);
        println!("══════════════════════════════════════════════════════════");

        for result in &suite.results {
            let status = if result.passed { "✓" } else { "✗" };
            let time = format!("{}ms", result.duration.as_millis());
            println!("  {} {:40} {:8} {}", status, result.name, time, result.message);
        }

        println!("────────────────────────────────────────────────────────────");
        println!("  小计: {}/{} 通过, 耗时: {}ms",
            summary.passed, summary.total_tests, summary.total_duration.as_millis());
    }

    println!("\n╔════════════════════════════════════════════════════════════════╗");
    println!("║                        总计                                    ║");
    println!("║  通过: {}  失败: {}  成功率: {:.1}%                      ║",
        total_passed, total_failed,
        if total_passed + total_failed > 0 {
            (total_passed as f64 / (total_passed + total_failed) as f64) * 100.0
        } else {
            0.0
        });
    println!("╚════════════════════════════════════════════════════════════════╝\n");
}
