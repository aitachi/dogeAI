"""
Claude Code专用端点和核心消息路由
统一使用 AITACHI Cloud API
"""

import json
import logging
import traceback
import time
import uuid
from typing import Optional
from fastapi import Request
from fastapi.responses import JSONResponse, StreamingResponse

import httpx

from config import logger, API_BASE_URL, API_KEY, DEFAULT_MODEL
from utils import make_anthropic_headers
from anthropic_client import call_anthropic_api, call_anthropic_api_stream, map_to_api_model
from routes.monitor import start_call_recording, end_call_recording


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
    """注册核心消息路由处理器 - 统一使用 AITACHI Cloud API"""

    @app.post("/v1/messages")
    async def handle_messages(request: Request):
        """处理消息请求 - 统一使用 AITACHI Cloud API"""

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

        original_model = body.get("model", DEFAULT_MODEL)
        is_stream = body.get("stream", False)

        # 映射到 API 模型名称
        target_model = map_to_api_model(original_model)

        logger.info(
            f"收到请求 model={original_model} -> {target_model} stream={is_stream} "
            f"msgs={len(body.get('messages', []))} tools={len(body.get('tools', []))} "
            f"raw_max_tokens={body.get('max_tokens')}"
        )

        # 开始监控记录
        request_id, call_id = start_call_recording(target_model, original_model, is_stream)
        logger.info(f"[Monitor] 记录调用开始: request_id={request_id}, call_id={call_id}")

        resp_headers = make_anthropic_headers()

        # 准备消息
        messages = body.get("messages", [])
        system = body.get("system")
        max_tokens = body.get("max_tokens", 8192)
        temperature = body.get("temperature")
        tools = body.get("tools")

        # 使用 AITACHI Cloud API
        if is_stream:
            return await _handle_stream_request(
                call_id, original_model, target_model,
                messages, system, max_tokens, temperature, tools,
                resp_headers
            )
        else:
            return await _handle_non_stream_request(
                call_id, original_model, target_model,
                messages, system, max_tokens, temperature, tools,
                resp_headers
            )


async def _handle_stream_request(
    call_id: int, original_model: str, target_model: str,
    messages: list, system: str, max_tokens: int, temperature: float, tools: list,
    resp_headers: dict
):
    """处理流式请求"""
    try:
        async def _gen():
            input_tokens = 0
            output_tokens = 0
            try:
                async for line in call_anthropic_api_stream(
                    messages=messages,
                    model=target_model,
                    max_tokens=max_tokens,
                    system=system,
                    temperature=temperature,
                    tools=tools
                ):
                    yield line
            except Exception as exc:
                logger.error(f"流式错误: {exc}\n{traceback.format_exc()}")
                end_call_recording(call_id, "error", error_message=str(exc))
            finally:
                end_call_recording(call_id, "success", input_tokens=input_tokens, output_tokens=output_tokens)

        return StreamingResponse(
            _gen(),
            media_type="text/event-stream",
            headers={**resp_headers, "Cache-Control": "no-cache", "Connection": "keep-alive"},
        )

    except Exception as e:
        end_call_recording(call_id, "error", error_message=str(e))
        logger.error(f"流式请求处理错误: {e}")
        return JSONResponse(
            status_code=500,
            content={"type": "error", "error": {"type": "api_error", "message": str(e)}},
            headers=resp_headers,
        )


async def _handle_non_stream_request(
    call_id: int, original_model: str, target_model: str,
    messages: list, system: str, max_tokens: int, temperature: float, tools: list,
    resp_headers: dict
):
    """处理非流式请求"""
    try:
        result = await call_anthropic_api(
            messages=messages,
            model=target_model,
            max_tokens=max_tokens,
            system=system,
            temperature=temperature,
            tools=tools
        )

        if "error" in result:
            err_msg = result.get("error", {}).get("message", "Unknown error")
            status_code = result.get("status_code", 500)
            logger.error(f"AITACHI API 错误: {err_msg}")
            end_call_recording(call_id, "error", error_message=err_msg)
            return JSONResponse(
                status_code=status_code,
                content={"type": "error", "error": {"type": "api_error", "message": err_msg}},
                headers=resp_headers,
            )

        # 转换响应格式
        anthropic_resp = _convert_to_claude_format(result, original_model)

        usage = anthropic_resp.get("usage", {})
        input_tokens = usage.get("input_tokens", 0)
        output_tokens = usage.get("output_tokens", 0)
        end_call_recording(call_id, "success", input_tokens=input_tokens, output_tokens=output_tokens)

        logger.info(f"响应 stop_reason={anthropic_resp.get('stop_reason')}")
        return JSONResponse(content=anthropic_resp, headers=resp_headers)

    except Exception as e:
        end_call_recording(call_id, "error", error_message=str(e))
        logger.error(f"请求处理错误: {e}")
        return JSONResponse(
            status_code=500,
            content={"type": "error", "error": {"type": "api_error", "message": str(e)}},
            headers=resp_headers,
        )


def _convert_to_claude_format(response: dict, original_model: str) -> dict:
    """将 BigModel API 响应转换为 Claude 格式"""
    if "error" in response:
        return {
            "type": "error",
            "error": response.get("error", {
                "type": "api_error",
                "message": "Unknown error"
            }),
        }

    # BigModel API 返回的是标准 Anthropic 格式
    # 只需要更新 model 字段和确保 usage 格式正确
    content = response.get("content", [])
    if not content:
        content = [{"type": "text", "text": ""}]

    usage = response.get("usage", {})

    return {
        "id": response.get("id", f"msg_{uuid.uuid4().hex[:24]}"),
        "type": response.get("type", "message"),
        "role": response.get("role", "assistant"),
        "content": content,
        "model": original_model,  # 使用请求时的模型名
        "stop_reason": response.get("stop_reason", "end_turn"),
        "stop_sequence": response.get("stop_sequence"),
        "usage": {
            "input_tokens": usage.get("input_tokens", 0),
            "output_tokens": usage.get("output_tokens", 0),
            "cache_creation_input_tokens": usage.get("cache_creation_input_tokens", 0),
            "cache_read_input_tokens": usage.get("cache_read_input_tokens", 0),
        },
    }


def _map_finish_reason(reason: Optional[str]) -> str:
    """映射结束原因"""
    if reason in ("tool_calls", "function_call"):
        return "tool_use"
    if reason == "length":
        return "max_tokens"
    return "end_turn"
