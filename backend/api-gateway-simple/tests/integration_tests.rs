// DogeAI API Gateway 集成测试
// 运行: cargo test --test integration_tests

use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::{Duration, Instant};

const BASE_URL: &str = "http://127.0.0.1:8081";

#[derive(Debug, Serialize, Clone)]
struct LoginRequest {
    username: String,
    password: String,
}

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    #[serde(default)]
    stream: bool,
    #[serde(default = "default_max_tokens")]
    max_tokens: u32,
}

#[derive(Debug, Serialize, Clone)]
struct ChatMessage {
    role: String,
    content: String,
}

fn default_max_tokens() -> u32 { 2000 }

#[derive(Debug, Deserialize)]
struct LoginResponse {
    status: String,
    #[serde(default)]
    token: Option<String>,
    #[serde(default)]
    user: Option<UserInfo>,
}

#[derive(Debug, Deserialize)]
struct UserInfo {
    id: String,
    username: String,
    balance: i64,
    #[serde(default)]
    tier: String,
}

struct TestContext {
    client: Client,
    admin_token: Option<String>,
    user_tokens: Vec<String>,
}

impl TestContext {
    fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .unwrap(),
            admin_token: None,
            user_tokens: Vec::new(),
        }
    }

    async fn login(&mut self, username: &str, password: &str) -> Result<String, String> {
        let resp = self.client
            .post(format!("{}/api/user/login", BASE_URL))
            .json(&LoginRequest {
                username: username.to_string(),
                password: password.to_string(),
            })
            .send()
            .await
            .map_err(|e| format!("请求失败: {}", e))?;

        let status = resp.status();
        let body = resp.text().await.map_err(|e| format!("响应解析失败: {}", e))?;

        if !status.is_success() {
            return Err(format!("登录失败 ({}): {}", status.as_u16(), body));
        }

        let login_resp: LoginResponse = serde_json::from_str(&body)
            .map_err(|e| format!("响应解析失败: {}", e))?;

        let token = login_resp.token.ok_or_else(|| "响应缺少token".to_string())?;

        // 保存token到列表
        self.user_tokens.push(token.clone());

        Ok(token)
    }

    async fn get_balance(&self, token: &str) -> Result<i64, String> {
        let resp = self.client
            .get(format!("{}/api/user/balance", BASE_URL))
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| format!("请求失败: {}", e))?;

        let status = resp.status();
        let body = resp.text().await.map_err(|e| format!("响应解析失败: {}", e))?;

        if !status.is_success() {
            return Err(format!("请求失败 ({}): {}", status.as_u16(), body));
        }

        let balance: serde_json::Value = serde_json::from_str(&body)
            .map_err(|e| format!("响应解析失败: {}", e))?;

        Ok(balance["balance"].as_i64().unwrap_or(0))
    }
}

// ========== 认证模块测试 ==========

#[tokio::test]
async fn test_auth_register_normal() {
    let client = Client::new();

    let username = format!("testuser_{}", chrono::Utc::now().timestamp());
    let resp = client
        .post(format!("{}/api/user/register", BASE_URL))
        .json(&json!({
            "username": username,
            "password": "password123",
            "email": format!("{}@example.com", username)
        }))
        .send()
        .await
        .expect("注册请求失败");

    assert!(resp.status().is_success() || resp.status().as_u16() == 400,
        "注册应该成功或返回用户已存在");
}

#[tokio::test]
async fn test_auth_login_success() {
    let mut ctx = TestContext::new();

    // 使用测试用户登录
    let result = ctx.login("test_user_001", "password123").await;
    assert!(result.is_ok(), "登录应该成功: {:?}", result.err());
    let token = result.unwrap();
    assert!(!token.is_empty(), "Token不应为空");
}

#[tokio::test]
async fn test_auth_login_wrong_password() {
    let client = Client::new();

    let resp = client
        .post(format!("{}/api/user/login", BASE_URL))
        .json(&LoginRequest {
            username: "test_user_001".to_string(),
            password: "wrongpassword".to_string(),
        })
        .send()
        .await
        .expect("请求失败");

    assert_eq!(resp.status().as_u16(), 401, "错误密码应返回401");
}

