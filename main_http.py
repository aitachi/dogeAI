"""
Claude Code Proxy - HTTP Only Mode (用于systemd)
端口443由nginx处理，本服务仅提供HTTP
"""
import sys
import logging
from fastapi import FastAPI, Request
from fastapi.middleware.cors import CORSMiddleware
import uvicorn

# 导入配置和工具
from config import logger, HTTP_PORT
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

if __name__ == "__main__":
    print("=" * 60)
    print("  Claude Code Proxy v3.3 (HTTP Only)")
    print(f"  HTTP  -> http://0.0.0.0:{HTTP_PORT}")
    print(f"  HTTPS -> 由nginx在443端口代理")
    print("=" * 60)
    
    logger.info(f"HTTP 代理启动 -> http://0.0.0.0:{HTTP_PORT}")
    
    try:
        uvicorn.run(
            app,
            host="0.0.0.0",
            port=HTTP_PORT,
            log_level="info",
        )
    except Exception as e:
        logger.error(f"启动失败: {e}")
        sys.exit(1)
