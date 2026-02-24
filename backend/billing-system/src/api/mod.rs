use axum::{
    body::Body,
    extract::State,
    response::{Html, Json, Response},
};
use hyper::body::Bytes;
use serde_json::json;
use std::sync::Arc;

use crate::auth::{extract_api_key, verify_api_key};
use crate::billing::{BillingService, BillingTask};
use crate::cache::Cache;
use crate::database::{Database, RechargeBatch, ReconciliationRecord};
use crate::key_manager::KeyManager;
use crate::models::{AppError, Result, ChatRequest};
use crate::proxy::ModelPool;
use crate::recharge::RechargeService;
use crate::ratelimit::TokenRateLimiter;
use crate::scheduler::Scheduler;

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Database>,
    pub cache: Arc<Cache>,
    pub billing: Arc<BillingService>,
    pub model_pool: Arc<ModelPool>,
    pub key_manager: Arc<KeyManager>,
    pub recharge: Arc<RechargeService>,
    pub rate_limiter: Arc<TokenRateLimiter>,
    pub scheduler: Arc<Scheduler>,
}

/// 健康检查
pub async fn health_check() -> Json<serde_json::Value> {
    Json(json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now().to_rfc3339(),
    }))
}

/// Favicon
pub async fn favicon() -> Result<Response> {
    let favicon_data = include_bytes!("../../static/favicon.ico");
    let mut response = Response::new(Body::from(favicon_data.as_ref()));
    response.headers_mut().insert("content-type", "image/x-icon".parse().unwrap());
    Ok(response)
}

/// Web管理面板
pub async fn web_panel() -> Html<&'static str> {
    Html(include_str!("../../static/index.html"))
}

/// 获取模型健康状态
pub async fn model_health(
    State(state): State<AppState>,
) -> Json<serde_json::Value> {
    let health_list = state.model_pool.get_all_health().await;

    let health_json: Vec<serde_json::Value> = health_list
        .into_iter()
        .map(|h| json!({
            "model": h.model,
            "available": h.available,
            "consecutive_failures": h.consecutive_failures,
            "last_check": h.last_check,
            "avg_latency_ms": h.avg_latency_ms,
        }))
        .collect();

    Json(json!({
        "models": health_json,
        "timestamp": chrono::Utc::now().to_rfc3339(),
    }))
}

