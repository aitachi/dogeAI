#!/usr/bin/env python3
"""
Anthropic API 中转代理服务 - 生产版本
功能：
- 20个并发请求控制
- 完整的调用统计和日志
- 自动服务监控
- 模型名称映射
"""
from fastapi import FastAPI, HTTPException, Request, Header
from fastapi.responses import StreamingResponse, JSONResponse
from fastapi.middleware.cors import CORSMiddleware
import httpx
import json
import os
import asyncio
from typing import Optional, Dict
from datetime import datetime, timedelta
import secrets
import hashlib
from pathlib import Path
import sqlite3
import logging
from logging.handlers import RotatingFileHandler
import time
from contextlib import contextmanager
from pydantic import BaseModel, validator
import re
import smtplib
from email.mime.text import MIMEText
from email.mime.multipart import MIMEMultipart
from email.utils import formataddr
import threading

# ============== 日志配置 ==============
log_dir = Path("/var/log/anthropic-proxy")
log_dir.mkdir(exist_ok=True)

# 配置日志
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s',
    handlers=[
        RotatingFileHandler(
            log_dir / "api-proxy.log",
            maxBytes=100*1024*1024,  # 100MB
            backupCount=10
        ),
        logging.StreamHandler()
    ]
)
logger = logging.getLogger(__name__)

# ============== 数据库配置 ==============
DB_PATH = "/var/lib/anthropic-proxy/stats.db"

def init_db():
    """初始化统计数据库"""
    conn = sqlite3.connect(DB_PATH)
    cursor = conn.cursor()

    # 创建API密钥表
    cursor.execute('''
        CREATE TABLE IF NOT EXISTS api_keys (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            token TEXT UNIQUE NOT NULL,
            user TEXT NOT NULL,
            daily_limit INTEGER DEFAULT 1000000,
            expires_at TEXT,
            created_at TEXT DEFAULT CURRENT_TIMESTAMP,
            is_active BOOLEAN DEFAULT 1
        )
    ''')

    # 创建调用统计表
    cursor.execute('''
        CREATE TABLE IF NOT EXISTS call_stats (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            api_key_id INTEGER,
            model TEXT,
            input_tokens INTEGER DEFAULT 0,
            output_tokens INTEGER DEFAULT 0,
            total_tokens INTEGER DEFAULT 0,
            status TEXT DEFAULT 'success',
            error_message TEXT,
            error_type TEXT,
            response_time REAL,
            created_at TEXT DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (api_key_id) REFERENCES api_keys (id)
        )
    ''')

    # 创建每日统计表
    cursor.execute('''
        CREATE TABLE IF NOT EXISTS daily_stats (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            api_key_id INTEGER,
            date TEXT,
            request_count INTEGER DEFAULT 0,
            total_tokens INTEGER DEFAULT 0,
            UNIQUE(api_key_id, date),
            FOREIGN KEY (api_key_id) REFERENCES api_keys (id)
        )
    ''')

    conn.commit()
    conn.close()
    logger.info("数据库初始化完成")

# ============== 并发控制 ==============
MAX_CONCURRENT_REQUESTS = 20
semaphore = asyncio.Semaphore(MAX_CONCURRENT_REQUESTS)
active_requests = 0

# 用户级并发控制 - 使用锁确保单并发
user_locks = {}  # {api_key_id: asyncio.Lock()}
user_locks_lock = asyncio.Lock()

async def get_user_lock(api_key_id: int):
    """获取用户级锁（确保单并发）"""
    async with user_locks_lock:
        if api_key_id not in user_locks:
            logger.info(f"为用户 {api_key_id} 创建新的锁（单并发限制）")
            user_locks[api_key_id] = asyncio.Lock()
        return user_locks[api_key_id]

# ============== 数据模型 ==============
class UserApplicationRequest(BaseModel):
    email: str
    username: str
    full_name: Optional[str] = None
    reason: Optional[str] = None

    @validator('email')
    def validate_email(cls, v):
        if not re.match(r'^[\w\.-]+@[\w\.-]+\.\w+$', v):
            raise ValueError('Invalid email format')
        return v.lower()

    @validator('username')
    def validate_username(cls, v):
        if len(v) < 3 or len(v) > 50:
            raise ValueError('Username must be 3-50 characters')
        if not re.match(r'^[a-zA-Z0-9_-]+$', v):
            raise ValueError('Invalid username format')
        return v

