"""
Claude Code -> Qwen (DashScope) 中转代理服务器 v3.3

v3.3 修复:
  - 修复 /api/claude_code_penguin_mode 路径错误
  - 添加更多调试日志
"""

import json
import os
import sys
import time
import uuid
import secrets
import logging
import traceback
import ipaddress
import threading
import datetime
from typing import Optional, Union, List, Dict, Any

# Python 3.6 compatibility
timezone = datetime.timezone

import httpx
import uvicorn
from fastapi import FastAPI, Request
from fastapi.responses import (
    JSONResponse, StreamingResponse, RedirectResponse,
    HTMLResponse, PlainTextResponse,
)
from fastapi.middleware.cors import CORSMiddleware

# ============================================================
# 日志
# ============================================================
logging.basicConfig(
    level=logging.DEBUG,
    format="%(asctime)s [%(levelname)s] %(message)s",
)
logger = logging.getLogger("proxy")

# ============================================================
# 加载配置
# ============================================================
CONFIG_PATH = os.path.join(os.path.dirname(os.path.abspath(__file__)), "config.json")
with open(CONFIG_PATH, "r", encoding="utf-8") as _f:
    CONFIG = json.load(_f)

TARGET_BASE_URL       = CONFIG["api_base"].rstrip("/")
TARGET_API_KEY        = CONFIG["api_key"]
MODEL_MAP             = CONFIG.get("model_map", {})
DEFAULT_MODEL         = CONFIG.get("default_model", "qwen-coder-plus")
DEFAULT_PARAMS        = CONFIG.get("default_params", {})
DEFAULT_MAX_TOKENS    = CONFIG.get("default_max_tokens", 8192)
MAX_OUTPUT_TOKENS_LIMIT = CONFIG.get("max_output_tokens_limit", {})
HTTP_PORT             = CONFIG.get("http_port", 3001)
HTTPS_PORT            = CONFIG.get("https_port", 443)

logger.info(f"目标地址  : {TARGET_BASE_URL}")
logger.info(f"默认模型  : {DEFAULT_MODEL}")
logger.info(f"模型映射  : {MODEL_MAP}")
logger.info(f"输出token上限: {MAX_OUTPUT_TOKENS_LIMIT}")
logger.info(f"HTTP  端口 : {HTTP_PORT}")
logger.info(f"HTTPS 端口 : {HTTPS_PORT}")

# 伪造身份常量
FAKE_ACCOUNT_UUID = f"user_{uuid.uuid4().hex[:24]}"
FAKE_ORG_ID       = "org_proxy_default"
FAKE_EMAIL        = "proxy@local.dev"
FAKE_NAME         = "Proxy User"

# ============================================================
# FastAPI
# ============================================================
app = FastAPI(title="Claude-Code-Proxy")
app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_methods=["*"],
    allow_headers=["*"],
)


# 请求日志中间件
@app.middleware("http")
async def log_every_request(request: Request, call_next):
    method = request.method
    path   = request.url.path
    qs     = str(request.url.query)
    host   = request.headers.get("host", "?")
    auth   = request.headers.get("authorization", "")
    xkey   = request.headers.get("x-api-key", "")
    auth_short = (auth[:50] + "…") if len(auth) > 50 else auth
    xkey_short = (xkey[:50] + "…") if len(xkey) > 50 else xkey

    logger.info(
        f">>> {method} {path}{'?' + qs[:80] if qs else ''} "
        f"| Host={host} | Auth={auth_short} | x-api-key={xkey_short}"
    )

    response = await call_next(request)

    logger.info(f"<<< {method} {path} -> {response.status_code}")
    return response


# ============================================================
# 工具函数
# ============================================================
def gen_id(prefix: str = "msg") -> str:
    return f"{prefix}_{uuid.uuid4().hex[:24]}"


def gen_request_id() -> str:
    return f"req_{uuid.uuid4().hex}"


def map_model(name: str) -> str:
    if name in MODEL_MAP:
        return MODEL_MAP[name]
    if name and ("claude" in name.lower() or "sonnet" in name.lower()
                 or "opus" in name.lower() or "haiku" in name.lower()):
        logger.warning(f"模型 '{name}' 不在 model_map, 使用默认 '{DEFAULT_MODEL}'")
        return DEFAULT_MODEL
    logger.warning(f"未知模型 '{name}', 使用默认 '{DEFAULT_MODEL}'")
    return DEFAULT_MODEL


