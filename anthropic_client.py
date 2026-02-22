"""
BigModel API 客户端 - AITACHI Cloud 原生协议
使用带 env 和 enabledPlugins 的格式调用 BigModel API
"""

import json
import logging
import httpx
from typing import Optional, List, Dict, Any

from config import logger, API_BASE_URL, API_KEY, ANTHROPIC_ENV, ENABLED_PLUGINS


async def call_anthropic_api(
    messages: List[Dict],
    model: str = "opus[1m]",
    max_tokens: int = 8192,
    stream: bool = False,
    tools: Optional[List] = None,
    system: Optional[str] = None,
    temperature: Optional[float] = None,
    **kwargs
) -> Dict:
    """
    使用 AITACHI Cloud 原生协议调用 BigModel API

    请求格式:
    POST https://open.bigmodel.cn/api/anthropic/v1/messages
    {
        "env": {
            "ANTHROPIC_AUTH_TOKEN": "...",
            "ANTHROPIC_BASE_URL": "https://open.bigmodel.cn/api/anthropic",
            ...
        },
        "model": "opus[1m]",
        "messages": [...],
        "max_tokens": 8192,
        "enabledPlugins": {
            "glm-plan-bug@zai-coding-plugins": true,
            "glm-plan-usage@zai-coding-plugins": true
        }
    }
    """

    headers = {
        "Content-Type": "application/json",
        "Authorization": f"Bearer {API_KEY}"
    }

    # 构建 AITACHI Cloud 原生格式请求
    request_body = {
        "env": ANTHROPIC_ENV,
        "model": model,
        "messages": messages,
        "max_tokens": max_tokens,
        "enabledPlugins": ENABLED_PLUGINS
    }

    # 添加可选参数
    if system:
        request_body["system"] = system
    if temperature is not None:
        request_body["temperature"] = temperature
    if tools:
        request_body["tools"] = tools
    if stream:
        request_body["stream"] = True

    # 添加其他 kwargs
    for key, value in kwargs.items():
        if value is not None:
            request_body[key] = value

    url = f"{API_BASE_URL}/v1/messages"

    logger.info(f"[BigModel API] Calling {url} with model={model}")
    logger.debug(f"[BigModel API] Request body keys: {list(request_body.keys())}")

    try:
        async with httpx.AsyncClient(timeout=httpx.Timeout(300.0, connect=30.0), verify=False) as client:
            response = await client.post(url, json=request_body, headers=headers)

            if response.status_code != 200:
                error_text = response.text
                try:
                    error_json = response.json()
                    error_msg = error_json.get("error", {}).get("message", error_text)
                except:
                    error_msg = error_text
                logger.error(f"[BigModel API] Error {response.status_code}: {error_msg}")
                return {
                    "error": {
                        "type": "api_error",
                        "message": error_msg
                    },
                    "status_code": response.status_code
                }

            return response.json()

    except httpx.ConnectError as e:
        logger.error(f"[BigModel API] Connection failed: {e}")
        return {
            "error": {
                "type": "connection_error",
                "message": f"Connection failed: {e}"
            }
        }
    except Exception as e:
        logger.error(f"[BigModel API] Unexpected error: {e}")
        return {
            "error": {
                "type": "unknown_error",
                "message": str(e)
            }
        }


async def call_anthropic_api_stream(
    messages: List[Dict],
    model: str = "opus[1m]",
    max_tokens: int = 8192,
    tools: Optional[List] = None,
    system: Optional[str] = None,
    temperature: Optional[float] = None,
    **kwargs
):
    """
    使用 AITACHI Cloud 原生协议流式调用

    返回异步生成器，产生 SSE 事件
    """

    headers = {
        "Content-Type": "application/json",
        "Authorization": f"Bearer {API_KEY}"
    }

    # 构建 AITACHI Cloud 原生格式请求
    request_body = {
        "env": ANTHROPIC_ENV,
        "model": model,
        "messages": messages,
        "max_tokens": max_tokens,
        "stream": True,
        "enabledPlugins": ENABLED_PLUGINS
    }

    if system:
        request_body["system"] = system
    if temperature is not None:
        request_body["temperature"] = temperature
    if tools:
        request_body["tools"] = tools

    for key, value in kwargs.items():
        if value is not None:
            request_body[key] = value

    url = f"{API_BASE_URL}/v1/messages"

    logger.info(f"[BigModel API Stream] Calling {url} with model={model}")

    client = httpx.AsyncClient(timeout=httpx.Timeout(300.0, connect=30.0), verify=False)

    try:
        async with client.stream("POST", url, json=request_body, headers=headers) as response:
            if response.status_code != 200:
                error_text = await response.aread()
                try:
                    error_json = json.loads(error_text)
                    error_msg = error_json.get("error", {}).get("message", error_text.decode())
                except:
                    error_msg = error_text.decode("utf-8", errors="replace")
                logger.error(f"[BigModel API Stream] Error {response.status_code}: {error_msg}")
                yield f"event: error\ndata: {json.dumps({'error': error_msg})}\n\n"
                return

            async for line in response.aiter_lines():
                if line.strip():
                    yield line + "\n"

    except httpx.ConnectError as e:
        logger.error(f"[BigModel API Stream] Connection failed: {e}")
        yield f"event: error\ndata: {json.dumps({'error': f'Connection failed: {e}'})}\n\n"
    except Exception as e:
        logger.error(f"[BigModel API Stream] Unexpected error: {e}")
        yield f"event: error\ndata: {json.dumps({'error': str(e)})}\n\n"
    finally:
        await client.aclose()


def map_to_api_model(original_model: str) -> str:
    """
    将原始模型名称映射到 API 模型名称
    所有模型都映射到 opus[1m]
    """
    from config import MODEL_MAP, DEFAULT_MODEL

    if original_model in MODEL_MAP:
        return MODEL_MAP[original_model]
    return DEFAULT_MODEL