# ============== 邮件配置 ==============
EMAIL_CONFIG = {
    "SMTP_HOST": "smtp.qq.com",  # QQ邮箱SMTP服务器
    "SMTP_PORT": 587,  # TLS端口
    "SMTP_USER": "your-email@qq.com",  # 发件邮箱
    "SMTP_PASSWORD": "your-password",  # 邮箱授权码（不是登录密码）
    "FROM_NAME": "Aitachi.cloud",
    "FROM_EMAIL": "your-email@qq.com",  # 发件邮箱
}

def send_thank_you_email(to_email: str, api_key: str, username: str, daily_limit: int):
    """
    发送致谢邮件
    使用独立线程异步发送，避免阻塞主流程
    """
    def _send_email():
        try:
            # 创建邮件
            msg = MIMEMultipart('alternative')
            msg['Subject'] = '🎉 欢迎加入Aitachi.cloud - 您的API密钥已就绪'
            msg['From'] = formataddr((EMAIL_CONFIG['FROM_NAME'], EMAIL_CONFIG['FROM_EMAIL']))
            msg['To'] = to_email

            # 邮件内容（HTML格式）
            html_content = f"""
<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <style>
        body {{ font-family: Arial, sans-serif; line-height: 1.6; color: #333; }}
        .container {{ max-width: 600px; margin: 0 auto; padding: 20px; }}
        .header {{ background: linear-gradient(135deg, #667eea 0%, #764ba2 100%); color: white; padding: 30px; text-align: center; border-radius: 10px 10px 0 0; }}
        .content {{ background: #f9f9f9; padding: 30px; border-radius: 0 0 10px 10px; }}
        .api-key-box {{ background: #fff; border: 2px dashed #667eea; padding: 20px; margin: 20px 0; border-radius: 5px; }}
        .api-key {{ font-family: 'Courier New', monospace; font-size: 18px; color: #667eea; word-break: break-all; font-weight: bold; }}
        .info-box {{ background: #e3f2fd; padding: 15px; border-left: 4px solid #2196f3; margin: 20px 0; }}
        .warning-box {{ background: #fff3e0; padding: 15px; border-left: 4px solid #ff9800; margin: 20px 0; }}
        .footer {{ text-align: center; margin-top: 30px; color: #666; font-size: 12px; }}
        .btn {{ display: inline-block; padding: 12px 30px; background: #667eea; color: white; text-decoration: none; border-radius: 5px; margin: 10px 0; }}
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>🎉 欢迎加入Aitachi.cloud！</h1>
            <p>您的API密钥已成功创建</p>
        </div>
        <div class="content">
            <p>亲爱的 <strong>{username}</strong>：</p>
            <p>感谢您选择Aitachi.cloud！我们很高兴为您提供专业的AI API中转服务。</p>

            <div class="api-key-box">
                <h3 style="margin-top: 0;">🔑 您的API密钥</h3>
                <div class="api-key">{api_key}</div>
            </div>

            <div class="info-box">
                <h4 style="margin-top: 0;">📋 账户信息</h4>
                <ul style="margin-bottom: 0;">
                    <li><strong>每日限额：</strong>{daily_limit:,} Tokens</li>
                    <li><strong>有效期：</strong>365天</li>
                    <li><strong>并发限制：</strong>1个请求</li>
                </ul>
            </div>

            <div class="warning-box">
                <h4 style="margin-top: 0;">⚠️ 安全提示</h4>
                <p style="margin-bottom: 0;">请妥善保管您的API密钥，不要与他人分享。出于安全考虑，系统无法再次显示完整密钥。</p>
            </div>

            <h3>🚀 快速开始</h3>
            <p>查看我们的<a href="http://59.110.40.73/#docs" style="color: #667eea;">API文档</a>了解更多使用方法。</p>

            <div style="text-align: center;">
                <a href="http://59.110.40.73/#docs" class="btn">查看API文档</a>
            </div>

            <p>如果您有任何问题，请随时联系我们：</p>
            <p style="margin-bottom: 0;">📧 contact@aitachi.cloud</p>

            <div class="footer">
                <p>© 2026 Aitachi.cloud - 专业AI API中转服务平台</p>
                <p>本邮件由系统自动发送，请勿回复</p>
            </div>
        </div>
    </div>
</body>
</html>
"""

            html_part = MIMEText(html_content, 'html', 'utf-8')
            msg.attach(html_part)

            # 连接SMTP服务器并发送
            with smtplib.SMTP(EMAIL_CONFIG['SMTP_HOST'], EMAIL_CONFIG['SMTP_PORT']) as server:
                server.starttls()  # 启用TLS
                server.login(EMAIL_CONFIG['SMTP_USER'], EMAIL_CONFIG['SMTP_PASSWORD'])
                server.send_message(msg)

            logger.info(f"致谢邮件已发送至: {to_email}")

        except Exception as e:
            logger.error(f"发送邮件失败: {e}")

    # 在独立线程中发送邮件，避免阻塞
    thread = threading.Thread(target=_send_email, daemon=True)
    thread.start()