def clamp_max_tokens(requested: Optional[int], target_model: str) -> int:
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
    h = uuid.uuid4().hex + uuid.uuid4().hex + uuid.uuid4().hex
    return f"sk-ant-api03-{h[:40]}-{h[40:52]}-{h[52:76]}-AA"


def gen_oauth_code() -> str:
    return secrets.token_urlsafe(32)


def gen_oauth_token(prefix: str) -> str:
    return f"{prefix}-{secrets.token_hex(16)}-{secrets.token_hex(8)}"


def _user_profile() -> dict:
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
            "organization": _org_info(),
            "role": "owner",
        }],
    }


def _org_info() -> dict:
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


# ============================================================
# 请求转换  Anthropic -> OpenAI
# ============================================================
def _extract_text(content) -> str:
    if isinstance(content, str):
        return content
    if isinstance(content, list):
        parts = []
        for b in content:
            if isinstance(b, str):
                parts.append(b)
            elif isinstance(b, dict) and b.get("type") == "text":
                parts.append(b.get("text", ""))
        return "\n".join(parts)
    return str(content) if content else ""


def convert_messages(messages: list, system=None) -> List[Dict[str, Any]]:
    result: List[Dict[str, Any]] = []

    if system:
        sys_text = _extract_text(system)
        if sys_text:
            result.append({"role": "system", "content": sys_text})

    for msg in messages:
        role = msg.get("role", "user")
        content = msg.get("content") or ""

        if role == "user":
            if isinstance(content, list):
                text_parts: List[str] = []
                tool_results: List[Dict[str, Any]] = []

                for block in content:
                    if not isinstance(block, dict):
                        text_parts.append(str(block))
                        continue
                    btype = block.get("type", "")

                    if btype == "text":
                        text_parts.append(block.get("text", ""))
                    elif btype == "tool_result":
                        tc_content = block.get("content", "")
                        if isinstance(tc_content, list):
                            tc_content = "\n".join(
                                b.get("text", "")
                                for b in tc_content
                                if isinstance(b, dict) and b.get("type") == "text"
                            )
                        elif not isinstance(tc_content, str):
                            tc_content = (
                                json.dumps(tc_content, ensure_ascii=False)
                                if tc_content
                                else ""
                            )
                        if block.get("is_error"):
                            tc_content = f"[ERROR] {tc_content}"

                        tool_results.append({
                            "role": "tool",
                            "tool_call_id": block.get("tool_use_id", ""),
                            "content": tc_content,
                        })

                for tr in tool_results:
                    result.append(tr)
                combined = "\n".join(text_parts).strip()
                if combined:
                    result.append({"role": "user", "content": combined})
                if not tool_results and not combined:
                    result.append({"role": "user", "content": ""})
            else:
                result.append({"role": "user", "content": str(content)})

        elif role == "assistant":
            if isinstance(content, list):
                text_parts = []
                tool_calls = []
                for block in content:
                    if not isinstance(block, dict):
                        text_parts.append(str(block))
                        continue
                    btype = block.get("type", "")
                    if btype == "text":
                        text_parts.append(block.get("text", ""))
                    elif btype == "tool_use":
                        tool_calls.append({
                            "id": block.get("id", gen_id("call")),
                            "type": "function",
                            "function": {
                                "name": block.get("name", ""),
                                "arguments": json.dumps(
                                    block.get("input", {}), ensure_ascii=False
                                ),
                            },
                        })
                out: Dict[str, Any] = {"role": "assistant"}
                out["content"] = "\n".join(text_parts) if text_parts else None
                if tool_calls:
                    out["tool_calls"] = tool_calls
                result.append(out)
            else:
                result.append({"role": "assistant", "content": str(content)})

    return result


def convert_tools(tools: Optional[List]) -> Optional[List]:
    if not tools:
        return None
    out = []
    for t in tools:
        out.append({
            "type": "function",
            "function": {
                "name": t.get("name", ""),
                "description": t.get("description", ""),
                "parameters": t.get(
                    "input_schema", {"type": "object", "properties": {}}
                ),
            },
        })
    return out


