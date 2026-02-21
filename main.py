"""
Claude Code -> Qwen (DashScope) 中转代理服务器 v3.3

v3.3 修复:
  - 修复 /api/claude_code_penguin_mode 路径错误
  - 添加更多调试日志
  - 重构为模块化架构
"""

import sys
import threading
import logging
from fastapi import FastAPI, Request
from fastapi.middleware.cors import CORSMiddleware

# 导入配置和工具
from config import logger, HTTP_PORT, HTTPS_PORT
from ssl_helper import ensure_ssl_cert
from routes import register_all_routes

# 创建FastAPI应用
app = FastAPI(title="Claude-Code-Proxy")

# 添加CORS中间件
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
    path = request.url.path
    qs = str(request.url.query)
    host = request.headers.get("host", "?")
    auth = request.headers.get("authorization", "")
    xkey = request.headers.get("x-api-key", "")
    auth_short = (auth[:50] + "…") if len(auth) > 50 else auth
    xkey_short = (xkey[:50] + "…") if len(xkey) > 50 else xkey

    logger.info(
        f">>> {method} {path}{'?' + qs[:80] if qs else ''} "
        f"| Host={host} | Auth={auth_short} | x-api-key={xkey_short}"
    )

    response = await call_next(request)

    logger.info(f"<<< {method} {path} -> {response.status_code}")
    return response


# 注册所有路由
register_all_routes(app)


# ============================================================
# 启动入口
# ============================================================
if __name__ == "__main__":
    import uvicorn

    cert_file, key_file = ensure_ssl_cert()

    print("=" * 60)
    print("  Claude Code Proxy v3.3 (Modular)")
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