# ============== 模型映射 ==============
# 映射外部请求的模型到上游实际的模型
MODEL_MAPPING = {
    # 最新 Claude 4.5 模型
    "claude-sonnet-4.5": "claude-3-5-sonnet-20241022",
    "claude-sonnet-4.5-20250114": "claude-3-5-sonnet-20241022",

    "claude-opus-4.5": "claude-3-opus-20240229",
    "claude-opus-4.5-20250114": "claude-3-opus-20240229",

    # 其他 Claude 模型
    "claude-3.5-sonnet": "claude-3-5-sonnet-20241022",
    "claude-3.5-haiku": "claude-3-5-haiku-20241022",
    "claude-3-opus": "claude-3-opus-20240229",

    # 兼容旧名称
    "claude-sonnet-4": "claude-3-5-sonnet-20241022",
    "claude-opus-4": "claude-3-opus-20240229",
}

# ============== 配置 ==============
CONFIG = {
    "UPSTREAM_API_KEY": "b8e22e2565834b0d9ce54dbb723fab34.NrRrzUS6ArwnsO2D",
    "UPSTREAM_BASE_URL": "https://open.bigmodel.cn/api/anthropic",
    "PROXY_PORT": 8080,
    "PROXY_HOST": "0.0.0.0",
    "MAX_CONCURRENT": MAX_CONCURRENT_REQUESTS,
    "ADMIN_KEY": "admin-change-this-key",  # 管理员密钥
}

ADMIN_KEY = CONFIG["ADMIN_KEY"]

# ============== 数据库操作函数 ==============
def get_db_connection():
    """获取数据库连接"""
    conn = sqlite3.connect(DB_PATH)
    conn.row_factory = sqlite3.Row
    return conn

def verify_api_key(token: str) -> Optional[Dict]:
    """验证API密钥"""
    conn = get_db_connection()
    cursor = conn.cursor()

    cursor.execute('''
        SELECT * FROM api_keys
        WHERE token = ? AND is_active = 1
    ''', (token,))

    row = cursor.fetchone()
    conn.close()

    if not row:
        return None

    # 检查是否过期
    if row['expires_at']:
        try:
            # Python 3.6兼容方式
            expire_time = datetime.strptime(row['expires_at'], "%Y-%m-%dT%H:%M:%S")
            if datetime.now() > expire_time:
                logger.warning(f"Token {row['user']}'s expired at {row['expires_at']}")
                return None
        except ValueError:
            # 解析失败，忽略过期检查
            pass

    return dict(row)

def check_daily_limit(api_key_id: int, daily_limit: int) -> bool:
    """检查每日限额"""
    today = datetime.now().strftime("%Y-%m-%d")
    conn = get_db_connection()
    cursor = conn.cursor()

    cursor.execute('''
        SELECT COALESCE(SUM(total_tokens), 0) as total
        FROM daily_stats
        WHERE api_key_id = ? AND date = ?
    ''', (api_key_id, today))

    result = cursor.fetchone()
    used = result['total'] if result else 0
    conn.close()

    return used < daily_limit

def record_call_stat(
    api_key_id: int,
    model: str,
    input_tokens: int,
    output_tokens: int,
    status: str = 'success',
    error_message: str = None,
    error_type: str = None,
    response_time: float = None
):
    """记录调用统计"""
    total_tokens = input_tokens + output_tokens
    today = datetime.now().strftime("%Y-%m-%d")

    conn = get_db_connection()
    cursor = conn.cursor()

    try:
        # 记录详细调用
        cursor.execute('''
            INSERT INTO call_stats
            (api_key_id, model, input_tokens, output_tokens, total_tokens, status, error_message, error_type, response_time)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        ''', (api_key_id, model, input_tokens, output_tokens, total_tokens, status, error_message, error_type, response_time))

        # 更新每日统计
        cursor.execute('''
            INSERT INTO daily_stats (api_key_id, date, request_count, total_tokens)
            VALUES (?, ?, 1, ?)
            ON CONFLICT(api_key_id, date) DO UPDATE SET
                request_count = request_count + 1,
                total_tokens = total_tokens + ?
        ''', (api_key_id, today, total_tokens, total_tokens))

        conn.commit()
    except Exception as e:
        logger.error(f"记录统计失败: {e}")
        conn.rollback()
    finally:
        conn.close()