#[tokio::test]
async fn test_auth_login_empty_credentials() {
    let client = Client::new();

    // 空用户名
    let resp1 = client
        .post(format!("{}/api/user/login", BASE_URL))
        .json(&json!({"username": "", "password": "pass"}))
        .send()
        .await
        .expect("请求失败");
    assert!(!resp1.status().is_success(), "空用户名应被拒绝");

    // 空密码
    let resp2 = client
        .post(format!("{}/api/user/login", BASE_URL))
        .json(&json!({"username": "user", "password": ""}))
        .send()
        .await
        .expect("请求失败");
    assert!(!resp2.status().is_success(), "空密码应被拒绝");
}

#[tokio::test]
async fn test_auth_token_without_header() {
    let client = Client::new();

    let resp = client
        .get(format!("{}/api/user/balance", BASE_URL))
        .send()
        .await
        .expect("请求失败");

    assert_eq!(resp.status().as_u16(), 401, "无认证应返回401");
}

#[tokio::test]
async fn test_auth_token_with_bearer() {
    let mut ctx = TestContext::new();
    let token = ctx.login("test_user_001", "password123").await.expect("登录失败");

    let balance = ctx.get_balance(&token).await.expect("获取余额失败");
    assert!(balance >= 0, "余额应该非负");
}

#[tokio::test]
async fn test_auth_token_with_x_api_key() {
    let client = Client::new();

    let resp = client
        .get(format!("{}/api/user/balance", BASE_URL))
        .header("x-api-key", "test-api-key")
        .send()
        .await
        .expect("请求失败");

    // 应该失败，因为api-key无效
    assert!(!resp.status().is_success() || resp.status().as_u16() == 401);
}

#[tokio::test]
async fn test_auth_token_invalid_format() {
    let client = Client::new();

    let resp = client
        .get(format!("{}/api/user/balance", BASE_URL))
        .header("Authorization", "Bearer invalid.token.here")
        .send()
        .await
        .expect("请求失败");

    assert!(!resp.status().is_success(), "无效token应被拒绝");
}

#[tokio::test]
async fn test_auth_logout() {
    let mut ctx = TestContext::new();
    let token = ctx.login("test_user_001", "password123").await.expect("登录失败");

    // 登出 (注意: 这个端点可能不存在)
    let resp = ctx.client
        .post(format!("{}/v1/auth/logout", BASE_URL))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .expect("请求失败");

    let status = resp.status();
    if status.is_success() {
        // 如果登出成功，验证token是否失效
        let balance_result = ctx.get_balance(&token).await;
        assert!(balance_result.is_err(), "登出后token应失效");
    }
}

#[tokio::test]
async fn test_auth_concurrent_login() {
    let client = Client::new();
    let username = "test_user_001";
    let password = "password123";

    // 并发10个登录请求
    let handles: Vec<_> = (0..10)
        .map(|_| {
            let client = client.clone();
            let username = username.to_string();
            let password = password.to_string();
            tokio::spawn(async move {
                let resp = client
                    .post(format!("{}/api/user/login", BASE_URL))
                    .json(&LoginRequest { username, password })
                    .send()
                    .await;
                match resp {
                    Ok(r) => r.status(),
                    Err(_) => reqwest::StatusCode::SERVICE_UNAVAILABLE,
                }
            })
        })
        .collect();

    let results: Vec<_> = futures_util::future::join_all(handles).await;

    // 所有请求应该成功
    for result in results {
        let status = result.unwrap_or(reqwest::StatusCode::SERVICE_UNAVAILABLE);
        assert!(status.is_success(), "并发登录应该都成功");
    }
}

// ========== 聊天模块测试 ==========