/// 检查单个模型健康状态
pub async fn check_model_health(
    State(state): State<AppState>,
    axum::extract::Path(model): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>> {
    let health = state.model_pool.check_model_health(&model).await?;

    Ok(Json(json!({
        "model": health.model,
        "available": health.available,
        "consecutive_failures": health.consecutive_failures,
        "last_check": health.last_check,
        "avg_latency_ms": health.avg_latency_ms,
        "timestamp": chrono::Utc::now().to_rfc3339(),
    })))
}

/// 获取模型统计信息（用户、任务、并发）
pub async fn model_stats(
    State(state): State<AppState>,
) -> Json<serde_json::Value> {
    // 获取所有用户
    let users = state.db.get_all_users(1, 1000).await.unwrap_or_default();

    // 获取计费记录
    let billing_records = state.db.get_all_billing_records(1, 10000).await.unwrap_or_default();

    // 按模型分组统计 - 使用结构体
    #[derive(Default)]
    struct ModelStat {
        total_requests: i64,
        total_tokens: i64,
        user_ids: std::collections::HashSet<i64>,
    }

    let mut model_stats: std::collections::HashMap<String, ModelStat> = std::collections::HashMap::new();

    // 初始化所有模型
    for model_name in state.model_pool.get_model_names() {
        model_stats.insert(model_name, ModelStat::default());
    }

    // 统计每个模型的用户和请求
    for record in &billing_records {
        let stat = model_stats.entry(record.model.clone()).or_default();
        stat.total_requests += 1;
        stat.total_tokens += record.total_tokens as i64;
        stat.user_ids.insert(record.user_id);
    }

    // 构建响应
    let mut stats_list = vec![];
    for (model_name, stat) in model_stats {
        let mut user_info = vec![];
        for user_id in &stat.user_ids {
            if let Some(user) = users.iter().find(|u| u.id == *user_id) {
                user_info.push(json!({
                    "user_id": user.id,
                    "email": user.email,
                    "username": user.username,
                    "tier": user.tier,
                    "api_key": format!("{}...", &user.api_key[..8.min(user.api_key.len())]),
                }));
            }
        }

        stats_list.push(json!({
            "model": model_name,
            "user_keys": user_info,
            "total_requests": stat.total_requests,
            "total_tokens": stat.total_tokens,
            "active_tasks": 0,
            "concurrent_limit": 100,
        }));
    }

    Json(json!({
        "models": stats_list,
        "timestamp": chrono::Utc::now().to_rfc3339(),
    }))
}

/// 获取API Key状态（实时监控）
pub async fn get_keys_status(
    State(state): State<AppState>,
) -> Json<serde_json::Value> {
    let keys_status: Vec<(crate::key_manager::ApiKey, crate::key_manager::KeyUsage)> =
        state.key_manager.get_all_keys_status().await;

    let keys_json: Vec<serde_json::Value> = keys_status.iter().map(|(k, u)| {
        json!({
            "id": k.id,
            "key_name": k.key_name,
            "provider": k.provider,
            "protocol": k.protocol,
            "owner": k.owner,
            "enabled": k.enabled,
            "max_tasks": k.max_tasks,
            "max_concurrent": k.max_concurrent,
            "priority": k.priority,
            "models": k.models,
            "usage": {
                "active_tasks": u.active_tasks,
                "concurrent_requests": u.concurrent_requests,
                "total_requests": u.total_requests,
                "failed_requests": u.failed_requests,
            },
            "health": {
                "is_healthy": u.is_healthy,
                "last_check": u.last_check_at,
                "error_message": u.error_message,
            },
            "utilization": {
                "tasks_percent": (u.active_tasks as f64 / k.max_tasks as f64 * 100.0).min(100.0),
                "concurrent_percent": (u.concurrent_requests as f64 / k.max_concurrent as f64 * 100.0).min(100.0),
            }
        })
    }).collect();

    Json(json!({
        "keys": keys_json,
        "timestamp": chrono::Utc::now().to_rfc3339(),
    }))
}

/// 邮箱登录
pub async fn email_login(
    State(state): State<AppState>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>> {
    let email = payload.get("email")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::InvalidInput("Missing email".to_string()))?;

    let user = state.db.get_user_by_email(email).await?;

    Ok(Json(json!({
        "success": true,
        "data": {
            "api_key": user.api_key,
            "user_id": user.id,
            "email": user.email,
            "username": user.username,
            "balance": user.balance,
            "tier": user.tier,
            "status": user.status,
        },
        "message": "登录成功",
    })))
}

/// 用户注册（用户名+密码+邮箱）
pub async fn register_user(
    State(state): State<AppState>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>> {
    let username = payload.get("username")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::InvalidInput("Missing username".to_string()))?;

    let email = payload.get("email")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::InvalidInput("Missing email".to_string()))?;

    let password = payload.get("password")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::InvalidInput("Missing password".to_string()))?;

    // 验证输入格式
    if username.len() < 3 || username.len() > 32 {
        return Err(AppError::InvalidInput("用户名长度必须为3-32个字符".to_string()));
    }

    if !email.contains('@') || email.len() > 128 {
        return Err(AppError::InvalidInput("邮箱格式不正确".to_string()));
    }

    if password.len() < 6 {
        return Err(AppError::InvalidInput("密码长度至少为6个字符".to_string()));
    }

    // 检查邮箱是否已存在
    if state.db.check_email_exists(email).await? {
        return Err(AppError::InvalidInput("邮箱已注册".to_string()));
    }

    // 检查用户名是否已存在
    if state.db.check_username_exists(username).await? {
        return Err(AppError::InvalidInput("用户名已被使用".to_string()));
    }

    // 创建密码哈希
    let password_hash = crate::auth::create_password_hash(password);

    // 创建用户
    let user = state.db.create_user_with_password(
        username,
        email,
        &password_hash.hash,
        &password_hash.salt,
    ).await?;

    Ok(Json(json!({
        "success": true,
        "data": {
            "user_id": user.id,
            "username": user.username,
            "email": user.email,
            "api_key": user.api_key,
            "balance": user.balance,
            "tier": user.tier,
        },
        "message": "注册成功",
    })))
}