def convert_tool_choice(tc) -> Optional[Union[str, Dict]]:
    if not tc or not isinstance(tc, dict):
        return None
    t = tc.get("type", "auto")
    if t == "auto":
        return "auto"
    if t == "any":
        return "required"
    if t == "tool":
        return {"type": "function", "function": {"name": tc.get("name", "")}}
    return "auto"


def build_openai_request(body: dict) -> dict:
    model = map_model(body.get("model", ""))
    messages = convert_messages(body.get("messages", []), body.get("system"))

    raw_max_tokens = body.get("max_tokens")
    clamped_max_tokens = clamp_max_tokens(raw_max_tokens, model)

    req: Dict[str, Any] = {
        "model": model,
        "messages": messages,
        "stream": body.get("stream", False),
        "max_tokens": clamped_max_tokens,
    }

    temp = body.get("temperature")
    if temp is not None:
        req["temperature"] = temp
    elif "temperature" in DEFAULT_PARAMS:
        req["temperature"] = DEFAULT_PARAMS["temperature"]

    if "top_p" in body:
        req["top_p"] = body["top_p"]

    tools = convert_tools(body.get("tools"))
    if tools:
        req["tools"] = tools
        tc = convert_tool_choice(body.get("tool_choice"))
        if tc:
            req["tool_choice"] = tc

    if body.get("stop_sequences"):
        req["stop"] = body["stop_sequences"]

    if req["stream"]:
        req["stream_options"] = {"include_usage": True}

    return req


# ============================================================
# 响应转换  OpenAI -> Anthropic
# ============================================================
def _map_finish(fr: Optional[str]) -> str:
    if fr in ("tool_calls", "function_call"):
        return "tool_use"
    if fr == "length":
        return "max_tokens"
    return "end_turn"


def build_anthropic_response(openai_resp: dict, original_model: str) -> dict:
    if "error" in openai_resp:
        return {
            "type": "error",
            "error": {
                "type": "api_error",
                "message": openai_resp["error"].get("message", "Unknown error"),
            },
        }
    choices = openai_resp.get("choices", [])
    if not choices:
        return {
            "type": "error",
            "error": {"type": "api_error", "message": "No choices in response"},
        }

    message = choices[0].get("message", {})
    content: List[Dict[str, Any]] = []

    if message.get("content"):
        content.append({"type": "text", "text": message["content"]})

    if message.get("tool_calls"):
        for tc in message["tool_calls"]:
            try:
                args = json.loads(tc["function"]["arguments"])
            except (json.JSONDecodeError, KeyError, TypeError):
                args = {}
            content.append({
                "type": "tool_use",
                "id": tc.get("id", gen_id("toolu")),
                "name": tc["function"]["name"],
                "input": args,
            })
    if not content:
        content.append({"type": "text", "text": ""})

    usage = openai_resp.get("usage", {})

    return {
        "id": gen_id("msg"),
        "type": "message",
        "role": "assistant",
        "content": content,
        "model": original_model,
        "stop_reason": _map_finish(choices[0].get("finish_reason")),
        "stop_sequence": None,
        "usage": {
            "input_tokens": usage.get("prompt_tokens", 0),
            "output_tokens": usage.get("completion_tokens", 0),
            "cache_creation_input_tokens": 0,
            "cache_read_input_tokens": 0,
        },
    }