#[tokio::test]
async fn test_chat_normal_request() {
    let mut ctx = TestContext::new();
    let token = ctx.login("test_user_001", "password123").await.expect("登录失败");

    let chat_req = ChatRequest {
        model: "opus".to_string(),
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: "Hello, how are you?".to_string(),
        }],
        stream: false,
        max_tokens: 100,
    };

    let resp = ctx.client
        .post(format!("{}/v1/chat/completions", BASE_URL))
        .header("Authorization", format!("Bearer {}", token))
        .json(&chat_req)
        .send()
        .await
        .expect("聊天请求失败");

    // 根据实际API实现调整断言
    println!("聊天响应状态: {}", resp.status());
}

#[tokio::test]
async fn test_chat_empty_messages() {
    let mut ctx = TestContext::new();
    let token = ctx.login("test_user_001", "password123").await.expect("登录失败");

    let chat_req = ChatRequest {
        model: "opus".to_string(),
        messages: vec![],
        stream: false,
        max_tokens: 100,
    };

    let resp = ctx.client
        .post(format!("{}/v1/chat/completions", BASE_URL))
        .header("Authorization", format!("Bearer {}", token))
        .json(&chat_req)
        .send()
        .await
        .expect("聊天请求失败");

    assert!(!resp.status().is_success() || resp.status().as_u16() == 400,
        "空消息应被拒绝");
}

#[tokio::test]
async fn test_chat_model_not_exist() {
    let mut ctx = TestContext::new();
    let token = ctx.login("test_user_001", "password123").await.expect("登录失败");

    let chat_req = ChatRequest {
        model: "nonexistent_model".to_string(),
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: "test".to_string(),
        }],
        stream: false,
        max_tokens: 100,
    };

    let resp = ctx.client
        .post(format!("{}/v1/chat/completions", BASE_URL))
        .header("Authorization", format!("Bearer {}", token))
        .json(&chat_req)
        .send()
        .await
        .expect("聊天请求失败");

    assert!(!resp.status().is_success() || resp.status().as_u16() == 400,
        "不存在的模型应被拒绝");
}

#[tokio::test]
async fn test_chat_streaming_request() {
    let mut ctx = TestContext::new();
    let token = ctx.login("test_user_001", "password123").await.expect("登录失败");

    let chat_req = ChatRequest {
        model: "opus".to_string(),
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: "Hello!".to_string(),
        }],
        stream: true,
        max_tokens: 100,
    };

    let resp = ctx.client
        .post(format!("{}/v1/chat/completions", BASE_URL))
        .header("Authorization", format!("Bearer {}", token))
        .json(&chat_req)
        .send()
        .await
        .expect("聊天请求失败");

    println!("流式聊天响应状态: {}", resp.status());
}

#[tokio::test]
async fn test_chat_multi_turn_conversation() {
    let mut ctx = TestContext::new();
    let token = ctx.login("test_user_001", "password123").await.expect("登录失败");

    let chat_req = ChatRequest {
        model: "opus".to_string(),
        messages: vec![
            ChatMessage { role: "system".to_string(), content: "You are a helpful assistant.".to_string() },
            ChatMessage { role: "user".to_string(), content: "First message".to_string() },
            ChatMessage { role: "assistant".to_string(), content: "First response".to_string() },
            ChatMessage { role: "user".to_string(), content: "Second message".to_string() },
        ],
        stream: false,
        max_tokens: 100,
    };

    let resp = ctx.client
        .post(format!("{}/v1/chat/completions", BASE_URL))
        .header("Authorization", format!("Bearer {}", token))
        .json(&chat_req)
        .send()
        .await
        .expect("聊天请求失败");

    println!("多轮对话响应状态: {}", resp.status());
}

#[tokio::test]
async fn test_chat_system_message() {
    let mut ctx = TestContext::new();
    let token = ctx.login("test_user_001", "password123").await.expect("登录失败");

    let chat_req = ChatRequest {
        model: "opus".to_string(),
        messages: vec![
            ChatMessage { role: "system".to_string(), content: "You are a helpful assistant.".to_string() },
            ChatMessage { role: "user".to_string(), content: "Test with system message".to_string() },
        ],
        stream: false,
        max_tokens: 100,
    };

    let resp = ctx.client
        .post(format!("{}/v1/chat/completions", BASE_URL))
        .header("Authorization", format!("Bearer {}", token))
        .json(&chat_req)
        .send()
        .await
        .expect("聊天请求失败");

    println!("带系统消息响应状态: {}", resp.status());
}