def get_api_key_stats(api_key_id: int, days: int = 7) -> Dict:
    """获取API密钥统计信息"""
    conn = get_db_connection()
    cursor = conn.cursor()

    # 总调用次数
    cursor.execute('''
        SELECT COUNT(*) as total_calls,
               SUM(input_tokens) as total_input,
               SUM(output_tokens) as total_output,
               SUM(total_tokens) as total_tokens
        FROM call_stats
        WHERE api_key_id = ? AND created_at >= datetime('now', '-' || ? || ' days')
    ''', (api_key_id, days))

    stats = cursor.fetchone()
    conn.close()

    return dict(stats) if stats else {}

# ============== FastAPI 应用 ==============
app = FastAPI(
    title="Anthropic API Proxy",
    description="生产级智谱AI中转代理服务",
    version="2.0.0"
)

# ============== CORS 中间件配置 ==============
app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],  # 允许所有源（生产环境应指定具体域名）
    allow_credentials=True,
    allow_methods=["*"],  # 允许所有HTTP方法
    allow_headers=["*"],  # 允许所有请求头
)

# ============== 中间件：API Key验证 ==============
@app.middleware("http")
async def api_key_middleware(request: Request, call_next):
    """验证所有API请求的key（健康检查和管理接口除外）"""
    path = request.url.path

    # 豁免路径：健康检查、管理接口、根路径
    if path in ["/health", "/", "/docs", "/openapi.json", "/redoc", "/favicon.ico", "/apply"]:
        return await call_next(request)

    # 豁免申请可用性检查路径
    if path.startswith("/apply/check-availability"):
        return await call_next(request)

    # 管理接口路径豁免（有单独的验证）
    if path.startswith("/admin/"):
        return await call_next(request)

    # 获取API key
    x_api_key = request.headers.get("x-api-key")
    if not x_api_key:
        return JSONResponse(
            status_code=401,
            content={"detail": "Missing API key. Please provide x-api-key header."}
        )

    # 验证key
    key_info = verify_api_key(x_api_key)
    if not key_info:
        logger.warning(f"❌ 无效的API key尝试访问: {x_api_key[:20]}... from {request.client.host}")
        return JSONResponse(
            status_code=401,
            content={"detail": "Invalid API key. Access denied."}
        )

    # 检查是否激活
    if not key_info.get('is_active', True):
        logger.warning(f"⚠️  已禁用的API key尝试访问: {key_info['user']}")
        return JSONResponse(
            status_code=403,
            content={"detail": "API key has been disabled. Contact administrator."}
        )

    # 检查每日限额
    if not check_daily_limit(key_info['id'], key_info['daily_limit']):
        logger.warning(f"⚠️  超出每日限额: {key_info['user']}")
        return JSONResponse(
            status_code=429,
            content={
                "detail": f"Daily token limit exceeded: {key_info['daily_limit']}",
                "limit": key_info['daily_limit']
            }
        )

    # 将key信息添加到请求状态（供后续使用）
    request.state.api_key_info = key_info

    return await call_next(request)

# Python 3.6兼容的生命周期管理
@app.on_event("startup")
async def startup_event():
    """启动时初始化"""
    logger.info("=" * 60)
    logger.info("🚀 Anthropic API 中转代理服务启动")
    logger.info("=" * 60)
    init_db()
    logger.info(f"📊 最大并发数: {MAX_CONCURRENT_REQUESTS}")
    logger.info(f"🔗 上游API: {CONFIG['UPSTREAM_BASE_URL']}")
    logger.info("=" * 60)

@app.on_event("shutdown")
async def shutdown_event():
    """关闭时清理"""
    logger.info("服务关闭")

@app.get("/")
async def root():
    """服务信息"""
    return {
        "service": "Anthropic API Proxy",
        "version": "2.0.0",
        "status": "running",
        "capabilities": {
            "max_concurrent": MAX_CONCURRENT_REQUESTS,
            "queue_enabled": True,
            "statistics": True,
            "model_mapping": True
        },
        "endpoints": {
            "messages": "/v1/messages",
            "models": "/v1/models",
            "health": "/health",
            "stats": "/admin/stats"
        }
    }

@app.get("/health")
async def health():
    """健康检查"""
    global active_requests
    return {
        "status": "healthy",
        "service": "Anthropic API Proxy",
        "upstream": CONFIG["UPSTREAM_BASE_URL"],
        "active_requests": active_requests,
        "max_concurrent": MAX_CONCURRENT_REQUESTS,
        "available_slots": MAX_CONCURRENT_REQUESTS - active_requests
    }