# ============================================================
# 流式 SSE 转换
# ============================================================
async def stream_sse(response: httpx.Response, original_model: str):
    msg_id = gen_id("msg")

    ms = {
        "type": "message_start",
        "message": {
            "id": msg_id,
            "type": "message",
            "role": "assistant",
            "content": [],
            "model": original_model,
            "stop_reason": None,
            "stop_sequence": None,
            "usage": {
                "input_tokens": 0,
                "output_tokens": 0,
                "cache_creation_input_tokens": 0,
                "cache_read_input_tokens": 0,
            },
        },
    }
    yield f"event: message_start\ndata: {json.dumps(ms)}\n\n"
    yield 'event: ping\ndata: {"type": "ping"}\n\n'

    text_open = False
    text_idx = -1
    next_idx = 0
    tool_blocks: Dict[int, int] = {}
    closed: set = set()
    finish_reason_received: Optional[str] = None
    input_tokens = 0
    output_tokens = 0
    chunk_count = 0

    async for raw_line in response.aiter_lines():
        line = raw_line.strip()
        if not line:
            continue
        if not line.startswith("data:"):
            continue

        if line.startswith("data: "):
            payload = line[6:]
        else:
            payload = line[5:]

        payload = payload.strip()
        if payload == "[DONE]":
            break

        try:
            chunk = json.loads(payload)
        except json.JSONDecodeError:
            continue

        chunk_count += 1
        if chunk_count <= 3:
            logger.debug(f"SSE<<< chunk#{chunk_count}: {json.dumps(chunk, ensure_ascii=False)[:300]}")

        if "error" in chunk and not chunk.get("choices"):
            err_msg = chunk["error"].get("message", "Unknown upstream error")
            logger.error(f"SSE<<< 上游流内错误: {err_msg}")
            if not text_open:
                text_idx = next_idx
                next_idx += 1
                bs = {
                    "type": "content_block_start",
                    "index": text_idx,
                    "content_block": {"type": "text", "text": ""},
                }
                yield f"event: content_block_start\ndata: {json.dumps(bs)}\n\n"
                text_open = True
            td = {
                "type": "content_block_delta",
                "index": text_idx,
                "delta": {"type": "text_delta", "text": f"[Proxy Error] {err_msg}"},
            }
            yield f"event: content_block_delta\ndata: {json.dumps(td)}\n\n"
            continue

        choices = chunk.get("choices", [])

        if not choices:
            u = chunk.get("usage")
            if u:
                input_tokens = u.get("prompt_tokens", input_tokens)
                output_tokens = u.get("completion_tokens", output_tokens)
            continue

        delta = choices[0].get("delta", {})
        finish_reason = choices[0].get("finish_reason")

        text_piece = delta.get("content")
        if not text_piece:
            text_piece = delta.get("reasoning_content")

        if text_piece:
            if not text_open:
                text_idx = next_idx
                next_idx += 1
                bs = {
                    "type": "content_block_start",
                    "index": text_idx,
                    "content_block": {"type": "text", "text": ""},
                }
                yield f"event: content_block_start\ndata: {json.dumps(bs)}\n\n"
                text_open = True

            td = {
                "type": "content_block_delta",
                "index": text_idx,
                "delta": {"type": "text_delta", "text": text_piece},
            }
            yield f"event: content_block_delta\ndata: {json.dumps(td)}\n\n"

        if delta.get("tool_calls"):
            for tc in delta["tool_calls"]:
                tc_i = tc.get("index", 0)

                if tc_i not in tool_blocks:
                    if text_open and text_idx not in closed:
                        yield f'event: content_block_stop\ndata: {{"type":"content_block_stop","index":{text_idx}}}\n\n'
                        closed.add(text_idx)
                        text_open = False

                    bi = next_idx
                    next_idx += 1
                    tool_id = tc.get("id", gen_id("toolu"))
                    tool_name = tc.get("function", {}).get("name", "")
                    tbs = {
                        "type": "content_block_start",
                        "index": bi,
                        "content_block": {
                            "type": "tool_use",
                            "id": tool_id,
                            "name": tool_name,
                            "input": {},
                        },
                    }
                    yield f"event: content_block_start\ndata: {json.dumps(tbs)}\n\n"
                    tool_blocks[tc_i] = bi

                arg_chunk = tc.get("function", {}).get("arguments", "")
                if arg_chunk:
                    bi = tool_blocks[tc_i]
                    ijd = {
                        "type": "content_block_delta",
                        "index": bi,
                        "delta": {
                            "type": "input_json_delta",
                            "partial_json": arg_chunk,
                        },
                    }
                    yield f"event: content_block_delta\ndata: {json.dumps(ijd)}\n\n"

        if finish_reason:
            finish_reason_received = finish_reason
            u = chunk.get("usage")
            if u:
                input_tokens = u.get("prompt_tokens", input_tokens)
                output_tokens = u.get("completion_tokens", output_tokens)

    if next_idx == 0:
        bs = {
            "type": "content_block_start",
            "index": 0,
            "content_block": {"type": "text", "text": ""},
        }
        yield f"event: content_block_start\ndata: {json.dumps(bs)}\n\n"
        yield 'event: content_block_stop\ndata: {"type":"content_block_stop","index":0}\n\n'
    else:
        if text_open and text_idx not in closed:
            yield f'event: content_block_stop\ndata: {{"type":"content_block_stop","index":{text_idx}}}\n\n'
            closed.add(text_idx)
        for _, bi in sorted(tool_blocks.items()):
            if bi not in closed:
                yield f'event: content_block_stop\ndata: {{"type":"content_block_stop","index":{bi}}}\n\n'
                closed.add(bi)

    stop_reason = _map_finish(finish_reason_received) if finish_reason_received else "end_turn"

    md = {
        "type": "message_delta",
        "delta": {"stop_reason": stop_reason, "stop_sequence": None},
        "usage": {"output_tokens": output_tokens},
    }
    yield f"event: message_delta\ndata: {json.dumps(md)}\n\n"
    yield 'event: message_stop\ndata: {"type": "message_stop"}\n\n'