#[tokio::test]
async fn test_chat_with_max_tokens() {
    let mut ctx = TestContext::new();
    let token = ctx.login("test_user_001", "password123").await.expect("登录失败");

    let chat_req = ChatRequest {
        model: "opus".to_string(),
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: "Generate a long response.".to_string(),
        }],
        stream: false,
        max_tokens: 5000,
    };

    let resp = ctx.client
        .post(format!("{}/v1/chat/completions", BASE_URL))
        .header("Authorization", format!("Bearer {}", token))
        .json(&chat_req)
        .send()
        .await
        .expect("聊天请求失败");

    println!("大token数响应状态: {}", resp.status());
}

// ========== 用户管理测试 ==========

#[tokio::test]
async fn test_user_balance_query() {
    let mut ctx = TestContext::new();
    let token = ctx.login("test_user_001", "password123").await.expect("登录失败");

    let balance = ctx.get_balance(&token).await.expect("获取余额失败");
    assert!(balance >= 0, "余额应该非负");
    println!("用户余额: {}", balance);
}

#[tokio::test]
async fn test_user_balance_concurrent() {
    let mut ctx = TestContext::new();
    let token = ctx.login("test_user_001", "password123").await.expect("登录失败");

    // 并发查询余额10次
    let handles: Vec<_> = (0..10)
        .map(|_| {
            let client = ctx.client.clone();
            let token = token.clone();
            tokio::spawn(async move {
                let resp = client
                    .get(format!("{}/api/user/balance", BASE_URL))
                    .header("Authorization", format!("Bearer {}", token))
                    .send()
                    .await;

                match resp {
                    Ok(r) => {
                        let status = r.status();
                        if let Ok(body) = r.text().await {
                            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&body) {
                                (status, value["balance"].as_i64())
                            } else {
                                (status, None)
                            }
                        } else {
                            (status, None)
                        }
                    }
                    Err(_) => (reqwest::StatusCode::SERVICE_UNAVAILABLE, None),
                }
            })
        })
        .collect();

    let results: Vec<_> = futures_util::future::join_all(handles).await;

    // 所有查询应该返回相同结果
    let first_balance = results[0].as_ref().unwrap().1;
    for result in &results {
        let (status, balance) = result.as_ref().unwrap();
        assert!(status.is_success(), "并发请求应该成功");
        assert_eq!(*balance, first_balance, "并发余额查询应该返回相同结果");
    }
}

// ========== 健康检查测试 ==========

#[tokio::test]
async fn test_health_check() {
    let client = Client::new();

    let resp = client
        .get(format!("{}/health", BASE_URL))
        .send()
        .await
        .expect("健康检查失败");

    assert!(resp.status().is_success(), "健康检查应该成功");

    let body = resp.text().await.expect("响应解析失败");
    let health: serde_json::Value = serde_json::from_str(&body).expect("JSON解析失败");

    assert_eq!(health["status"], "healthy", "状态应该是healthy");
    assert!(health["version"].is_string(), "应该有版本信息");
    println!("健康检查: {}", health);
}

#[tokio::test]
async fn test_health_check_database() {
    let client = Client::new();

    let resp = client
        .get(format!("{}/health", BASE_URL))
        .send()
        .await
        .expect("健康检查失败");

    let body = resp.text().await.expect("响应解析失败");
    let health: serde_json::Value = serde_json::from_str(&body).expect("JSON解析失败");

    assert!(health["database"].is_string(), "应该有数据库状态");
    println!("数据库状态: {}", health["database"]);
}

#[tokio::test]
async fn test_health_check_cache() {
    let client = Client::new();

    let resp = client
        .get(format!("{}/health", BASE_URL))
        .send()
        .await
        .expect("健康检查失败");

    let body = resp.text().await.expect("响应解析失败");
    let health: serde_json::Value = serde_json::from_str(&body).expect("JSON解析失败");

    if let Some(_cache_stats) = health.get("cache_stats") {
        println!("缓存统计: {}", _cache_stats);
    }
}