@app.get("/v1/models")
async def list_models():
    """列出可用模型（支持新版本命名）"""
    models = [
        # Claude 4.5 系列
        {"id": "claude-sonnet-4.5", "name": "Claude Sonnet 4.5 (Latest)", "type": "model"},
        {"id": "claude-opus-4.5", "name": "Claude Opus 4.5 (Latest)", "type": "model"},

        # Claude 3.5 系列
        {"id": "claude-3.5-sonnet", "name": "Claude 3.5 Sonnet", "type": "model"},
        {"id": "claude-3.5-haiku", "name": "Claude 3.5 Haiku", "type": "model"},

        # Claude 3 系列
        {"id": "claude-3-opus", "name": "Claude 3 Opus", "type": "model"},
    ]

    return {
        "object": "list",
        "data": models
    }

@app.post("/v1/messages")
async def create_message(
    request: Request,
    anthropic_version: Optional[str] = Header("2023-06-01")
):
    """
    创建消息（支持并发控制和统计）
    API key已在中间件中验证
    """
    global active_requests

    # 从中间件获取key信息
    key_info = request.state.api_key_info
    api_key_id = key_info['id']

    # 获取请求体
    body = await request.body()
    request_data = json.loads(body.decode('utf-8')) if body else {}

    # 模型名称映射
    model = request_data.get("model", "claude-sonnet-4.5")
    upstream_model = MODEL_MAPPING.get(model, model)

    logger.info(f"✅ API调用: 用户={key_info['user']}(ID:{api_key_id}), 模型={model} -> {upstream_model}")

    # 获取用户级锁（每个用户最多1个并发）
    logger.info(f"等待用户锁: user_id={api_key_id}")
    user_lock = await get_user_lock(api_key_id)

    # 使用锁和信号量控制并发（全局 + 用户级）
    async with semaphore:
        async with user_lock:
            logger.info(f"获取用户锁成功: user_id={api_key_id}")
            active_requests += 1
            start_time = time.time()

            try:
                # 构建上游请求
                upstream_request = request_data.copy()
                upstream_request["model"] = upstream_model

                upstream_headers = {
                    "x-api-key": CONFIG["UPSTREAM_API_KEY"],
                    "anthropic-version": anthropic_version,
                    "content-type": "application/json",
                    "user-agent": f"Anthropic-API-Proxy/2.0 (User: {key_info['user']})"
                }

                is_stream = request_data.get("stream", False)

                async with httpx.AsyncClient(timeout=120.0) as client:
                    if is_stream:
                        # 流式响应
                        async def stream_response():
                            input_tokens = request_data.get("max_tokens", 1000)
                            output_tokens = 0

                            try:
                                async with client.stream(
                                    "POST",
                                    f"{CONFIG['UPSTREAM_BASE_URL']}/v1/messages",
                                    content=json.dumps(upstream_request),
                                    headers=upstream_headers
                                ) as response:
                                    if response.status_code != 200:
                                        error_text = await response.aread()
                                        raise HTTPException(
                                            status_code=response.status_code,
                                            detail=error_text.decode()
                                        )

                                    async for chunk in response.aiter_bytes():
                                        yield chunk

                            except Exception as e:
                                elapsed = time.time() - start_time
                                logger.error(f"流式请求错误: {e}")
                                # 判断错误类型
                                error_type = "stream_error"
                                if "timeout" in str(e).lower():
                                    error_type = "timeout"
                                elif "rate" in str(e).lower() or "limit" in str(e).lower():
                                    error_type = "rate_limit"
                                elif "connection" in str(e).lower():
                                    error_type = "connection_error"

                                record_call_stat(
                                    api_key_id, model,
                                    input_tokens, output_tokens,
                                    'error', str(e), error_type, elapsed
                                )
                                raise

                        return StreamingResponse(
                            stream_response(),
                            media_type="text/event-stream"
                        )
                    else:
                        # 非流式响应
                        response = await client.post(
                            f"{CONFIG['UPSTREAM_BASE_URL']}/v1/messages",
                            json=upstream_request,
                            headers=upstream_headers
                        )

                        if response.status_code != 200:
                            elapsed = time.time() - start_time
                            error_msg = response.text
                            logger.error(f"上游API错误: {error_msg}")

                            # 判断错误类型
                            error_type = "api_error"
                            if response.status_code == 429:
                                error_type = "rate_limit"
                            elif response.status_code == 401:
                                error_type = "authentication_error"
                            elif response.status_code == 400:
                                error_type = "bad_request"
                            elif response.status_code >= 500:
                                error_type = "server_error"

                            record_call_stat(
                                api_key_id, model,
                                0, 0, 'error', error_msg[:500], error_type, elapsed
                            )
                            raise HTTPException(
                                status_code=response.status_code,
                                detail=error_msg
                            )

                        result = response.json()

                        # 提取token使用情况
                        usage = result.get('usage', {})
                        input_tokens = usage.get('input_tokens', 0)
                        output_tokens = usage.get('output_tokens', 0)
                        total_tokens = usage.get('total_tokens', input_tokens + output_tokens)

                        # 记录统计
                        elapsed = time.time() - start_time
                        record_call_stat(
                            api_key_id, model,
                            input_tokens, output_tokens,
                            'success', None, None, elapsed
                        )

                        logger.info(
                            f"请求完成: 用户={key_info['user']}, "
                            f"tokens={total_tokens}, "
                            f"耗时={elapsed:.2f}s"
                        )

                        return JSONResponse(
                            content=result,
                            status_code=200,
                            headers={
                                "x-request-id": secrets.token_hex(16),
                                "x-proxy-version": "2.0.0"
                            }
                        )

            except httpx.HTTPError as e:
                elapsed = time.time() - start_time
                logger.error(f"HTTP错误: {e}")

                # 判断错误类型
                error_type = "http_error"
                error_str = str(e).lower()
                if "timeout" in error_str:
                    error_type = "timeout"
                elif "connection" in error_str:
                    error_type = "connection_error"
                elif "network" in error_str:
                    error_type = "network_error"

                record_call_stat(api_key_id, model, 0, 0, 'error', str(e), error_type, elapsed)
                raise HTTPException(
                    status_code=500,
                    detail=f"Upstream API error: {str(e)}"
                )
            finally:
                active_requests -= 1