# ============================================================
# 核心路由: POST /v1/messages
# ============================================================
@app.post("/v1/messages")
async def handle_messages(request: Request):
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
    logger.info(
        f"转发 -> {openai_body['model']}, max_tokens={openai_body['max_tokens']}, "
        f"目标: {TARGET_BASE_URL}/chat/completions"
    )

    headers = {
        "Authorization": f"Bearer {TARGET_API_KEY}",
        "Content-Type": "application/json",
    }
    target_url = f"{TARGET_BASE_URL}/chat/completions"
    resp_headers = make_anthropic_headers()

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
                return JSONResponse(
                    status_code=resp.status_code,
                    content={"type": "error", "error": {"type": "api_error", "message": err_msg}},
                    headers=resp_headers,
                )

            async def _gen():
                try:
                    async for chunk in stream_sse(resp, original_model):
                        yield chunk
                except Exception as exc:
                    logger.error(f"流式错误: {exc}\n{traceback.format_exc()}")
                finally:
                    await resp.aclose()
                    await client.aclose()

            return StreamingResponse(
                _gen(),
                media_type="text/event-stream",
                headers={**resp_headers, "Cache-Control": "no-cache", "Connection": "keep-alive"},
            )

        except httpx.ConnectError as e:
            logger.error(f"连接失败: {e}")
            return JSONResponse(
                status_code=502,
                content={"type": "error", "error": {"type": "api_error", "message": f"Connect failed: {e}"}},
                headers=resp_headers,
            )

    try:
        async with httpx.AsyncClient(timeout=httpx.Timeout(300.0, connect=30.0)) as client:
            resp = await client.post(target_url, json=openai_body, headers=headers)

            if resp.status_code != 200:
                try:
                    err_msg = resp.json().get("error", {}).get("message", resp.text)
                except Exception:
                    err_msg = resp.text
                logger.error(f"上游错误 {resp.status_code}: {err_msg}")
                return JSONResponse(
                    status_code=resp.status_code,
                    content={"type": "error", "error": {"type": "api_error", "message": err_msg}},
                    headers=resp_headers,
                )

            anthropic_resp = build_anthropic_response(resp.json(), original_model)
            logger.info(f"响应 stop_reason={anthropic_resp.get('stop_reason')}")
            return JSONResponse(content=anthropic_resp, headers=resp_headers)

    except httpx.ConnectError as e:
        return JSONResponse(
            status_code=502,
            content={"type": "error", "error": {"type": "api_error", "message": f"Connect failed: {e}"}},
            headers=resp_headers,
        )


# ============================================================
# Claude Code 启动检查
# ============================================================

@app.get("/")
async def health():
    return {"status": "ok", "proxy": "claude-code-to-qwen"}


@app.get("/v1/models")
async def list_models():
    models = []
    for model_name in MODEL_MAP:
        models.append({
            "id": model_name,
            "object": "model",
            "created": int(time.time()) - 86400,
            "owned_by": "anthropic",
        })
    return {"object": "list", "data": models}


@app.get("/v1/models/{model_id:path}")
async def get_model(model_id: str):
    return {
        "id": model_id,
        "object": "model",
        "created": int(time.time()) - 86400,
        "owned_by": "anthropic",
    }


@app.api_route("/v1/organizations", methods=["GET", "POST"])
async def handle_organizations(request: Request):
    return JSONResponse(
        content={"data": [_org_info()]},
        headers=make_anthropic_headers(),
    )


# ============================================================
# ★★★ 核心修复: 平台健康检查 — 必须匹配真实 API 格式 ★★★
# ============================================================