// ========== 模型列表测试 ==========

#[tokio::test]
async fn test_models_list() {
    let client = Client::new();

    let resp = client
        .get(format!("{}/v1/models", BASE_URL))
        .send()
        .await
        .expect("模型列表请求失败");

    assert!(resp.status().is_success(), "获取模型列表应该成功");

    let body = resp.text().await.expect("响应解析失败");
    let models: serde_json::Value = serde_json::from_str(&body).expect("JSON解析失败");

    if let Some(data) = models.get("data").and_then(|d| d.as_array()) {
        println!("可用模型数量: {}", data.len());
        assert!(!data.is_empty(), "应该有可用模型");
    }
}

#[tokio::test]
async fn test_models_list_includes_opus() {
    let client = Client::new();

    let resp = client
        .get(format!("{}/v1/models", BASE_URL))
        .send()
        .await
        .expect("模型列表请求失败");

    let body = resp.text().await.expect("响应解析失败");
    let models: serde_json::Value = serde_json::from_str(&body).expect("JSON解析失败");

    if let Some(data) = models.get("data").and_then(|d| d.as_array()) {
        let has_opus = data.iter().any(|m| {
            m.get("id").and_then(|id| id.as_str())
                .map(|id| id.contains("opus") || id.contains("claude-opus"))
                .unwrap_or(false)
        });
        assert!(has_opus, "应该包含Opus模型");
    }
}

// ========== 充值模块测试 ==========

#[tokio::test]
async fn test_recharge_with_valid_card() {
    let mut ctx = TestContext::new();
    let token = ctx.login("test_user_001", "password123").await.expect("登录失败");

    // 先获取初始余额
    let initial_balance = ctx.get_balance(&token).await.expect("获取余额失败");

    // 使用测试充值卡 (需要在测试数据库中创建)
    let resp = ctx.client
        .post(format!("{}/api/recharge/redeem", BASE_URL))
        .header("Authorization", format!("Bearer {}", token))
        .json(&json!({"code": "TEST-CARD-0001"}))
        .send()
        .await
        .expect("充值请求失败");

    println!("充值响应状态: {}", resp.status());

    // 验证余额增加 (如果充值成功)
    if resp.status().is_success() {
        let new_balance = ctx.get_balance(&token).await.expect("获取余额失败");
        assert!(new_balance >= initial_balance, "充值后余额应该增加");
    }
}

// ========== Token查询测试 ==========

#[tokio::test]
async fn test_token_query() {
    let mut ctx = TestContext::new();
    let token = ctx.login("test_user_001", "password123").await.expect("登录失败");

    let resp = ctx.client
        .post(format!("{}/v1/token/query", BASE_URL))
        .header("Authorization", format!("Bearer {}", token))
        .json(&json!({
            "model": "opus",
            "input_tokens": 100,
            "output_tokens": 50
        }))
        .send()
        .await
        .expect("Token查询失败");

    println!("Token查询响应状态: {}", resp.status());
    if resp.status().is_success() {
        let body = resp.text().await.expect("响应解析失败");
        println!("Token查询响应: {}", body);
    }
}

// ========== 统计接口测试 ==========

#[tokio::test]
async fn test_public_stats() {
    let client = Client::new();

    let resp = client
        .get(format!("{}/api/stats", BASE_URL))
        .send()
        .await
        .expect("统计请求失败");

    assert!(resp.status().is_success(), "获取统计应该成功");

    let body = resp.text().await.expect("响应解析失败");
    let stats: serde_json::Value = serde_json::from_str(&body).expect("JSON解析失败");
    println!("统计信息: {}", stats);
}

// ========== 性能测试 ==========

#[tokio::test]
async fn performance_health_check_latency() {
    let client = Client::new();
    let start = Instant::now();

    let resp = client
        .get(format!("{}/health", BASE_URL))
        .send()
        .await
        .expect("健康检查失败");

    let elapsed = start.elapsed();
    assert!(resp.status().is_success(), "健康检查应该成功");
    assert!(elapsed.as_millis() < 100, "健康检查延迟应该 < 100ms");
    println!("健康检查延迟: {}ms", elapsed.as_millis());
}