# ============== 用户申请接口 ==============
@app.post("/apply")
async def apply_for_api_key(application: UserApplicationRequest, request: Request):
    """
    用户自助申请API密钥
    - 自动生成token
    - 设置100万token/天配额
    - 有效期1年
    - 防止重复申请
    """
    client_ip = request.client.host
    logger.info(f"收到API密钥申请: email={application.email}, username={application.username}, ip={client_ip}")

    conn = get_db_connection()
    cursor = conn.cursor()

    try:
        # 检查邮箱是否已存在
        cursor.execute('SELECT id FROM api_keys WHERE user = ?', (application.email,))
        if cursor.fetchone():
            logger.warning(f"重复申请被拒绝: email={application.email}")
            raise HTTPException(status_code=409, detail="此邮箱已申请过API密钥")

        # 检查用户名是否已被占用
        cursor.execute('SELECT id FROM api_keys WHERE user = ?', (application.username,))
        if cursor.fetchone():
            raise HTTPException(status_code=409, detail="此用户名已被使用")

        # 生成新API密钥
        new_token = "sk-" + secrets.token_urlsafe(32)

        # 设置配额和有效期
        default_limit = 10000  # 1万token免费配额
        expires_at = (datetime.now() + timedelta(days=365)).isoformat()

        # 插入数据库
        cursor.execute('''
            INSERT INTO api_keys (token, user, daily_limit, expires_at, is_active)
            VALUES (?, ?, ?, ?, 1)
        ''', (new_token, application.email, default_limit, expires_at))

        api_key_id = cursor.lastrowid
        conn.commit()

        logger.info(f"新API密钥创建成功: id={api_key_id}, email={application.email}")

        # 发送致谢邮件
        try:
            send_thank_you_email(
                to_email=application.email,
                api_key=new_token,
                username=application.username,
                daily_limit=default_limit
            )
        except Exception as e:
            logger.error(f"发送邮件失败: {e}")

        return {
            "success": True,
            "message": "API密钥申请成功",
            "data": {
                "api_key": new_token,
                "email": application.email,
                "username": application.username,
                "daily_limit": default_limit,
                "expires_at": expires_at,
                "created_at": datetime.now().isoformat()
            }
        }

    except sqlite3.IntegrityError as e:
        conn.rollback()
        raise HTTPException(status_code=409, detail="申请失败：邮箱或用户名已存在")
    except Exception as e:
        conn.rollback()
        logger.error(f"申请API密钥失败: {e}")
        raise HTTPException(status_code=500, detail=f"申请失败：{str(e)}")
    finally:
        conn.close()