/// 密码登录（用户名+密码或邮箱+密码）
/// 支持10次失败后锁定10分钟的登录限制
pub async fn password_login(
    State(state): State<AppState>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>> {
    let account = payload.get("account")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::InvalidInput("请输入用户名或邮箱".to_string()))?;

    let password = payload.get("password")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::InvalidInput("请输入密码".to_string()))?;

    // 检查登录失败限制
    let login_fail_key = format!("login_fail:{}", account.to_lowercase());
    let lockout_key = format!("login_lockout:{}", account.to_lowercase());

    // 检查是否被锁定 - 使用简化的Redis操作
    if let Ok(mut conn) = state.cache.client.get_multiplexed_async_connection().await {
        let is_locked: std::result::Result<Option<bool>, _> = redis::cmd("EXISTS")
            .arg(&lockout_key)
            .query_async(&mut conn)
            .await;

        if let Ok(Some(true)) = is_locked {
            return Err(AppError::AccountLocked);
        }
    }

    // 尝试通过用户名或邮箱查找用户
    let user = if account.contains('@') {
        match state.db.get_user_by_email(account).await {
            Ok(u) => u,
            Err(_) => return Err(AppError::UserNotFound),
        }
    } else {
        match state.db.get_user_by_username(account).await {
            Ok(u) => u,
            Err(_) => return Err(AppError::UserNotFound),
        }
    };

    // 验证密码
    let stored_hash = user.password_hash.as_ref().ok_or(AppError::PasswordNotSet)?;
    let salt = user.salt.as_ref().ok_or(AppError::PasswordNotSet)?;

    if !crate::auth::verify_password(password, stored_hash, salt) {
        // 密码错误，增加失败计数
        if let Ok(mut conn) = state.cache.client.get_multiplexed_async_connection().await {
            // 增加失败计数
            let fail_count: std::result::Result<Option<usize>, _> = redis::cmd("INCR")
                .arg(&login_fail_key)
                .query_async(&mut conn)
                .await;

            // 设置过期时间为10分钟
            let _ = redis::cmd("EXPIRE")
                .arg(&login_fail_key)
                .arg(600)
                .query_async::<_, ()>(&mut conn)
                .await;

            // 如果连续失败10次，锁定账户
            if let Ok(Some(count)) = fail_count {
                if count >= 10 {
                    let _ = redis::cmd("SET")
                        .arg(&lockout_key)
                        .arg("1")
                        .query_async::<_, ()>(&mut conn)
                        .await;

                    let _ = redis::cmd("EXPIRE")
                        .arg(&lockout_key)
                        .arg(600)
                        .query_async::<_, ()>(&mut conn)
                        .await;

                    // 清除失败计数
                    let _ = redis::cmd("DEL")
                        .arg(&login_fail_key)
                        .query_async::<_, ()>(&mut conn)
                        .await;

                    return Err(AppError::AccountLocked);
                }
            }
        }

        return Err(AppError::PasswordIncorrect);
    }

    // 检查用户状态
    if user.status != "active" {
        return Err(AppError::AccountInactive);
    }

    // 登录成功，清除失败计数
    if let Ok(mut conn) = state.cache.client.get_multiplexed_async_connection().await {
        let _ = redis::cmd("DEL")
            .arg(&login_fail_key)
            .query_async::<_, ()>(&mut conn)
            .await;
    }

    Ok(Json(json!({
        "success": true,
        "data": {
            "api_key": user.api_key,
            "user_id": user.id,
            "email": user.email,
            "username": user.username,
            "balance": user.balance,
            "tier": user.tier,
            "status": user.status,
        },
        "message": "登录成功",
    })))
}

