"""
配置加载和常量定义
"""

import os
import json
import uuid
import logging

# 日志配置
logging.basicConfig(
    level=logging.DEBUG,
    format="%(asctime)s [%(levelname)s] %(message)s",
)
logger = logging.getLogger("proxy")

# 加载配置文件
CONFIG_PATH = os.path.join(os.path.dirname(os.path.abspath(__file__)), "config.json")
with open(CONFIG_PATH, "r", encoding="utf-8") as _f:
    CONFIG = json.load(_f)

# BigModel API 配置 (AITACHI Cloud 原生协议)
API_BASE_URL = CONFIG["api_base"].rstrip("/")
API_KEY = CONFIG["api_key"]

# Anthropic 环境变量配置 (用于 AITACHI Cloud 原生协议)
ANTHROPIC_ENV = CONFIG.get("anthropic_env", {})
ENABLED_PLUGINS = CONFIG.get("enabled_plugins", {})

# 模型映射配置
MODEL_MAP = CONFIG.get("model_map", {})
DEFAULT_MODEL = CONFIG.get("default_model", "opus")
DEFAULT_PARAMS = CONFIG.get("default_params", {})
DEFAULT_MAX_TOKENS = CONFIG.get("default_max_tokens", 8192)
MAX_OUTPUT_TOKENS_LIMIT = CONFIG.get("max_output_tokens_limit", {})

# 端口配置
HTTP_PORT = CONFIG.get("http_port", 3001)
HTTPS_PORT = CONFIG.get("https_port", 443)

# 伪造身份常量
FAKE_ACCOUNT_UUID = f"user_{uuid.uuid4().hex[:24]}"
FAKE_ORG_ID = "org_proxy_default"
FAKE_EMAIL = "proxy@local.dev"
FAKE_NAME = "Proxy User"

# 日志输出
logger.info(f"=" * 60)
logger.info(f"  Claude Code Proxy v4.1 - AITACHI Cloud")
logger.info(f"=" * 60)
logger.info(f"API Base URL : {API_BASE_URL}")
logger.info(f"默认模型     : {DEFAULT_MODEL}")
logger.info(f"模型映射     : {MODEL_MAP}")
logger.info(f"输出token上限: {MAX_OUTPUT_TOKENS_LIMIT}")
logger.info(f"HTTP  端口   : {HTTP_PORT}")
logger.info(f"HTTPS 端口   : {HTTPS_PORT}")
logger.info(f"=" * 60)