@app.get("/api/hello")
async def api_hello(request: Request):
    """真实 API 返回 {"message": "hello"}, 必须一模一样"""
    logger.info("[Platform] GET /api/hello -> {\"message\": \"hello\"} ✓")
    return JSONResponse(content={"message": "hello"})


@app.get("/v1/oauth/hello")
async def oauth_hello(request: Request):
    """真实 API 返回 {"message":"hello"}, 必须一模一样"""
    logger.info("[Platform] GET /v1/oauth/hello -> {\"message\":\"hello\"} ✓")
    return JSONResponse(content={"message": "hello"})


# ============================================================
# ★★★ 核心修复: Claude Code 专用端点 ★★★
# ============================================================

@app.get("/api/claude_code/settings")
async def claude_code_settings(request: Request):
    """Claude Code v2.1.39 启动时请求此端点获取设置"""
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
    """Claude Code v2.1.39 启动时请求此端点获取策略限制"""
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


# ★★★ 修复路径错误：penguin_mode 前缀应该是 /api/claude_code/ ★★★
@app.get("/api/claude_code/penguin_mode")
async def claude_code_penguin_mode(request: Request):
    """Claude Code v2.1.39 启动时请求此端点"""
    logger.info("[Claude Code] GET /api/claude_code/penguin_mode ✓")
    return JSONResponse(content={
        "enabled": False,
    })


# ============================================================
# 平台 API 端点
# ============================================================

@app.get("/api/bootstrap")
async def api_bootstrap(request: Request):
    logger.info("[Platform] GET /api/bootstrap")
    return JSONResponse(content={
        "account": _user_profile(),
        "organizations": [_org_info()],
        "statsig": {},
    })


@app.get("/api/auth")
async def api_auth(request: Request):
    logger.info("[Platform] GET /api/auth")
    return JSONResponse(content={
        "account": _user_profile(),
        "account_flags": [],
    })


@app.get("/api/auth/session")
async def api_auth_session(request: Request):
    logger.info("[Platform] GET /api/auth/session")
    return JSONResponse(content={
        "account": _user_profile(),
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
    return JSONResponse(content=_user_profile())


@app.get("/api/settings")
async def api_settings(request: Request):
    logger.info("[Platform] GET /api/settings")
    return JSONResponse(content={})


@app.get("/api/organizations")
async def api_organizations(request: Request):
    logger.info("[Platform] GET /api/organizations")
    return JSONResponse(content={"data": [_org_info()]})


@app.post("/api/organizations/{org_id}/api_keys")
async def api_create_key_no_prefix(request: Request, org_id: str):
    return await _handle_create_api_key(request, org_id)


@app.post("/api/organizations/{org_id}/api-keys")
async def api_create_key_hyphen(request: Request, org_id: str):
    return await _handle_create_api_key(request, org_id)


async def _handle_create_api_key(request: Request, org_id: str):
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
            "id": gen_id("apikey"),
            "type": "api_key",
            "api_key": fake_key,
            "name": key_name,
            "created_at": int(time.time()),
            "status": "active",
        },
        headers=make_anthropic_headers(),
    )


# 遥测/事件沉默端点
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


# ============================================================
# OAuth 认证端点
# ============================================================

@app.get("/oauth/authorize")
async def oauth_authorize(request: Request):
    state = request.query_params.get("state", "")
    redirect_uri = request.query_params.get("redirect_uri", "")
    fake_code = gen_oauth_code()
    logger.info(f"[OAuth] 授权请求, state={state[:30]}…, 生成 code={fake_code[:20]}…")

    if redirect_uri:
        separator = "&" if "?" in redirect_uri else "?"
        redirect_url = f"{redirect_uri}{separator}code={fake_code}&state={state}"
        return RedirectResponse(url=redirect_url, status_code=302)

    return HTMLResponse(content=_build_code_page(fake_code))


@app.get("/oauth/code/callback")
async def oauth_code_callback(request: Request):
    code = request.query_params.get("code", gen_oauth_code())
    logger.info(f"[OAuth] 回调页面, code={code[:20]}…")
    return HTMLResponse(content=_build_code_page(code))


@app.get("/generate-code")
async def generate_code_endpoint(request: Request):
    code = gen_oauth_code()
    logger.info(f"[OAuth] /generate-code -> {code}")
    return PlainTextResponse(code)


def _build_code_page(code: str) -> str:
    return f"""<!DOCTYPE html>
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
</html>"""