#[tokio::test]
async fn performance_concurrent_balance_queries() {
    let mut ctx = TestContext::new();
    let token = ctx.login("test_user_001", "password123").await.expect("登录失败");

    let start = Instant::now();

    let handles: Vec<_> = (0..100)
        .map(|_| {
            let client = ctx.client.clone();
            let token = token.clone();
            tokio::spawn(async move {
                let resp = client
                    .get(format!("{}/api/user/balance", BASE_URL))
                    .header("Authorization", format!("Bearer {}", token))
                    .send()
                    .await;

                match resp {
                    Ok(r) => r.status().is_success(),
                    Err(_) => false,
                }
            })
        })
        .collect();

    let results: Vec<_> = futures_util::future::join_all(handles).await;
    let elapsed = start.elapsed();

    let success_count = results.iter().filter(|s| *s.as_ref().unwrap_or(&false)).count();
    assert!(success_count > 95, "至少95%的请求应该成功");
    assert!(elapsed.as_millis() < 5000, "100并发应该在5秒内完成");
    println!("100并发余额查询: {}ms, 成功: {}/{}", elapsed.as_millis(), success_count, results.len());
}

#[tokio::test]
async fn performance_token_verify_with_cache() {
    let mut ctx = TestContext::new();
    let token = ctx.login("test_user_001", "password123").await.expect("登录失败");

    let start = Instant::now();
    let mut success_count = 0;

    // 连续验证token 100次
    for _ in 0..100 {
        let resp = ctx.client
            .get(format!("{}/api/user/balance", BASE_URL))
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .expect("请求失败");

        if resp.status().is_success() {
            success_count += 1;
        } else {
            break;
        }
    }

    let elapsed = start.elapsed();
    let avg_latency = elapsed.as_micros() / 100;

    println!("100次Token验证 (缓存): 总时间={}ms, 平均={}μs, 成功={}",
             elapsed.as_millis(), avg_latency, success_count);
}

// ========== 安全测试 ==========

#[tokio::test]
async fn security_sql_injection_protection() {
    let client = Client::new();

    let malicious_payload = json!({
        "username": "admin'; DROP TABLE users; --",
        "password": "password123"
    });

    let resp = client
        .post(format!("{}/api/user/login", BASE_URL))
        .json(&malicious_payload)
        .send()
        .await
        .expect("登录请求失败");

    // 应该被拒绝或返回错误
    assert!(!resp.status().is_success() || resp.status().as_u16() == 401,
        "SQL注入攻击应被防御");
    println!("SQL注入保护: 状态码={}", resp.status().as_u16());
}

#[tokio::test]
async fn security_xss_in_username() {
    let client = Client::new();

    let xss_payload = json!({
        "username": "<script>alert('xss')</script>",
        "password": "password123"
    });

    let resp = client
        .post(format!("{}/api/user/login", BASE_URL))
        .json(&xss_payload)
        .send()
        .await
        .expect("登录请求失败");

    // 应该被拒绝或正确转义
    println!("XSS保护: 状态码={}", resp.status().as_u16());
}

#[tokio::test]
async fn security_path_traversal_protection() {
    let client = Client::new();

    let paths = vec![
        "/../../../etc/passwd",
        "..\\..\\..\\..\\windows\\system32\\config\\system",
        "/api/user/../../etc/passwd",
    ];

    for path in paths {
        let resp = client
            .get(format!("{}{}", BASE_URL, path))
            .send()
            .await
            .expect("请求失败");

        // 路径遍历应该被拒绝
        assert!(!resp.status().is_success() || resp.status().as_u16() == 404,
            "路径遍历攻击应被防御");
    }
}

// ========== 限流测试 ==========

