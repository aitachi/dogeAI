"""
基础路由：健康检查、模型列表、测试端点
"""

import time
import httpx
from fastapi import Request
from fastapi.responses import JSONResponse

from config import logger, MODEL_MAP, TARGET_BASE_URL, TARGET_API_KEY, HTTP_PORT
from utils import make_anthropic_headers


def register_base_routes(app):
    """注册基础路由"""

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
        from utils import org_info
        return JSONResponse(
            content={"data": [org_info()]},
            headers=make_anthropic_headers(),
        )

    @app.get("/test")
    async def test_endpoint():
        from config import DEFAULT_MODEL, MAX_OUTPUT_TOKENS_LIMIT
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