@app.post("/oauth/token")
async def oauth_token(request: Request):
    logger.info("[OAuth] Token 交换请求")
    try:
        content_type = request.headers.get("content-type", "")
        if "form" in content_type:
            form = await request.form()
            grant_type = form.get("grant_type", "authorization_code")
            code = form.get("code", "?")
            logger.info(f"[OAuth] grant_type={grant_type} (form) code={str(code)[:20]}…")
        else:
            body = await request.json()
            grant_type = body.get("grant_type", "authorization_code")
            code = body.get("code", "?")
            logger.info(f"[OAuth] grant_type={grant_type} (json) code={str(code)[:20]}…")
    except Exception:
        grant_type = "authorization_code"

    access_token  = gen_oauth_token("sk-ant-sid01")
    refresh_token = gen_oauth_token("sk-ant-rt01")

    resp_data = {
        "access_token": access_token,
        "token_type": "Bearer",
        "expires_in": 31536000,
        "refresh_token": refresh_token,
        "scope": "org:create_api_key user:profile user:inference user:sessions:claude_code user:mcp_servers",
        "account_uuid": FAKE_ACCOUNT_UUID,
    }
    logger.info(f"[OAuth] 返回 access_token={access_token[:35]}…")
    return JSONResponse(content=resp_data)


# ============================================================
# 用户/会话/API Key 端点
# ============================================================

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
    return JSONResponse(content=_user_profile())


@app.api_route("/api/me", methods=["GET", "POST"])
async def get_api_me(request: Request):
    return JSONResponse(content=_user_profile())


@app.api_route("/v1/me", methods=["GET", "POST"])
async def get_v1_me(request: Request):
    return JSONResponse(content=_user_profile())


@app.post("/v1/organizations/{org_id}/api_keys")
async def create_org_api_key(org_id: str, request: Request):
    return await _handle_create_api_key(request, org_id)


