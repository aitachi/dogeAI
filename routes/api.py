"""
平台API端点：bootstrap、auth、account、settings等
"""

import secrets
import datetime
from fastapi import Request
from fastapi.responses import JSONResponse

from config import logger
from utils import user_profile, org_info, gen_fake_api_key, make_anthropic_headers


def register_api_routes(app):
    """注册平台API路由"""

    @app.get("/api/bootstrap")
    async def api_bootstrap(request: Request):
        logger.info("[Platform] GET /api/bootstrap")
        return JSONResponse(content={
            "account": user_profile(),
            "organizations": [org_info()],
            "statsig": {},
        })

    @app.get("/api/auth")
    async def api_auth(request: Request):
        logger.info("[Platform] GET /api/auth")
        return JSONResponse(content={
            "account": user_profile(),
            "account_flags": [],
        })

    @app.get("/api/auth/session")
    async def api_auth_session(request: Request):
        logger.info("[Platform] GET /api/auth/session")
        return JSONResponse(content={
            "account": user_profile(),
            "session": {
                "id": f"sess_{secrets.token_hex(16)}",
                "expires_at": (
                    datetime.datetime.utcnow() + datetime.timedelta(days=30)
                ).isoformat() + "Z",
            },
            "account_flags": [],
        })

    @app.get("/api/account")
    async def api_account(request: Request):
        logger.info("[Platform] GET /api/account")
        return JSONResponse(content=user_profile())

    @app.get("/api/settings")
    async def api_settings(request: Request):
        logger.info("[Platform] GET /api/settings")
        return JSONResponse(content={})

    @app.get("/api/organizations")
    async def api_organizations(request: Request):
        logger.info("[Platform] GET /api/organizations")
        return JSONResponse(content={"data": [org_info()]})

    @app.post("/api/organizations/{org_id}/api_keys")
    async def api_create_key_no_prefix(request: Request, org_id: str):
        return await _handle_create_api_key(request, org_id)

    @app.post("/api/organizations/{org_id}/api-keys")
    async def api_create_key_hyphen(request: Request, org_id: str):
        return await _handle_create_api_key(request, org_id)

    @app.post("/api/report")
    async def api_report(request: Request):
        return JSONResponse(content={"ok": True})

    @app.post("/api/telemetry")
    async def api_telemetry(request: Request):
        return JSONResponse(content={"ok": True})

    @app.post("/api/events")
    async def api_events(request: Request):
        return JSONResponse(content={"ok": True})

    @app.post("/api/statsig")
    async def api_statsig(request: Request):
        return JSONResponse(content={"ok": True})


async def _handle_create_api_key(request: Request, org_id: str):
    """处理创建API Key请求"""
    try:
        body = await request.json()
    except Exception:
        body = {}

    key_name = body.get("name", "claude-code-session-key")
    fake_key = gen_fake_api_key()

    logger.info(f"[Auth] 创建 API Key: org={org_id} name={key_name}")
    logger.info(f"[Auth] 返回 key={fake_key[:30]}…")

    return JSONResponse(
        content={
            "id": gen_id(),
            "type": "api_key",
            "api_key": fake_key,
            "name": key_name,
            "created_at": int(time.time()),
            "status": "active",
        },
        headers=make_anthropic_headers(),
    )


def gen_id():
    """生成ID（工具函数）"""
    import uuid
    return f"apikey_{uuid.uuid4().hex[:24]}"