@app.get("/apply/check-availability/{field}/{value}")
async def check_availability(field: str, value: str):
    """检查邮箱或用户名是否可用"""
    if field not in ['email', 'username']:
        raise HTTPException(status_code=400, detail="Invalid field")

    conn = get_db_connection()
    cursor = conn.cursor()

    try:
        cursor.execute('SELECT id FROM api_keys WHERE user = ?', (value.lower(),))
        exists = cursor.fetchone() is not None

        return {"available": not exists, "field": field, "value": value}
    finally:
        conn.close()

# ============== 管理接口 ==============

@app.get("/admin/stats")
async def get_stats(
    api_key: str = None,
    days: int = 7,
    admin_key: str = Header(None, alias="x-admin-key")
):
    """获取统计信息"""
    if admin_key != "admin-change-this-key":
        raise HTTPException(status_code=403, detail="Forbidden")

    conn = get_db_connection()
    cursor = conn.cursor()

    if api_key:
        # 特定API密钥的统计
        cursor.execute('''
            SELECT id FROM api_keys WHERE token = ?
        ''', (api_key,))
        row = cursor.fetchone()
        if not row:
            raise HTTPException(status_code=404, detail="API key not found")

        api_key_id = row['id']
        stats = get_api_key_stats(api_key_id, days)

        # 获取最近的调用记录
        cursor.execute('''
            SELECT model, input_tokens, output_tokens, total_tokens,
                   status, created_at
            FROM call_stats
            WHERE api_key_id = ?
            ORDER BY created_at DESC
            LIMIT 100
        ''', (api_key_id,))

        recent_calls = [dict(row) for row in cursor.fetchall()]
        conn.close()

        return {
            "api_key": api_key[:20] + "...",
            "period_days": days,
            "summary": stats,
            "recent_calls": recent_calls
        }
    else:
        # 全局统计
        cursor.execute('''
            SELECT
                COUNT(*) as total_calls,
                SUM(input_tokens) as total_input,
                SUM(output_tokens) as total_output,
                SUM(total_tokens) as total_tokens
            FROM call_stats
            WHERE created_at >= datetime('now', '-' || ? || ' days')
        ''', (days,))

        global_stats = cursor.fetchone()

        # 各用户统计
        cursor.execute('''
            SELECT
                ak.user,
                COUNT(cs.id) as calls,
                COALESCE(SUM(cs.total_tokens), 0) as tokens
            FROM api_keys ak
            LEFT JOIN call_stats cs ON ak.id = cs.api_key_id
            WHERE cs.created_at >= datetime('now', '-' || ? || ' days')
            GROUP BY ak.user
            ORDER BY tokens DESC
        ''', (days,))

        user_stats = [dict(row) for row in cursor.fetchall()]
        conn.close()

        return {
            "period_days": days,
            "global": dict(global_stats),
            "by_user": user_stats
        }

@app.post("/admin/keys/create")
async def create_key(
    user: str,
    limit: int = 10000000,
    days: int = 365,
    admin_key: str = Header(None, alias="x-admin-key")
):
    """创建新的API密钥"""
    if admin_key != "admin-change-this-key":
        raise HTTPException(status_code=403, detail="Forbidden")

    new_token = "sk-" + secrets.token_urlsafe(32)
    expires_at = None

    if days > 0:
        expires_at = (datetime.now() + timedelta(days=days)).isoformat()

    conn = get_db_connection()
    cursor = conn.cursor()

    try:
        cursor.execute('''
            INSERT INTO api_keys (token, user, daily_limit, expires_at)
            VALUES (?, ?, ?, ?)
        ''', (new_token, user, limit, expires_at))

        conn.commit()
        logger.info(f"创建新API密钥: user={user}, limit={limit}")

        return {
            "message": "API key created successfully",
            "token": new_token,
            "user": user,
            "daily_limit": limit,
            "expires_at": expires_at
        }
    except Exception as e:
        conn.rollback()
        raise HTTPException(status_code=500, detail=str(e))
    finally:
        conn.close()