#[tokio::test]
async fn rate_limiting_base_user() {
    let client = Client::new();

    // 首先登录获取token
    let login_resp = client
        .post(format!("{}/api/user/login", BASE_URL))
        .json(&LoginRequest {
            username: "test_user_002".to_string(),
            password: "password123".to_string(),
        })
        .send()
        .await
        .expect("登录请求失败");

    let body = login_resp.text().await.expect("响应解析失败");
    let login_resp: LoginResponse = serde_json::from_str(&body).unwrap();
    let token = login_resp.token.expect("登录应该返回token");

    // Base用户QPS限制为10，快速发送20个请求
    let start = Instant::now();
    let mut rate_limited = false;

    for i in 1..=21 {
        let resp = client
            .get(format!("{}/api/user/balance", BASE_URL))
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .expect("请求失败");

        if resp.status().as_u16() == 429 {
            rate_limited = true;
            println!("第{}次请求触发限流", i);
            break;
        }

        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    let elapsed = start.elapsed();
    println!("限流测试: 耗时={}ms, 限流触发={}", elapsed.as_millis(), rate_limited);
}

// ========== 错误处理测试 ==========

#[tokio::test]
async fn error_handling_404_not_found() {
    let client = Client::new();

    let resp = client
        .get(format!("{}/nonexistent_endpoint", BASE_URL))
        .send()
        .await
        .expect("请求失败");

    assert_eq!(resp.status().as_u16(), 404, "不存在的端点应返回404");
}

#[tokio::test]
async fn error_handling_405_method_not_allowed() {
    let client = Client::new();

    let resp = client
        .post(format!("{}/health", BASE_URL))
        .send()
        .await
        .expect("请求失败");

    assert_eq!(resp.status().as_u16(), 405, "POST到GET端点应返回405");
}

// ========== 缓存测试 ==========

#[tokio::test]
async fn cache_hit() {
    let mut ctx = TestContext::new();
    let token = ctx.login("test_user_001", "password123").await.expect("登录失败");

    let start = Instant::now();

    // 第一次查询 - 缓存未命中
    let _ = ctx.get_balance(&token).await;

    let first_time = start.elapsed();

    // 第二次查询 - 缓存命中
    let start2 = Instant::now();
    let _ = ctx.get_balance(&token).await;
    let second_time = start2.elapsed();

    println!("缓存测试: 首次={}μs, 二次={}μs (加速比={:.1}x)",
             first_time.as_micros(), second_time.as_micros(),
             if first_time.as_micros() > 0 {
                 first_time.as_micros() as f64 / second_time.as_micros().max(1) as f64
             } else {
                 1.0
             });
}

// ========== 并发安全测试 ==========

#[tokio::test]
async fn concurrent_same_token_usage() {
    let mut ctx = TestContext::new();
    let token = ctx.login("test_user_001", "password123").await.expect("登录失败");

    // 100个并发请求使用同一token
    let handles: Vec<_> = (0..100)
        .map(|_| {
            let client = ctx.client.clone();
            let token = token.clone();
            tokio::spawn(async move {
                let resp = client
                    .get(format!("{}/api/user/balance", BASE_URL))
                    .header("Authorization", format!("Bearer {}", token))
                    .send()
                    .await;

                match resp {
                    Ok(r) => {
                        let status = r.status();
                        let body = r.text().await.ok();
                        let balance = body.and_then(|b| {
                            serde_json::from_str::<serde_json::Value>(&b).ok()
                                .and_then(|v| v["balance"].as_i64())
                        });
                        (status.is_success(), balance)
                    }
                    Err(_) => (false, None),
                }
            })
        })
        .collect();

    let results: Vec<_> = futures_util::future::join_all(handles).await;

    // 所有请求应该成功
    let success_count = results.iter().filter(|r| r.as_ref().map(|(s, _)| *s).unwrap_or(false)).count();
    assert!(success_count > 95, "至少95%的并发请求应该成功");

    // 验证数据一致性
    let balances: Vec<i64> = results.iter()
        .filter_map(|r| r.as_ref().ok().and_then(|(_, b)| *b))
        .collect();

    if !balances.is_empty() {
        let first_balance = balances[0];
        for balance in &balances {
            assert_eq!(*balance, first_balance, "并发请求应返回相同余额");
        }
    }

    println!("并发安全测试: 成功={}/100", success_count);
}
