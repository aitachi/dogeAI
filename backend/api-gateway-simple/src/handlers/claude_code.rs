// ========== Claude Code 专用端点处理器 ==========
// 根据 pyDogeAI 分支的端点实现，兼容 Claude Code 客户端

use axum::{
    extract::{Request, Host},
    http::{HeaderName, HeaderValue, StatusCode},
    response::{Html, IntoResponse, Json, Response},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

// ========== 响应数据结构 ==========

/// Bootstrap 响应 - 平台引导信息
#[derive(Debug, Serialize)]
pub struct BootstrapResponse {
    pub account: UserAccountInfo,
    pub organizations: Vec<OrganizationInfo>,
    pub statsig: serde_json::Value,
}

/// Auth 响应 - 认证信息
#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub account: UserAccountInfo,
    pub account_flags: Vec<String>,
}

/// Auth Session 响应 - 会话信息
#[derive(Debug, Serialize)]
pub struct AuthSessionResponse {
    pub account: UserAccountInfo,
    pub session: SessionInfo,
    pub account_flags: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct SessionInfo {
    pub id: String,
    pub expires_at: String,
}

/// Claude Code Settings 响应
#[derive(Debug, Serialize)]
pub struct ClaudeCodeSettingsResponse {
    pub expiry: Option<String>,
    pub isolated: bool,
    pub allowed_tools: Vec<String>,
    pub max_turns: Option<u32>,
    pub internet_policy: String,
}

/// Claude Code Policy Limits 响应
#[derive(Debug, Serialize)]
pub struct ClaudeCodePolicyLimitsResponse {
    pub rate_limits: RateLimits,
    pub usage: UsageStats,
    pub limits: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct RateLimits {
    pub requests_per_minute: u32,
    pub tokens_per_minute: u64,
    pub tokens_per_day: u64,
}

#[derive(Debug, Serialize)]
pub struct UsageStats {
    pub tokens_used_today: u64,
    pub requests_today: u32,
}

/// Penguin Mode 响应
#[derive(Debug, Serialize)]
pub struct PenguinModeResponse {
    pub enabled: bool,
}

/// 用户账户信息
#[derive(Debug, Serialize, Clone)]
pub struct UserAccountInfo {
    #[serde(rename = "uuid")]
    pub id: String,
    #[serde(rename = "id")]
    pub account_id: String,
    #[serde(rename = "type")]
    pub user_type: String,
    pub email: String,
    #[serde(rename = "email_address")]
    pub email_address: String,
    pub name: String,
    #[serde(rename = "full_name")]
    pub full_name: String,
    #[serde(rename = "display_name")]
    pub display_name: String,
    #[serde(rename = "created_at")]
    pub created_at: i64,
    #[serde(rename = "chat_enabled")]
    pub chat_enabled: bool,
    pub memberships: Vec<MembershipInfo>,
}

#[derive(Debug, Serialize, Clone)]
pub struct MembershipInfo {
    pub organization: OrganizationInfo,
    pub role: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct OrganizationInfo {
    pub id: String,
    #[serde(rename = "uuid")]
    pub org_uuid: String,
    #[serde(rename = "type")]
    pub org_type: String,
    pub name: String,
    #[serde(rename = "created_at")]
    pub created_at: i64,
    pub settings: OrganizationSettings,
    pub capabilities: Vec<String>,
    #[serde(rename = "api_disabled_reason")]
    pub api_disabled_reason: Option<String>,
    #[serde(rename = "active_flags")]
    pub active_flags: Vec<String>,
    #[serde(rename = "billing_status")]
    pub billing_status: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct OrganizationSettings {
    #[serde(rename = "tier")]
    pub tier: String,
    #[serde(rename = "claude_console_enabled")]
    pub claude_console_enabled: bool,
}

/// OAuth Token 请求
#[derive(Debug, Deserialize)]
pub struct OAuthTokenRequest {
    pub grant_type: Option<String>,
    pub code: Option<String>,
    pub redirect_uri: Option<String>,
}

/// OAuth Token 响应
#[derive(Debug, Serialize)]
pub struct OAuthTokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
    pub refresh_token: String,
    pub scope: String,
    #[serde(rename = "account_uuid")]
    pub account_uuid: String,
}

/// OpenID 配置
#[derive(Debug, Serialize)]
pub struct OpenIDConfiguration {
    pub issuer: String,
    pub authorization_endpoint: String,
    pub token_endpoint: String,
    pub userinfo_endpoint: String,
    pub response_types_supported: Vec<String>,
    pub grant_types_supported: Vec<String>,
    pub code_challenge_methods_supported: Vec<String>,
    pub token_endpoint_auth_methods_supported: Vec<String>,
}

/// OAuth Authorization Server 元数据
#[derive(Debug, Serialize)]
pub struct OAuthAuthorizationServer {
    pub issuer: String,
    pub authorization_endpoint: String,
    pub token_endpoint: String,
    pub response_types_supported: Vec<String>,
    pub grant_types_supported: Vec<String>,
    pub code_challenge_methods_supported: Vec<String>,
}

// ========== 辅助函数 ==========

/// 生成假的账户 UUID
fn fake_account_uuid() -> String {
    std::env::var("FAKE_ACCOUNT_UUID")
        .unwrap_or_else(|_| "3c90a1a6-8e4a-4d7a-9e5f-1a2b3c4d5e6f".to_string())
}

/// 生成假的邮箱
fn fake_email() -> String {
    std::env::var("FAKE_EMAIL")
        .unwrap_or_else(|_| "user@example.com".to_string())
}

/// 生成假的用户名
fn fake_name() -> String {
    std::env::var("FAKE_NAME")
        .unwrap_or_else(|_| "Claude User".to_string())
}

/// 生成假的组织 ID
fn fake_org_id() -> String {
    std::env::var("FAKE_ORG_ID")
        .unwrap_or_else(|_| "7d00b2b7-9f5b-4e8b-0a6f-2b3c4d5e6f7a".to_string())
}

/// 生成用户资料信息
fn user_profile() -> UserAccountInfo {
    let account_uuid = fake_account_uuid();
    let email = fake_email();
    let name = fake_name();

    UserAccountInfo {
        id: account_uuid.clone(),
        account_id: account_uuid.clone(),
        user_type: "user".to_string(),
        email: email.clone(),
        email_address: email,
        name: name.clone(),
        full_name: name.clone(),
        display_name: name,
        created_at: 1704067200, // 2024-01-01
        chat_enabled: true,
        memberships: vec![MembershipInfo {
            organization: org_info(),
            role: "owner".to_string(),
        }],
    }
}

/// 生成组织信息
fn org_info() -> OrganizationInfo {
    let org_id = fake_org_id();

    OrganizationInfo {
        id: org_id.clone(),
        org_uuid: org_id,
        org_type: "organization".to_string(),
        name: "Default Organization".to_string(),
        created_at: 1706745600, // 2024-02-01
        settings: OrganizationSettings {
            tier: "scale".to_string(),
            claude_console_enabled: true,
        },
        capabilities: vec!["api_access".to_string(), "model_access".to_string()],
        api_disabled_reason: None,
        active_flags: vec![],
        billing_status: "active".to_string(),
    }
}

/// 生成 OAuth 授权码
fn gen_oauth_code() -> String {
    use rand::Rng;
    let mut code = String::with_capacity(43);
    let charset = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut rng = rand::thread_rng();
    for _ in 0..43 {
        code.push(charset[rng.gen_range(0..charset.len())] as char);
    }
    code
}

/// 生成 OAuth Token
fn gen_oauth_token(prefix: &str) -> String {
    use rand::Rng;
    let uuid1 = Uuid::new_v4().to_string().replace("-", "");
    let uuid2 = Uuid::new_v4().to_string().replace("-", "");
    format!("{}-{}-{}", prefix, &uuid1[..16], &uuid2[..8])
}

/// 生成 Anthropic 风格的请求 ID
fn gen_request_id() -> String {
    format!("req_{}", Uuid::new_v4().to_string()[..24].replace("-", ""))
}

/// 生成 Anthropic 风格的响应头
fn make_anthropic_headers() -> [(String, String); 9] {
    let req_id = gen_request_id();
    [
        ("x-request-id".to_string(), req_id.clone()),
        ("request-id".to_string(), req_id.clone()),
        ("anthropic-version".to_string(), "2023-06-01".to_string()),
        ("x-anthropic-ratelimit-requests-limit".to_string(), "10000".to_string()),
        ("x-anthropic-ratelimit-requests-remaining".to_string(), "9999".to_string()),
        ("x-anthropic-ratelimit-requests-reset".to_string(), "2026-12-31T23:59:59Z".to_string()),
        ("x-anthropic-ratelimit-tokens-limit".to_string(), "1000000".to_string()),
        ("x-anthropic-ratelimit-tokens-remaining".to_string(), "999999".to_string()),
        ("x-anthropic-ratelimit-tokens-reset".to_string(), "2026-12-31T23:59:59Z".to_string()),
    ]
}

/// 构建包含 Anthropic 风格响应头的响应
fn with_anthropic_headers(body: Json<impl serde::Serialize>) -> Response {
    let headers = make_anthropic_headers();
    let mut response = body.into_response();
    for (key, value) in headers {
        // 使用 HeaderName/HeaderValue 避免生命周期问题
        if let Ok(header_name) = HeaderName::from_bytes(key.as_bytes()) {
            if let Ok(header_value) = HeaderValue::from_str(&value) {
                response.headers_mut().insert(header_name, header_value);
            }
        }
    }
    response
}

// ========== 端点处理器 ==========

/// GET /api/bootstrap - 平台引导信息
pub async fn bootstrap_handler() -> Response {
    tracing::info!("[Claude Code] GET /api/bootstrap");
    let response = BootstrapResponse {
        account: user_profile(),
        organizations: vec![org_info()],
        statsig: serde_json::json!({}),
    };
    with_anthropic_headers(Json(response))
}

/// GET /api/auth - 认证信息
pub async fn auth_handler() -> Response {
    tracing::info!("[Claude Code] GET /api/auth");
    let response = AuthResponse {
        account: user_profile(),
        account_flags: vec![],
    };
    with_anthropic_headers(Json(response))
}

/// GET /api/auth/session - 会话信息
pub async fn auth_session_handler() -> Response {
    tracing::info!("[Claude Code] GET /api/auth/session");

    // 生成会话 ID
    let session_id = format!("sess_{}", Uuid::new_v4().to_string().replace("-", ""));

    // 计算过期时间（30天后）
    let now = chrono::Utc::now();
    let expires_at = (now + chrono::Duration::days(30)).to_rfc3339() + "Z";

    let response = AuthSessionResponse {
        account: user_profile(),
        session: SessionInfo {
            id: session_id,
            expires_at,
        },
        account_flags: vec![],
    };
    with_anthropic_headers(Json(response))
}

/// GET /api/account - 账户信息
pub async fn account_handler() -> Response {
    tracing::info!("[Claude Code] GET /api/account");
    with_anthropic_headers(Json(user_profile()))
}

/// GET /api/me - 用户资料（与 account 相同）
pub async fn me_handler() -> Response {
    tracing::info!("[Claude Code] GET /api/me");
    with_anthropic_headers(Json(user_profile()))
}

/// GET /api/settings - 设置信息
pub async fn settings_handler() -> Response {
    tracing::info!("[Claude Code] GET /api/settings");
    with_anthropic_headers(Json(serde_json::json!({})))
}

/// GET /api/claude_code/settings - Claude Code 专用设置
pub async fn claude_code_settings_handler() -> Response {
    tracing::info!("[Claude Code] GET /api/claude_code/settings");
    let response = ClaudeCodeSettingsResponse {
        expiry: None,
        isolated: false,
        allowed_tools: vec!["computer".to_string(), "text_editor".to_string(), "bash".to_string()],
        max_turns: None,
        internet_policy: "allow".to_string(),
    };
    with_anthropic_headers(Json(response))
}

/// GET /api/claude_code/policy_limits - Claude Code 策略限制
pub async fn claude_code_policy_limits_handler() -> Response {
    tracing::info!("[Claude Code] GET /api/claude_code/policy_limits");
    let response = ClaudeCodePolicyLimitsResponse {
        rate_limits: RateLimits {
            requests_per_minute: 60,
            tokens_per_minute: 1000000,
            tokens_per_day: 50000000,
        },
        usage: UsageStats {
            tokens_used_today: 0,
            requests_today: 0,
        },
        limits: HashMap::new(),
    };
    with_anthropic_headers(Json(response))
}

/// GET /api/claude_code/penguin_mode - Penguin 模式
pub async fn penguin_mode_handler() -> Response {
    tracing::info!("[Claude Code] GET /api/claude_code/penguin_mode");
    let response = PenguinModeResponse {
        enabled: false,
    };
    with_anthropic_headers(Json(response))
}

/// GET /api/organizations - 组织列表
pub async fn organizations_handler() -> Response {
    tracing::info!("[Claude Code] GET /api/organizations");
    let response = serde_json::json!({
        "data": vec![org_info()]
    });
    with_anthropic_headers(Json(response))
}

/// GET /userinfo - OAuth 用户信息
pub async fn userinfo_handler() -> Response {
    tracing::info!("[Claude Code] GET /userinfo");
    with_anthropic_headers(Json(user_profile()))
}

/// POST /api/organizations/{org_id}/api_keys - 创建组织 API Key（带下划线）
pub async fn create_org_api_key_handler(
    axum::extract::Path(org_id): axum::extract::Path<String>,
    _req: Request,
) -> Response {
    tracing::info!("[Claude Code] POST /api/organizations/{}/api_keys", org_id);

    // 生成假的 API Key
    let h1 = Uuid::new_v4().to_string().replace("-", "");
    let h2 = Uuid::new_v4().to_string().replace("-", "");
    let h3 = Uuid::new_v4().to_string().replace("-", "");
    let fake_key = format!("sk-ant-api03-{}-{}-{}-AA", &h1[..40], &h2[..12], &h3[..24]);

    let response = serde_json::json!({
        "id": format!("apikey_{}", Uuid::new_v4().to_string().replace("-", "")[..24].to_string()),
        "type": "api_key",
        "api_key": fake_key,
        "name": "claude-code-session-key",
        "created_at": chrono::Utc::now().timestamp(),
        "status": "active",
    });
    with_anthropic_headers(Json(response))
}

/// POST /api/organizations/{org_id}/api-keys - 创建组织 API Key（带连字符）
pub async fn create_org_api_key_dash_handler(
    axum::extract::Path(org_id): axum::extract::Path<String>,
    req: Request,
) -> Response {
    tracing::info!("[Claude Code] POST /api/organizations/{}/api-keys", org_id);
    // 委托给下划线版本的处理器
    create_org_api_key_handler(axum::extract::Path(org_id), req).await
}

/// POST /api/report - 报告端点
pub async fn report_handler() -> Response {
    tracing::info!("[Claude Code] POST /api/report");
    with_anthropic_headers(Json(serde_json::json!({"ok": true})))
}

/// POST /api/telemetry - 遥测端点
pub async fn telemetry_handler() -> Response {
    tracing::info!("[Claude Code] POST /api/telemetry");
    with_anthropic_headers(Json(serde_json::json!({"ok": true})))
}

/// POST /api/events - 事件端点
pub async fn events_handler() -> Response {
    tracing::info!("[Claude Code] POST /api/events");
    with_anthropic_headers(Json(serde_json::json!({"ok": true})))
}

/// POST /api/statsig - Statsig 端点
pub async fn statsig_handler() -> Response {
    tracing::info!("[Claude Code] POST /api/statsig");
    with_anthropic_headers(Json(serde_json::json!({"ok": true})))
}

// ========== OAuth 端点 ==========

/// GET /oauth/authorize - OAuth 授权端点
pub async fn oauth_authorize_handler(
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> Result<Html<String>, StatusCode> {
    let state = params.get("state").cloned().unwrap_or_default();
    let redirect_uri = params.get("redirect_uri").cloned().unwrap_or_default();
    let fake_code = gen_oauth_code();

    tracing::info!("[OAuth] 授权请求, state={}, code={}", &state[..30.min(state.len())], &fake_code[..20]);

    if !redirect_uri.is_empty() {
        let _separator = if redirect_uri.contains('?') { "&" } else { "?" };
        let _redirect_url = format!("{}code={}&state={}", redirect_uri, fake_code, state);
        return Err(axum::http::StatusCode::FOUND); // 重定向
    }

    Ok(Html(build_code_page(fake_code)))
}

/// POST /oauth/token - OAuth Token 交换端点
pub async fn oauth_token_handler(
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> Response {
    tracing::info!("[OAuth] Token 交换请求");

    let grant_type = params.get("grant_type").unwrap_or(&"authorization_code".to_string()).clone();
    let code = params.get("code").unwrap_or(&"?".to_string()).clone();

    tracing::info!("[OAuth] grant_type={}, code={}", grant_type, &code[..20.min(code.len())]);

    let access_token = gen_oauth_token("sk-ant-sid01");
    let refresh_token = gen_oauth_token("sk-ant-rt01");

    let response = OAuthTokenResponse {
        access_token: access_token.clone(),
        token_type: "Bearer".to_string(),
        expires_in: 31536000,
        refresh_token,
        scope: "org:create_api_key user:profile user:inference user:sessions:claude_code user:mcp_servers".to_string(),
        account_uuid: fake_account_uuid(),
    };

    tracing::info!("[OAuth] 返回 access_token={}", &access_token[..35.min(access_token.len())]);
    with_anthropic_headers(Json(response))
}

/// GET /.well-known/openid-configuration - OpenID 配置
pub async fn openid_configuration_handler(
    Host(host): Host,
) -> Response {
    let base = format!("http://{}", host);
    let response = OpenIDConfiguration {
        issuer: base.clone(),
        authorization_endpoint: format!("{}/oauth/authorize", base),
        token_endpoint: format!("{}/oauth/token", base),
        userinfo_endpoint: format!("{}/userinfo", base),
        response_types_supported: vec!["code".to_string()],
        grant_types_supported: vec!["authorization_code".to_string(), "refresh_token".to_string()],
        code_challenge_methods_supported: vec!["S256".to_string()],
        token_endpoint_auth_methods_supported: vec!["none".to_string()],
    };
    with_anthropic_headers(Json(response))
}

/// GET /.well-known/oauth-authorization-server - OAuth 服务器元数据
pub async fn oauth_authorization_server_handler(
    Host(host): Host,
) -> Response {
    let base = format!("http://{}", host);
    let response = OAuthAuthorizationServer {
        issuer: base.clone(),
        authorization_endpoint: format!("{}/oauth/authorize", base),
        token_endpoint: format!("{}/oauth/token", base),
        response_types_supported: vec!["code".to_string()],
        grant_types_supported: vec!["authorization_code".to_string(), "refresh_token".to_string()],
        code_challenge_methods_supported: vec!["S256".to_string()],
    };
    with_anthropic_headers(Json(response))
}

/// GET /oauth/code/callback - OAuth 授权码回调页面
pub async fn oauth_code_callback_handler() -> Html<String> {
    let code = gen_oauth_code();
    tracing::info!("[OAuth] 回调页面, code={}", &code[..20]);
    Html(build_code_page(code))
}

/// GET /generate-code - 生成授权码端点
pub async fn generate_code_handler() -> String {
    let code = gen_oauth_code();
    tracing::info!("[OAuth] /generate-code -> {}", &code[..20]);
    code
}

// ========== 辅助 HTML 生成 ==========

fn build_code_page(code: String) -> String {
    format!(r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>Authorization Successful</title>
<style>
  body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
         display: flex; justify-content: center; align-items: center;
         min-height: 100vh; margin: 0; background: #f7f7f8; }}
  .card {{ background: white; padding: 2.5rem; border-radius: 12px;
           box-shadow: 0 2px 16px rgba(0,0,0,0.08); text-align: center; max-width: 480px; }}
  h2 {{ color: #1a1a2e; margin-bottom: 0.5rem; }}
  .subtitle {{ color: #666; margin-bottom: 1.5rem; }}
  .code-box {{ display: block; padding: 1rem 1.5rem; background: #f0f0f5;
               border-radius: 8px; font-family: 'SF Mono', Monaco, monospace;
               font-size: 0.85rem; word-break: break-all; margin: 1rem 0;
               cursor: pointer; border: 2px solid transparent; transition: border-color 0.2s; }}
  .code-box:hover {{ border-color: #5436DA; }}
  button {{ padding: 0.75rem 2rem; background: #5436DA; color: white;
            border: none; border-radius: 8px; cursor: pointer; font-size: 1rem;
            font-weight: 500; transition: background 0.2s; }}
  button:hover {{ background: #4128b0; }}
  .hint {{ color: #888; font-size: 0.85rem; margin-top: 1rem; }}
  .success {{ color: #22c55e; font-size: 0.85rem; display: none; margin-top: 0.5rem; }}
</style>
</head>
<body>
<div class="card">
  <h2>✓ Authorization Successful</h2>
  <p class="subtitle">Copy this code and paste it in your terminal</p>
  <code class="code-box" id="code" onclick="copyCode()">{code}</code>
  <button onclick="copyCode()">📋 Copy Code</button>
  <p class="success" id="success">Copied! Now paste it in your terminal.</p>
  <p class="hint">Click the code or button to copy</p>
</div>
<script>
function copyCode() {{
  const code = document.getElementById('code').textContent;
  navigator.clipboard.writeText(code).then(() => {{
    document.getElementById('success').style.display = 'block';
    document.querySelector('button').textContent = '✓ Copied!';
    document.querySelector('button').style.background = '#22c55e';
  }}).catch(() => {{
    const range = document.createRange();
    range.selectNode(document.getElementById('code'));
    window.getSelection().removeAllRanges();
    window.getSelection().addRange(range);
  }});
}}
</script>
</body>
</html>"#)
}

// ========== V1 版本的端点 ==========

/// GET /v1/me - V1 版本的用户资料
pub async fn v1_me_handler() -> Response {
    me_handler().await
}

/// POST /v1/me - V1 版本的用户资料（POST）
pub async fn v1_me_post_handler() -> Response {
    me_handler().await
}

/// POST /v1/organizations/{org_id}/api_keys - V1 版本的创建 API Key
pub async fn v1_create_org_api_key_handler(
    axum::extract::Path(org_id): axum::extract::Path<String>,
    req: Request,
) -> Response {
    tracing::info!("[Claude Code] POST /v1/organizations/{}/api_keys", org_id);
    create_org_api_key_handler(axum::extract::Path(org_id), req).await
}

/// GET /v1/dashboard/billing/usage - 计费使用情况
pub async fn v1_billing_usage_handler() -> Response {
    tracing::info!("[Claude Code] GET /v1/dashboard/billing/usage");
    let response = serde_json::json!({
        "daily_costs": [],
        "total_usage": 0.0
    });
    with_anthropic_headers(Json(response))
}

/// GET /v1/usage - 使用情况
pub async fn v1_usage_handler() -> Response {
    tracing::info!("[Claude Code] GET /v1/usage");
    let response = serde_json::json!({
        "daily_costs": [],
        "total_usage": 0
    });
    with_anthropic_headers(Json(response))
}
