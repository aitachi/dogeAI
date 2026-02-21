"""
Claude Code专用端点和核心消息路由
"""

import json
import logging
import traceback
from fastapi import Request
from fastapi.responses import JSONResponse, StreamingResponse

import httpx

from config import logger, TARGET_BASE_URL, TARGET_API_KEY
from utils import make_anthropic_headers
from converter import build_openai_request, build_anthropic_response
from streamer import stream_sse


def register_claude_routes(app):
    """注册Claude Code专用路由"""

    @app.get("/api/hello")
    async def api_hello(request: Request):
        """平台健康检查"""
        logger.info("[Platform] GET /api/hello -> {\"message\": \"hello\"} ✓")
        return JSONResponse(content={"message": "hello"})

    @app.get("/v1/oauth/hello")
    async def oauth_hello(request: Request):
        """OAuth健康检查"""
        logger.info("[Platform] GET /v1/oauth/hello -> {\"message\":\"hello\"} ✓")
        return JSONResponse(content={"message": "hello"})

    @app.get("/api/claude_code/settings")
    async def claude_code_settings(request: Request):
        """Claude Code设置端点"""
        logger.info("[Claude Code] GET /api/claude_code/settings ✓")
        return JSONResponse(content={
            "expiry": None,
            "isolated": False,
            "allowed_tools": ["computer", "text_editor", "bash"],
            "max_turns": None,
            "internet_policy": "allow",
        })

    @app.get("/api/claude_code/policy_limits")
    async def claude_code_policy_limits(request: Request):
        """Claude Code策略限制"""
        logger.info("[Claude Code] GET /api/claude_code/policy_limits ✓")
        return JSONResponse(content={
            "rate_limits": {
                "requests_per_minute": 60,
                "tokens_per_minute": 1000000,
                "tokens_per_day": 50000000,
            },
            "usage": {
                "tokens_used_today": 0,
                "requests_today": 0,
            },
            "limits": {},
        })

    @app.get("/api/claude_code/penguin_mode")
    async def claude_code_penguin_mode(request: Request):
        """Claude Code penguin模式"""
        logger.info("[Claude Code] GET /api/claude_code/penguin_mode ✓")
        return JSONResponse(content={
            "enabled": False,
        })

    # 其他辅助路由
    @app.get("/.well-known/openid-configuration")
    async def openid_configuration(request: Request):
        base = str(request.base_url).rstrip("/")
        return JSONResponse(content={
            "issuer": base,
            "authorization_endpoint": f"{base}/oauth/authorize",
            "token_endpoint": f"{base}/oauth/token",
            "userinfo_endpoint": f"{base}/userinfo",
            "response_types_supported": ["code"],
            "grant_types_supported": ["authorization_code", "refresh_token"],
            "code_challenge_methods_supported": ["S256"],
            "token_endpoint_auth_methods_supported": ["none"],
        })

    @app.get("/.well-known/oauth-authorization-server")
    async def oauth_metadata(request: Request):
        base = str(request.base_url).rstrip("/")
        return JSONResponse(content={
            "issuer": base,
            "authorization_endpoint": f"{base}/oauth/authorize",
            "token_endpoint": f"{base}/oauth/token",
            "response_types_supported": ["code"],
            "grant_types_supported": ["authorization_code", "refresh_token"],
            "code_challenge_methods_supported": ["S256"],
            "token_endpoint_auth_methods_supported": ["none"],
        })

    @app.api_route("/userinfo", methods=["GET", "POST"])
    async def get_userinfo(request: Request):
        from utils import user_profile
        return JSONResponse(content=user_profile())

    @app.api_route("/api/me", methods=["GET", "POST"])
    async def get_api_me(request: Request):
        from utils import user_profile
        return JSONResponse(content=user_profile())

    @app.api_route("/v1/me", methods=["GET", "POST"])
    async def get_v1_me(request: Request):
        from utils import user_profile
        return JSONResponse(content=user_profile())

    @app.post("/v1/organizations/{org_id}/api_keys")
    async def create_org_api_key(org_id: str, request: Request):
        from routes.api import _handle_create_api_key
        return await _handle_create_api_key(request, org_id)

    @app.api_route("/v1/organizations/{org_id:path}", methods=["GET", "POST"])
    async def handle_organization_detail(request: Request, org_id: str):
        from utils import org_info
        return JSONResponse(
            content=org_info(),
            headers=make_anthropic_headers(),
        )

    @app.get("/v1/dashboard/billing/usage")
    async def handle_billing():
        return JSONResponse(
            content={"daily_costs": [], "total_usage": 0.0},
            headers=make_anthropic_headers(),
        )

    @app.api_route("/v1/usage", methods=["GET", "POST"])
    async def handle_usage(request: Request):
        return JSONResponse(
            content={"daily_costs": [], "total_usage": 0},
            headers=make_anthropic_headers(),
        )

    @app.post("/v1/messages/count_tokens")
    async def handle_count_tokens(request: Request):
        body = await request.json()
        text = json.dumps(body.get("messages", []))
        estimated = max(1, len(text) // 4)
        return JSONResponse(
            content={"input_tokens": estimated},
            headers=make_anthropic_headers(),
        )

    @app.post("/v1/messages/batches")
    async def handle_batches(request: Request):
        return JSONResponse(
            content={"type": "error", "error": {"type": "not_found_error", "message": "Batches not available"}},
            status_code=404,
            headers=make_anthropic_headers(),
        )

    @app.post("/v1/complete")
    async def handle_complete(request: Request):
        """兼容旧版complete接口"""
        body = await request.json()
        body["messages"] = [{"role": "user", "content": body.get("prompt", "")}]
        body.setdefault("stream", False)
        scope = request.scope.copy()
        new_request = Request(scope, request.receive)
        new_request._json = body
        return await handle_messages(new_request)

    # 通配符路由（必须放最后）
    @app.api_route(
        "/{path:path}",
        methods=["GET", "POST", "PUT", "DELETE", "PATCH", "OPTIONS", "HEAD"],
    )
    async def catch_all(request: Request, path: str):
        method = request.method
        host = request.headers.get("host", "?")
        auth = request.headers.get("authorization", "")[:40]
        xkey = request.headers.get("x-api-key", "")[:40]

        body_text = ""
        if method in ("POST", "PUT", "PATCH"):
            try:
                raw = await request.body()
                body_text = raw.decode("utf-8", errors="replace")[:500]
            except Exception:
                pass

        logger.warning(
            f"[CATCH-ALL] {method} /{path} | Host={host} | Auth={auth} | x-api-key={xkey}"
        )
        if body_text:
            logger.warning(f"[CATCH-ALL] Body: {body_text}")

        return JSONResponse(
            content={"status": "ok", "type": "proxy_catch_all", "path": f"/{path}"},
            headers=make_anthropic_headers(),
        )


def register_messages_handler(app):
    """注册核心消息路由处理器"""

    @app.post("/v1/messages")
    async def handle_messages(request: Request):
        """处理消息请求"""
        # 导入监控函数
        from routes.monitor import start_call_recording, end_call_recording

        try:
            body = await request.json()
        except Exception as e:
            return JSONResponse(
                status_code=400,
                content={
                    "type": "error",
                    "error": {"type": "invalid_request_error", "message": f"Invalid JSON: {e}"},
                },
            )

        original_model = body.get("model", "claude-sonnet-4-5-20250929")
        is_stream = body.get("stream", False)

        logger.info(
            f"收到请求 model={original_model} stream={is_stream} "
            f"msgs={len(body.get('messages', []))} tools={len(body.get('tools', []))} "
            f"raw_max_tokens={body.get('max_tokens')}"
        )

        openai_body = build_openai_request(body)
        target_model = openai_body.get('model', original_model)

        logger.info(
            f"转发 -> {target_model}, max_tokens={openai_body['max_tokens']}, "
            f"目标: {TARGET_BASE_URL}/chat/completions"
        )

        # 开始监控记录
        request_id, call_id = start_call_recording(target_model, original_model, is_stream)
        logger.info(f"[Monitor] 记录调用开始: request_id={request_id}, call_id={call_id}")

        headers = {
            "Authorization": f"Bearer {TARGET_API_KEY}",
            "Content-Type": "application/json",
        }
        target_url = f"{TARGET_BASE_URL}/chat/completions"
        resp_headers = make_anthropic_headers()

        # 流式响应
        if is_stream:
            try:
                client = httpx.AsyncClient(timeout=httpx.Timeout(300.0, connect=30.0))
                req = client.build_request("POST", target_url, json=openai_body, headers=headers)
                resp = await client.send(req, stream=True)

                if resp.status_code != 200:
                    err_body = await resp.aread()
                    await resp.aclose()
                    await client.aclose()
                    try:
                        err_msg = json.loads(err_body).get("error", {}).get("message", err_body.decode())
                    except Exception:
                        err_msg = err_body.decode("utf-8", errors="replace")
                    logger.error(f"上游错误 {resp.status_code}: {err_msg}")

                    # 记录失败
                    end_call_recording(call_id, "error", error_message=err_msg)

                    return JSONResponse(
                        status_code=resp.status_code,
                        content={"type": "error", "error": {"type": "api_error", "message": err_msg}},
                        headers=resp_headers,
                    )

                # 创建流式响应生成器
                async def _gen():
                    input_tokens = 0
                    output_tokens = 0
                    try:
                        async for chunk in stream_sse(resp, original_model):
                            # 尝试解析token使用情况
                            if "usage" in chunk:
                                try:
                                    data = json.loads(chunk.split("data: ")[1])
                                    usage = data.get("message_delta", {}).get("usage", {})
                                    if usage:
                                        output_tokens = usage.get("output_tokens", 0)
                                except Exception:
                                    pass
                            yield chunk
                    except Exception as exc:
                        logger.error(f"流式错误: {exc}\n{traceback.format_exc()}")
                        end_call_recording(call_id, "error", error_message=str(exc))
                    finally:
                        await resp.aclose()
                        await client.aclose()
                        # 记录成功完成
                        end_call_recording(call_id, "success", input_tokens=input_tokens, output_tokens=output_tokens)

                return StreamingResponse(
                    _gen(),
                    media_type="text/event-stream",
                    headers={**resp_headers, "Cache-Control": "no-cache", "Connection": "keep-alive"},
                )

            except httpx.ConnectError as e:
                logger.error(f"连接失败: {e}")
                end_call_recording(call_id, "error", error_message=str(e))
                return JSONResponse(
                    status_code=502,
                    content={"type": "error", "error": {"type": "api_error", "message": f"Connect failed: {e}"}},
                    headers=resp_headers,
                )

        # 非流式响应
        try:
            async with httpx.AsyncClient(timeout=httpx.Timeout(300.0, connect=30.0)) as client:
                resp = await client.post(target_url, json=openai_body, headers=headers)

                if resp.status_code != 200:
                    try:
                        err_msg = resp.json().get("error", {}).get("message", resp.text)
                    except Exception:
                        err_msg = resp.text
                    logger.error(f"上游错误 {resp.status_code}: {err_msg}")

                    # 记录失败
                    end_call_recording(call_id, "error", error_message=err_msg)

                    return JSONResponse(
                        status_code=resp.status_code,
                        content={"type": "error", "error": {"type": "api_error", "message": err_msg}},
                        headers=resp_headers,
                    )

                anthropic_resp = build_anthropic_response(resp.json(), original_model)

                # 记录成功
                usage = anthropic_resp.get("usage", {})
                input_tokens = usage.get("input_tokens", 0)
                output_tokens = usage.get("output_tokens", 0)
                end_call_recording(call_id, "success", input_tokens=input_tokens, output_tokens=output_tokens)

                logger.info(f"响应 stop_reason={anthropic_resp.get('stop_reason')}")
                return JSONResponse(content=anthropic_resp, headers=resp_headers)

        except httpx.ConnectError as e:
            end_call_recording(call_id, "error", error_message=str(e))
            return JSONResponse(
                status_code=502,
                content={"type": "error", "error": {"type": "api_error", "message": f"Connect failed: {e}"}},
                headers=resp_headers,
            )
