"""
OAuth认证端点
"""

from fastapi import Request
from fastapi.responses import JSONResponse, RedirectResponse, HTMLResponse, PlainTextResponse

from config import logger, FAKE_ACCOUNT_UUID
from utils import gen_oauth_code, gen_oauth_token, user_profile, org_info, make_anthropic_headers


def register_oauth_routes(app):
    """注册OAuth路由"""

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

        access_token = gen_oauth_token("sk-ant-sid01")
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


def _build_code_page(code: str) -> str:
    """构建授权码显示页面"""
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
