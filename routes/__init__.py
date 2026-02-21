"""
路由模块初始化
"""
from fastapi import FastAPI
from .base import register_base_routes
from .api import register_api_routes
from .oauth import register_oauth_routes
from .claude import register_claude_routes, register_messages_handler


def register_all_routes(app: FastAPI):
    """注册所有路由"""
    register_base_routes(app)
    register_api_routes(app)
    register_oauth_routes(app)
    register_messages_handler(app)  # 必须在 register_claude_routes 之前！
    register_claude_routes(app)      # 包含通配符路由，必须最后注册