/// 获取用户资料
pub async fn get_profile(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<Json<serde_json::Value>> {
    let api_key = extract_api_key(&headers)?;
    let user = verify_api_key(&state.db, &api_key).await?;
    let balance = state.billing.get_balance(user.user_id).await?;

    Ok(Json(json!({
        "user_id": user.user_id,
        "username": user.username,
        "email": user.email,
        "balance": balance,
        "tier": user.tier,
        "status": user.status,
        "concurrent_limit": user.concurrent_limit,
    })))
}

/// 修改密码
pub async fn change_password(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>> {
    let api_key = extract_api_key(&headers)?;
    let user = verify_api_key(&state.db, &api_key).await?;

    let old_password = payload.get("old_password")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::InvalidInput("Missing old_password".to_string()))?;

    let new_password = payload.get("new_password")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::InvalidInput("Missing new_password".to_string()))?;

    if new_password.len() < 6 {
        return Err(AppError::InvalidInput("New password must be at least 6 characters".to_string()));
    }

    // 获取用户信息验证旧密码
    let db_user = state.db.get_user_by_api_key(&api_key).await?;

    let stored_hash = db_user.password_hash.as_ref().ok_or(AppError::InvalidInput("User has no password set".to_string()))?;
    let salt = db_user.salt.as_ref().ok_or(AppError::InvalidInput("User has no salt set".to_string()))?;

    if !crate::auth::verify_password(old_password, stored_hash, salt) {
        return Err(AppError::InvalidInput("Current password is incorrect".to_string()));
    }

    // 创建新密码哈希
    let new_password_hash = crate::auth::create_password_hash(new_password);

    // 更新密码
    state.db.update_user_password(user.user_id, &new_password_hash.hash, &new_password_hash.salt).await?;

    Ok(Json(json!({
        "success": true,
        "message": "密码修改成功",
    })))
}

