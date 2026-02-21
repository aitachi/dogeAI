"""
工具函数集合
"""

import uuid
import secrets
import time
from typing import Optional
from config import logger, MODEL_MAP, DEFAULT_MODEL, DEFAULT_MAX_TOKENS, MAX_OUTPUT_TOKENS_LIMIT


def gen_id(prefix: str = "msg") -> str:
    """生成ID"""
    return f"{prefix}_{uuid.uuid4().hex[:24]}"


def gen_request_id() -> str:
    """生成请求ID"""
    return f"req_{uuid.uuid4().hex}"


def map_model(name: str) -> str:
    """映射模型名称"""
    if name in MODEL_MAP:
        return MODEL_MAP[name]
    if name and ("claude" in name.lower() or "sonnet" in name.lower()
                 or "opus" in name.lower() or "haiku" in name.lower()):
        logger.warning(f"模型 '{name}' 不在 model_map, 使用默认 '{DEFAULT_MODEL}'")
        return DEFAULT_MODEL
    logger.warning(f"未知模型 '{name}', 使用默认 '{DEFAULT_MODEL}'")
    return DEFAULT_MODEL


def clamp_max_tokens(requested: Optional[int], target_model: str) -> int:
    """限制最大token数"""
    DASHSCOPE_HARD_MAX = 8192
    limit = MAX_OUTPUT_TOKENS_LIMIT.get(target_model, DEFAULT_MAX_TOKENS)
    limit = min(limit, DASHSCOPE_HARD_MAX)
    if requested is None or requested <= 0:
        return limit
    result = min(requested, limit)
    if result != requested:
        logger.info(f"max_tokens 钳制: {requested} -> {result} (模型 {target_model} 上限 {limit})")
    return result


def make_anthropic_headers() -> dict:
    """生成Anthropic风格的响应头"""
    req_id = gen_request_id()
    return {
        "x-request-id": req_id,
        "request-id": req_id,
        "anthropic-version": "2023-06-01",
        "x-anthropic-ratelimit-requests-limit": "10000",
        "x-anthropic-ratelimit-requests-remaining": "9999",
        "x-anthropic-ratelimit-requests-reset": "2026-12-31T23:59:59Z",
        "x-anthropic-ratelimit-tokens-limit": "1000000",
        "x-anthropic-ratelimit-tokens-remaining": "999999",
        "x-anthropic-ratelimit-tokens-reset": "2026-12-31T23:59:59Z",
    }


def gen_fake_api_key() -> str:
    """生成伪造的API Key"""
    h = uuid.uuid4().hex + uuid.uuid4().hex + uuid.uuid4().hex
    return f"sk-ant-api03-{h[:40]}-{h[40:52]}-{h[52:76]}-AA"


def gen_oauth_code() -> str:
    """生成OAuth授权码"""
    return secrets.token_urlsafe(32)


def gen_oauth_token(prefix: str) -> str:
    """生成OAuth token"""
    return f"{prefix}-{secrets.token_hex(16)}-{secrets.token_hex(8)}"


def user_profile() -> dict:
    """生成用户资料"""
    from config import FAKE_ACCOUNT_UUID, FAKE_EMAIL, FAKE_NAME
    return {
        "uuid": FAKE_ACCOUNT_UUID,
        "id": FAKE_ACCOUNT_UUID,
        "type": "user",
        "email": FAKE_EMAIL,
        "email_address": FAKE_EMAIL,
        "name": FAKE_NAME,
        "full_name": FAKE_NAME,
        "display_name": FAKE_NAME,
        "created_at": int(time.time()) - 86400 * 365,
        "chat_enabled": True,
        "memberships": [{
            "organization": org_info(),
            "role": "owner",
        }],
    }


def org_info() -> dict:
    """生成组织信息"""
    from config import FAKE_ORG_ID
    return {
        "id": FAKE_ORG_ID,
        "uuid": FAKE_ORG_ID,
        "type": "organization",
        "name": "Default Organization",
        "created_at": int(time.time()) - 86400 * 30,
        "settings": {
            "tier": "scale",
            "claude_console_enabled": True,
        },
        "capabilities": ["api_access", "model_access"],
        "api_disabled_reason": None,
        "active_flags": [],
        "billing_status": "active",
    }