@app.api_route("/v1/organizations/{org_id:path}", methods=["GET", "POST"])
async def handle_organization_detail(request: Request, org_id: str):
    return JSONResponse(
        content=_org_info(),
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
    body = await request.json()
    body["messages"] = [{"role": "user", "content": body.get("prompt", "")}]
    body.setdefault("stream", False)
    scope = request.scope.copy()
    new_request = Request(scope, request.receive)
    new_request._json = body
    return await handle_messages(new_request)


# ============================================================
# 测试端点
# ============================================================
@app.get("/test")
async def test_endpoint():
    test_body = {
        "model": DEFAULT_MODEL,
        "messages": [{"role": "user", "content": "say hello"}],
        "max_tokens": 50,
        "stream": False,
    }
    headers = {
        "Authorization": f"Bearer {TARGET_API_KEY}",
        "Content-Type": "application/json",
    }
    target_url = f"{TARGET_BASE_URL}/chat/completions"
    try:
        async with httpx.AsyncClient(timeout=httpx.Timeout(30.0)) as client:
            resp = await client.post(target_url, json=test_body, headers=headers)
            return {
                "status": resp.status_code,
                "upstream_ok": resp.status_code == 200,
                "proxy_config": {
                    "target": TARGET_BASE_URL,
                    "default_model": DEFAULT_MODEL,
                    "max_output_tokens_limit": MAX_OUTPUT_TOKENS_LIMIT,
                },
            }
    except Exception as e:
        return {"error": str(e)}


@app.get("/test-anthropic")
async def test_anthropic():
    test_body = {
        "model": "claude-sonnet-4-5-20250929",
        "max_tokens": 128000,
        "messages": [{"role": "user", "content": "1+1=?"}],
        "stream": False,
    }
    try:
        async with httpx.AsyncClient(timeout=httpx.Timeout(30.0)) as client:
            resp = await client.post(
                f"http://127.0.0.1:{HTTP_PORT}/v1/messages",
                json=test_body,
                headers={
                    "x-api-key": "sk-placeholder",
                    "anthropic-version": "2023-06-01",
                    "Content-Type": "application/json",
                },
            )
            return {"status": resp.status_code, "response": resp.json()}
    except Exception as e:
        return {"error": str(e)}


# ============================================================
# 通配符路由 (必须放最后)
# ============================================================
@app.api_route(
    "/{path:path}",
    methods=["GET", "POST", "PUT", "DELETE", "PATCH", "OPTIONS", "HEAD"],
)
async def catch_all(request: Request, path: str):
    method = request.method
    host   = request.headers.get("host", "?")
    auth   = request.headers.get("authorization", "")[:40]
    xkey   = request.headers.get("x-api-key", "")[:40]

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


# ============================================================
# SSL 自签名证书
# ============================================================
def ensure_ssl_cert():
    ssl_dir = os.path.join(os.path.dirname(os.path.abspath(__file__)), "ssl")
    cert_file = os.path.join(ssl_dir, "cert.pem")
    key_file = os.path.join(ssl_dir, "key.pem")

    if os.path.exists(cert_file) and os.path.exists(key_file):
        logger.info(f"SSL 证书已存在: {ssl_dir}")
        return cert_file, key_file

    os.makedirs(ssl_dir, exist_ok=True)
    logger.info("正在生成自签名 SSL 证书 ...")

    try:
        from cryptography import x509
        from cryptography.x509.oid import NameOID
        from cryptography.hazmat.primitives import hashes, serialization
        from cryptography.hazmat.primitives.asymmetric import rsa

        key = rsa.generate_private_key(public_exponent=65537, key_size=2048)

        subject = issuer = x509.Name([
            x509.NameAttribute(NameOID.COMMON_NAME, "api.anthropic.com"),
            x509.NameAttribute(NameOID.ORGANIZATION_NAME, "Claude Proxy"),
        ])

        now = datetime.datetime.now(timezone.utc)
        cert = (
            x509.CertificateBuilder()
            .subject_name(subject)
            .issuer_name(issuer)
            .public_key(key.public_key())
            .serial_number(x509.random_serial_number())
            .not_valid_before(now)
            .not_valid_after(now + datetime.timedelta(days=3650))
            .add_extension(
                x509.SubjectAlternativeName([
                    x509.DNSName("api.anthropic.com"),
                    x509.DNSName("*.anthropic.com"),
                    x509.DNSName("platform.claude.com"),
                    x509.DNSName("*.claude.com"),
                    x509.DNSName("localhost"),
                    x509.IPAddress(ipaddress.IPv4Address("127.0.0.1")),
                    x509.IPAddress(ipaddress.IPv4Address("0.0.0.0")),
                ]),
                critical=False,
            )
            .sign(key, hashes.SHA256())
        )

        with open(key_file, "wb") as f:
            f.write(key.private_bytes(
                encoding=serialization.Encoding.PEM,
                format=serialization.PrivateFormat.TraditionalOpenSSL,
                encryption_algorithm=serialization.NoEncryption(),
            ))

        with open(cert_file, "wb") as f:
            f.write(cert.public_bytes(serialization.Encoding.PEM))

        logger.info(f"SSL 证书已生成: {cert_file}")
        return cert_file, key_file

    except ImportError:
        logger.error("缺少 cryptography 库! pip install cryptography")
        sys.exit(1)


# ============================================================
# 启动入口
# ============================================================
if __name__ == "__main__":
    cert_file, key_file = ensure_ssl_cert()

    print("=" * 60)
    print("  Claude Code Proxy v3.3")
    print(f"  HTTP  -> http://0.0.0.0:{HTTP_PORT}")
    print(f"  HTTPS -> https://0.0.0.0:{HTTPS_PORT}")
    print("=" * 60)

    def start_http():
        uvicorn.run(app, host="0.0.0.0", port=HTTP_PORT, log_level="info")

    http_thread = threading.Thread(target=start_http, daemon=True)
    http_thread.start()
    logger.info(f"HTTP  代理已启动 -> http://0.0.0.0:{HTTP_PORT}")

    logger.info(f"HTTPS 代理启动中 -> https://0.0.0.0:{HTTPS_PORT}")
    try:
        uvicorn.run(
            app,
            host="0.0.0.0",
            port=HTTPS_PORT,
            ssl_certfile=cert_file,
            ssl_keyfile=key_file,
            log_level="info",
        )
    except PermissionError:
        logger.error(f"端口 {HTTPS_PORT} 需要管理员权限!")
        sys.exit(1)
    except OSError as e:
        if "address already in use" in str(e).lower() or "10048" in str(e):
            logger.error(f"端口 {HTTPS_PORT} 已被占用!")
        else:
            logger.error(f"启动失败: {e}")
        sys.exit(1)