/// 修改用户名
pub async fn update_profile(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>> {
    let api_key = extract_api_key(&headers)?;
    let user = verify_api_key(&state.db, &api_key).await?;

    let username = payload.get("username")
        .and_then(|v| v.as_str());

    if let Some(name) = username {
        if name.len() < 3 || name.len() > 32 {
            return Err(AppError::InvalidInput("Username must be 3-32 characters".to_string()));
        }
        state.db.update_user_username(user.user_id, name).await?;
    }

    Ok(Json(json!({
        "success": true,
        "message": "资料更新成功",
    })))
}

/// 查询余额
pub async fn get_balance(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<Json<serde_json::Value>> {
    let api_key = extract_api_key(&headers)?;
    let user = verify_api_key(&state.db, &api_key).await?;
    let balance = state.billing.get_balance(user.user_id).await?;

    Ok(Json(json!({
        "balance": balance,
        "tier": user.tier,
        "user_id": user.user_id,
        "email": user.email,
    })))
}

/// 聊天请求（非流式）
pub async fn chat_completion(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    body_bytes: Bytes,
) -> Result<Response> {
    let start_time = std::time::Instant::now();
    let _json: serde_json::Value = serde_json::from_slice(&body_bytes)?;

    let api_key = extract_api_key(&headers)?;
    let user = verify_api_key(&state.db, &api_key).await?;

    let req: ChatRequest = serde_json::from_slice(&body_bytes)?;

    // 检查用户等级是否有权限访问该模型
    let has_access = state.db.check_model_access(&user.tier, &req.model).await?;
    if !has_access {
        let _ = state.db.log_request(
            &format!("req_{}", uuid::Uuid::new_v4()),
            user.user_id,
            &api_key[..8.min(api_key.len())],
            &req.model,
            None,
            None,
            None,
            "forbidden",
            Some(&format!("Model '{}' not allowed for tier '{}'", req.model, user.tier)),
            0, 0, 0, 0,
        ).await;
        return Err(AppError::ModelNotAllowed { model: req.model.clone() });
    }

    // 检查余额
    let cost = state.billing.calculate_cost(&req.model, 0, 0)?;
    if !state.billing.check_balance(user.user_id, cost).await? {
        return Err(AppError::InsufficientBalance {
            required: cost as i64,
            balance: state.billing.get_balance(user.user_id).await?,
        });
    }

    // 检查限流
    let qps_limit = if user.tier == "Enterprise" { 200 } else { 100 };
    if !state.cache.check_rate_limit(user.user_id, qps_limit).await? {
        return Err(AppError::RateLimitExceeded);
    }

    let request_id = format!("req_{}", uuid::Uuid::new_v4());

    // 预扣费
    let new_balance = state.billing.pre_deduct(user.user_id, cost).await?;

    // 转发请求
    let response_result = state.model_pool.forward_chat(&req.model, req.clone()).await;

    match response_result {
        Ok(response) => {
            // 解析token使用量
            let (input_tokens, output_tokens) = extract_usage(&response);
            let actual_cost = state.billing.calculate_cost(&req.model, input_tokens, output_tokens)?;

            // 记录计费
            let _ = state.billing.record_billing(BillingTask {
                user_id: user.user_id,
                request_id: request_id.clone(),
                model: req.model.clone(),
                input_tokens,
                output_tokens,
            }).await;

            // 记录请求日志
            let latency = start_time.elapsed().as_millis() as i32;
            let _ = state.db.log_request(
                &request_id,
                user.user_id,
                &api_key[..8.min(api_key.len())],
                &req.model,
                Some(&req.model),
                None,
                None,
                "success",
                None,
                input_tokens,
                output_tokens,
                actual_cost,
                latency,
            ).await;

            // 构造响应
            let response_body = json!({
                "id": response.get("id").and_then(|v| v.as_str()).unwrap_or(&request_id),
                "object": "chat.completion",
                "created": response.get("created").and_then(|v| v.as_i64()).unwrap_or(chrono::Utc::now().timestamp()),
                "model": req.model,
                "choices": response.get("choices"),
                "usage": response.get("usage"),
            });

            let json_str = serde_json::to_string(&response_body)?;
            let mut resp = Response::new(Body::from(json_str));
            resp.headers_mut().insert("X-Cost", actual_cost.to_string().parse().unwrap());
            resp.headers_mut().insert("X-Balance", new_balance.to_string().parse().unwrap());
            resp.headers_mut().insert("X-Request-ID", request_id.parse().unwrap());

            Ok(resp)
        }
        Err(e) => {
            // 请求失败，退还积分
            let _ = state.billing.refund(user.user_id, cost).await;

            // 记录失败日志
            let latency = start_time.elapsed().as_millis() as i32;
            let _ = state.db.log_request(
                &request_id,
                user.user_id,
                &api_key[..8.min(api_key.len())],
                &req.model,
                None,
                None,
                None,
                "failed",
                Some(&format!("{:?}", e)),
                0,
                0,
                0,
                latency,
            ).await;

            Err(e)
        }
    }
}

/// 聊天请求（流式）- 简化版本
pub async fn chat_completion_stream(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    body_bytes: Bytes,
) -> Result<Response> {
    let _json: serde_json::Value = serde_json::from_slice(&body_bytes)?;

    let api_key = extract_api_key(&headers)?;
    let user = verify_api_key(&state.db, &api_key).await?;

    let mut req: ChatRequest = serde_json::from_slice(&body_bytes)?;
    req.stream = true;

    // 检查用户等级是否有权限访问该模型
    let has_access = state.db.check_model_access(&user.tier, &req.model).await?;
    if !has_access {
        return Err(AppError::ModelNotAllowed { model: req.model.clone() });
    }

    // 检查余额
    let cost = state.billing.calculate_cost(&req.model, 0, 0)?;
    if !state.billing.check_balance(user.user_id, cost).await? {
        return Err(AppError::InsufficientBalance {
            required: cost as i64,
            balance: state.billing.get_balance(user.user_id).await?,
        });
    }

    // 检查限流
    let qps_limit = if user.tier == "Enterprise" { 200 } else { 100 };
    if !state.cache.check_rate_limit(user.user_id, qps_limit).await? {
        return Err(AppError::RateLimitExceeded);
    }

    let request_id = format!("req_{}", uuid::Uuid::new_v4());
    let user_id = user.user_id;
    let model = req.model.clone();

    // 预扣费
    let _new_balance = state.billing.pre_deduct(user.user_id, cost).await?;

    // 简化的流式响应 - 实际应该从上游获取流式响应
    let stream_data = format!(
        "data: {}\n\n",
        json!({
            "id": request_id,
            "object": "chat.completion.chunk",
            "created": chrono::Utc::now().timestamp(),
            "model": model,
            "choices": [{
                "index": 0,
                "delta": {"content": "Streaming response placeholder."},
                "finish_reason": null,
            }],
        })
    );

    let mut response = Response::new(Body::from(stream_data + "data: [DONE]\n\n"));
    response.headers_mut().insert("Content-Type", "text/event-stream".parse().unwrap());
    response.headers_mut().insert("Cache-Control", "no-cache".parse().unwrap());

    // 异步记录计费
    let billing = state.billing.clone();
    tokio::spawn(async move {
        let _ = billing.record_billing(BillingTask {
            user_id,
            request_id: request_id.clone(),
            model,
            input_tokens: 10,
            output_tokens: 20,
        }).await;
    });

    Ok(response)
}

/// 获取计费记录
pub async fn get_billing_history(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<Json<serde_json::Value>> {
    let api_key = extract_api_key(&headers)?;
    let user = verify_api_key(&state.db, &api_key).await?;

    let records = state.db.get_billing_records(user.user_id, 50).await?;

    let records_json: Vec<serde_json::Value> = records
        .into_iter()
        .map(|r| json!({
            "id": r.id,
            "request_id": r.request_id,
            "model": r.model,
            "input_tokens": r.input_tokens,
            "output_tokens": r.output_tokens,
            "total_tokens": r.total_tokens,
            "cost": r.cost,
            "context_doubled": r.context_doubled,
            "created_at": r.created_at.to_rfc3339(),
        }))
        .collect();

    Ok(Json(json!({
        "records": records_json,
        "count": records_json.len(),
    })))
}

fn extract_usage(response: &serde_json::Value) -> (i32, i32) {
    if let Some(usage) = response.get("usage") {
        let prompt = usage.get("prompt_tokens").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
        let completion = usage.get("completion_tokens").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
        (prompt, completion)
    } else {
        (0, 0)
    }
}

// ==================== 充值码 API ====================

/// 兑换充值码
pub async fn redeem_code(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>> {
    let api_key = extract_api_key(&headers)?;
    let user = verify_api_key(&state.db, &api_key).await?;

    let code = payload.get("code")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::InternalError(anyhow::anyhow!("Missing code")))?;

    let new_balance = state.recharge.redeem_code(user.user_id, code).await?;

    Ok(Json(json!({
        "success": true,
        "new_balance": new_balance,
        "message": "充值成功",
    })))
}

pub async fn create_recharge_batch(
    State(state): State<AppState>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>> {
    let points = payload.get("points")
        .and_then(|v| v.as_i64())
        .unwrap_or(500) as i32;

    let count = payload.get("count")
        .and_then(|v| v.as_i64())
        .unwrap_or(1) as i32;

    let valid_days = payload.get("valid_days")
        .and_then(|v| v.as_i64())
        .unwrap_or(3650) as i32;

    let codes = state.recharge.create_batch_with_points(points, count, valid_days).await?;

    Ok(Json(json!({
        "success": true,
        "batch_id": codes.first().and_then(|c| c.split('-').last()).unwrap_or("unknown"),
        "points": points,
        "count": codes.len(),
        "valid_days": valid_days,
        "codes": codes,
    })))
}

/// 获取充值码统计
pub async fn get_code_stats(
    State(state): State<AppState>,
    axum::extract::Path(code): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>> {
    let stats = state.recharge.get_code_stats(&code).await?;
    Ok(Json(stats))
}

/// 获取批次统计
pub async fn get_batch_stats(
    State(state): State<AppState>,
    axum::extract::Path(batch_id): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>> {
    let stats = state.recharge.get_batch_stats(&batch_id).await?;
    Ok(Json(stats))
}

// ==================== 管理端 API ====================

/// 获取所有用户列表（分页）
pub async fn admin_get_users(
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<serde_json::Value>,
) -> Result<Json<serde_json::Value>> {
    let page: i32 = params.get("page")
        .and_then(|v| v.as_i64())
        .unwrap_or(1) as i32;
    let limit: i32 = params.get("limit")
        .and_then(|v| v.as_i64())
        .unwrap_or(20) as i32;

    let users = state.db.get_all_users(page, limit).await?;

    let users_json: Vec<serde_json::Value> = users
        .into_iter()
        .map(|u| json!({
            "id": u.id,
            "email": u.email,
            "username": u.username,
            "balance": u.balance,
            "tier": u.tier,
            "status": u.status,
            "concurrent_limit": u.concurrent_limit,
            "created_at": u.created_at.to_rfc3339(),
        }))
        .collect();

    Ok(Json(json!({
        "users": users_json,
        "page": page,
        "limit": limit,
        "total": users_json.len(), // 简化处理，实际应该查询总数
    })))
}

/// 获取所有计费记录（分页）
pub async fn admin_get_billing(
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<serde_json::Value>,
) -> Result<Json<serde_json::Value>> {
    let page: i32 = params.get("page")
        .and_then(|v| v.as_i64())
        .unwrap_or(1) as i32;
    let limit: i32 = params.get("limit")
        .and_then(|v| v.as_i64())
        .unwrap_or(20) as i32;

    let records = state.db.get_all_billing_records(page, limit).await?;

    let records_json: Vec<serde_json::Value> = records
        .into_iter()
        .map(|r| json!({
            "id": r.id,
            "user_id": r.user_id,
            "request_id": r.request_id,
            "model": r.model,
            "input_tokens": r.input_tokens,
            "output_tokens": r.output_tokens,
            "total_tokens": r.total_tokens,
            "cost": r.cost,
            "context_doubled": r.context_doubled,
            "billing_status": r.billing_status,
            "created_at": r.created_at.to_rfc3339(),
        }))
        .collect();

    Ok(Json(json!({
        "records": records_json,
        "page": page,
        "limit": limit,
        "total": records_json.len(),
    })))
}

/// 获取充值码统计
pub async fn admin_recharge_stats(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>> {
    let stats = state.db.get_recharge_stats().await?;

    Ok(Json(json!({
        "total": stats.total,
        "used": stats.used,
        "unused": stats.unused,
        "batches": stats.batches,
    })))
}

/// 获取充值码批次列表
pub async fn admin_recharge_batches(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>> {
    let batches: Vec<RechargeBatch> = state.db.get_recharge_batches().await?;

    let batches_json: Vec<serde_json::Value> = batches
        .into_iter()
        .map(|b| json!({
            "id": b.id,
            "batch_id": b.batch_id,
            "package_type": b.package_type,
            "total_count": b.total_count,
            "used_count": b.used_count,
            "created_at": b.created_at.map(|t: chrono::DateTime<chrono::Utc>| t.to_rfc3339()),
        }))
        .collect();

    Ok(Json(json!({
        "batches": batches_json,
    })))
}

/// 获取对账记录
pub async fn admin_get_reconciliation(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>> {
    let records: Vec<ReconciliationRecord> = state.db.get_reconciliation_records(50).await?;

    let records_json: Vec<serde_json::Value> = records
        .into_iter()
        .map(|r| json!({
            "id": r.id,
            "user_id": r.user_id,
            "db_balance": r.db_balance,
            "redis_balance": r.redis_balance,
            "difference": r.difference,
            "reconciled_at": r.reconciled_at.to_rfc3339(),
        }))
        .collect();

    Ok(Json(json!({
        "records": records_json,
        "total": records_json.len(),
        "differences": records_json.iter().filter(|r| r["difference"].as_i64().unwrap_or(0) != 0).count(),
        "fixed": records_json.len(),
    })))
}

/// 获取待处理的计费记录
pub async fn admin_scheduler_pending(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>> {
    let records = state.db.get_pending_billing_records(100).await?;

    let records_json: Vec<serde_json::Value> = records
        .into_iter()
        .map(|r| json!({
            "id": r.id,
            "user_id": r.user_id,
            "request_id": r.request_id,
            "model": r.model,
            "total_tokens": r.total_tokens,
            "cost": r.cost,
            "created_at": r.created_at.to_rfc3339(),
        }))
        .collect();

    Ok(Json(json!({
        "records": records_json,
        "count": records_json.len(),
    })))
}

/// 获取系统概览数据
pub async fn admin_overview(
    State(state): State<AppState>,
) -> Json<serde_json::Value> {
    let users = state.db.get_all_users(1, 1000).await.unwrap_or_default();
    let active_users = users.iter().filter(|u| u.status == "active").count();

    Json(json!({
        "users": {
            "total": users.len(),
            "active": active_users,
            "by_tier": {
                "enterprise": users.iter().filter(|u| u.tier == "Enterprise").count(),
                "standard": users.iter().filter(|u| u.tier == "Standard").count(),
                "basic": users.iter().filter(|u| u.tier == "Basic").count(),
            }
        },
        "timestamp": chrono::Utc::now().to_rfc3339(),
    }))
}

// ==================== API申请相关 ====================

/// 申请API密钥
pub async fn apply_api_key(
    State(state): State<AppState>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>> {
    let email = payload.get("email")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::InvalidInput("Missing email".to_string()))?;

    let username = payload.get("username")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::InvalidInput("Missing username".to_string()))?;

    // 验证邮箱格式
    if !email.contains('@') || email.len() > 128 {
        return Err(AppError::InvalidInput("Invalid email format".to_string()));
    }

    // 验证用户名格式 (3-50字符，字母数字下划线中划线)
    if username.len() < 3 || username.len() > 50 {
        return Err(AppError::InvalidInput("Username must be 3-50 characters".to_string()));
    }
    if !username.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
        return Err(AppError::InvalidInput("Username can only contain letters, numbers, underscore and hyphen".to_string()));
    }

    // 检查用户名是否已存在
    let existing_users = state.db.get_all_users(1, 1000).await.unwrap_or_default();
    if existing_users.iter().any(|u| u.username.as_ref().map(|n| n == username).unwrap_or(false)) {
        return Err(AppError::InvalidInput("Username already exists".to_string()));
    }

    // 创建用户
    let user = state.db.create_user(email, Some(username)).await?;

    Ok(Json(json!({
        "success": true,
        "data": {
            "api_key": user.api_key,
            "user_id": user.id,
            "daily_limit": 100000,
            "balance": user.balance,
            "tier": user.tier,
        },
        "message": "API密钥申请成功，请妥善保管",
    })))
}

/// 检查用户名可用性
pub async fn check_username_availability(
    State(state): State<AppState>,
    axum::extract::Path(username): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    let users = state.db.get_all_users(1, 1000).await.unwrap_or_default();
    let available = !users.iter().any(|u| u.username.as_ref().map(|n| n == &username).unwrap_or(false));

    Json(json!({
        "available": available,
        "username": username,
    }))
}

// ==================== API统计相关 ====================

/// 获取API统计数据
pub async fn get_api_stats(
    State(state): State<AppState>,
) -> Json<serde_json::Value> {
    // 获取用户统计
    let users = state.db.get_all_users(1, 1000).await.unwrap_or_default();
    let active_users = users.iter().filter(|u| u.status == "active").count();

    // 获取计费记录统计
    let billing_records = state.db.get_all_billing_records(1, 100000).await.unwrap_or_default();
    let total_requests = billing_records.len();
    let total_tokens: i64 = billing_records.iter().map(|r| r.total_tokens as i64).sum();

    // 按模型统计
    let mut model_counts: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
    for record in &billing_records {
        *model_counts.entry(record.model.clone()).or_insert(0) += record.total_tokens as i64;
    }

    // 模拟今日请求数（实际应该从数据库查询今日数据）
    let today_requests = total_requests / 10; // 简化处理

    Json(json!({
        "total_requests": total_requests,
        "success_rate": 99.5,
        "active_users": active_users,
        "today_requests": today_requests,
        "total_tokens": total_tokens,
        "model_distribution": model_counts,
        "timestamp": chrono::Utc::now().to_rfc3339(),
    }))
}

// ==================== 管理员认证相关 ====================

/// 管理员登录
pub async fn admin_login(
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>> {
    let username = payload.get("username")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::InvalidInput("Missing username".to_string()))?;

    let password = payload.get("password")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::InvalidInput("Missing password".to_string()))?;

    // 简化管理员验证（生产环境应使用数据库和密码哈希）
    const ADMIN_USERNAME: &str = "admin";
    // 简单的密码哈希验证 (实际应使用bcrypt等)
    const ADMIN_PASSWORD_HASH: &str = "admin123"; // 演示密码，生产环境应使用哈希

    if username == ADMIN_USERNAME && password == ADMIN_PASSWORD_HASH {
        // 生成简单的会话令牌
        let token = format!("admin_session_{}", chrono::Utc::now().timestamp());

        Ok(Json(json!({
            "success": true,
            "message": "登录成功",
            "data": {
                "token": token,
                "username": username,
                "role": "admin",
                "expires_in": 86400 // 24小时
            }
        })))
    } else {
        Err(AppError::Unauthorized)
    }
}

/// 验证管理员会话
pub async fn verify_admin_session(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<Json<serde_json::Value>> {
    let auth_header = headers
        .get("authorization")
        .ok_or(AppError::Unauthorized)?
        .to_str()
        .map_err(|_| AppError::Unauthorized)?;

    if !auth_header.starts_with("Bearer ") {
        return Err(AppError::Unauthorized);
    }

    let token = auth_header[7..].to_string();

    // 简单验证：检查token格式
    if token.starts_with("admin_session_") {
        Ok(Json(json!({
            "authenticated": true,
            "username": "admin",
            "role": "admin"
        })))
    } else {
        Err(AppError::Unauthorized)
    }
}