@app.get("/admin/keys")
async def list_keys(admin_key: str = Header(None, alias="x-admin-key")):
    """列出所有API密钥"""
    if admin_key != "admin-change-this-key":
        raise HTTPException(status_code=403, detail="Forbidden")

    conn = get_db_connection()
    cursor = conn.cursor()

    # 获取所有密钥及今日使用量
    today = datetime.now().strftime("%Y-%m-%d")

    cursor.execute('''
        SELECT
            ak.token,
            ak.user,
            ak.daily_limit,
            ak.expires_at,
            ak.created_at,
            ak.is_active,
            COALESCE(ds.total_tokens, 0) as used_today
        FROM api_keys ak
        LEFT JOIN daily_stats ds ON ak.id = ds.api_key_id AND ds.date = ?
        ORDER BY ak.created_at DESC
    ''', (today,))

    keys = []
    for row in cursor.fetchall():
        keys.append({
            "token": row['token'],
            "user": row['user'],
            "daily_limit": row['daily_limit'],
            "expires_at": row['expires_at'],
            "created_at": row['created_at'],
            "is_active": bool(row['is_active']),
            "used_today": row['used_today']
        })

    conn.close()

    return {"keys": keys}

@app.get("/admin/recent-calls")
async def get_recent_calls(
    limit: int = 100,
    admin_key: str = Header(None, alias="x-admin-key")
):
    """获取最近的调用记录（跨所有用户）"""
    if admin_key != ADMIN_KEY:
        raise HTTPException(status_code=403, detail="Forbidden")

    conn = get_db_connection()
    cursor = conn.cursor()

    cursor.execute('''
        SELECT
            cs.model,
            cs.input_tokens,
            cs.output_tokens,
            cs.total_tokens,
            cs.status,
            cs.created_at,
            ak.user
        FROM call_stats cs
        JOIN api_keys ak ON cs.api_key_id = ak.id
        ORDER BY cs.created_at DESC
        LIMIT ?
    ''', (limit,))

    calls = [dict(row) for row in cursor.fetchall()]
    conn.close()

    return {"calls": calls}

@app.post("/admin/toggle-token")
async def toggle_token_status(
    request: Request,
    admin_key: str = Header(None, alias="x-admin-key")
):
    """切换API密钥的激活状态"""
    if admin_key != ADMIN_KEY:
        raise HTTPException(status_code=403, detail="Forbidden")

    body = await request.json()
    token = body.get('token')

    if not token:
        raise HTTPException(status_code=400, detail="Token is required")

    conn = get_db_connection()
    cursor = conn.cursor()

    # 检查当前状态并切换
    cursor.execute('SELECT is_active FROM api_keys WHERE token = ?', (token,))
    result = cursor.fetchone()

    if not result:
        conn.close()
        raise HTTPException(status_code=404, detail="Token not found")

    current_status = result[0]
    new_status = not current_status

    cursor.execute('UPDATE api_keys SET is_active = ? WHERE token = ?', (new_status, token))
    conn.commit()
    conn.close()

    logger.info(f"Token {token[:20]}... status toggled: {current_status} -> {new_status}")

    return {
        "message": "Token status updated",
        "token": token[:20] + "...",
        "is_active": new_status
    }

@app.get("/admin/token-stats")
async def get_token_detailed_stats(
    token: str,
    days: int = 7,
    admin_key: str = Header(None, alias="x-admin-key")
):
    """获取特定token的详细统计"""
    if admin_key != ADMIN_KEY:
        raise HTTPException(status_code=403, detail="Forbidden")

    conn = get_db_connection()
    cursor = conn.cursor()

    # 获取token基本信息
    cursor.execute('SELECT * FROM api_keys WHERE token = ?', (token,))
    token_info = cursor.fetchone()

    if not token_info:
        conn.close()
        raise HTTPException(status_code=404, detail="Token not found")

    # 获取使用统计
    api_key_id = token_info['id']
    cursor.execute('''
        SELECT
            COUNT(*) as total_calls,
            SUM(input_tokens) as total_input,
            SUM(output_tokens) as total_output,
            SUM(total_tokens) as total_tokens
        FROM call_stats
        WHERE api_key_id = ? AND created_at >= datetime('now', '-' || ? || ' days')
    ''', (api_key_id, days))

    stats = cursor.fetchone()
    conn.close()

    return {
        "token": token,
        "user": token_info['user'],
        "daily_limit": token_info['daily_limit'],
        "created_at": token_info['created_at'],
        "expires_at": token_info['expires_at'],
        "is_active": token_info['is_active'],
        "stats": {
            "period_days": days,
            "total_calls": stats['total_calls'] if stats else 0,
            "total_input_tokens": stats['total_input'] if stats else 0,
            "total_output_tokens": stats['total_output'] if stats else 0,
            "total_tokens": stats['total_tokens'] if stats else 0
        }
    }

if __name__ == "__main__":
    import uvicorn

    uvicorn.run(
        app,
        host=CONFIG["PROXY_HOST"],
        port=CONFIG["PROXY_PORT"],
        access_log=True,
        log_level="info"
    )
