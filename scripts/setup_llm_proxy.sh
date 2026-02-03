#!/bin/bash
# ========================================
# 大模型API代理服务器 - 自动安装脚本
# 适用于: Alibaba Cloud Linux 3 / CentOS
# ========================================

set -e  # 遇到错误立即退出

echo "========================================="
echo "开始配置大模型API代理服务器环境"
echo "========================================="

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# 1. 安装基础工具
echo -e "\n${GREEN}[1/7] 安装基础工具...${NC}"
yum install -y epel-release
yum install -y wget curl git vim htop net-tools

# 2. 安装Nginx
echo -e "\n${GREEN}[2/7] 安装Nginx...${NC}"
yum install -y nginx
systemctl enable nginx
echo -e "${GREEN}✓ Nginx安装完成${NC}"

# 3. 安装Python环境
echo -e "\n${GREEN}[3/7] 配置Python环境...${NC}"
yum install -y python3-pip python3-devel
pip3 install --upgrade pip

# 创建项目目录
PROJECT_DIR="/var/www/llm-proxy"
mkdir -p $PROJECT_DIR
mkdir -p $PROJECT_DIR/logs

# 创建虚拟环境
python3 -m venv $PROJECT_DIR/venv
source $PROJECT_DIR/venv/bin/activate

# 安装Python依赖
pip install fastapi uvicorn[standard] httpx redis python-multipart pydantic-settings
echo -e "${GREEN}✓ Python环境配置完成${NC}"

# 4. 安装Redis
echo -e "\n${GREEN}[4/7] 安装Redis...${NC}"
yum install -y redis
sed -i 's/^bind 127.0.0.1/bind 0.0.0.0/' /etc/redis.conf
sed -i 's/^protected-mode yes/protected-mode no/' /etc/redis.conf
sed -i 's/^maxmemory <bytes>/maxmemory 256mb/' /etc/redis.conf
sed -i 's/^# maxmemory-policy noeviction/maxmemory-policy allkeys-lru/' /etc/redis.conf
systemctl enable redis
systemctl start redis
echo -e "${GREEN}✓ Redis安装完成${NC}"

# 5. 安装Supervisor
echo -e "\n${GREEN}[5/7] 安装Supervisor...${NC}"
yum install -y supervisor
systemctl enable supervisord
echo -e "${GREEN}✓ Supervisor安装完成${NC}"

# 6. 配置防火墙
echo -e "\n${GREEN}[6/7] 配置防火墙...${NC}"
if command -v firewall-cmd &> /dev/null; then
    firewall-cmd --permanent --add-service=http
    firewall-cmd --permanent --add-service=https
    firewall-cmd --reload
    echo -e "${GREEN}✓ 防火墙配置完成${NC}"
else
    echo -e "${YELLOW}⚠ 防火墙未启用或使用iptables${NC}"
fi

# 7. 创建示例应用
echo -e "\n${GREEN}[7/7] 创建示例应用...${NC}"
cat > $PROJECT_DIR/main.py << 'PYEOF'
"""
大模型API代理服务
支持转发到OpenAI、通义千问等多个大模型平台
"""
from fastapi import FastAPI, HTTPException, Request
from fastapi.responses import StreamingResponse, JSONResponse
import httpx
import redis
import json
import os
from typing import Optional
from pydantic import BaseModel

app = FastAPI(title="LLM API Proxy", version="1.0.0")

# Redis连接
redis_client = redis.Redis(host='localhost', port=6379, db=0, decode_responses=True)

# 配置
class Config:
    # 在这里配置你的上游API密钥
    UPSTREAM_API_URL = "https://api.openai.com/v1"  # 默认OpenAI，可修改为其他平台
    API_KEY = "your-api-key-here"  # 修改为实际的API密钥
    CACHE_TTL = 3600  # 缓存时间（秒）

config = Config()

class ChatRequest(BaseModel):
    model: str
    messages: list
    temperature: Optional[float] = 0.7
    max_tokens: Optional[int] = 2000
    stream: Optional[bool] = False

@app.get("/")
async def root():
    return {
        "service": "LLM API Proxy",
        "status": "running",
        "endpoints": {
            "chat": "/v1/chat/completions",
            "health": "/health"
        }
    }

@app.get("/health")
async def health():
    return {"status": "healthy"}

@app.post("/v1/chat/completions")
async def chat_completions(request: ChatRequest):
    """聊天接口，支持流式和非流式输出"""

    # 检查缓存
    cache_key = f"chat:{request.model}:{hash(str(request.messages))}"
    cached = redis_client.get(cache_key)
    if cached and not request.stream:
        return JSONResponse(content=json.loads(cached))

    # 转发到上游API
    headers = {
        "Authorization": f"Bearer {config.API_KEY}",
        "Content-Type": "application/json"
    }

    async with httpx.AsyncClient(timeout=60.0) as client:
        try:
            if request.stream:
                # 流式响应
                async def stream_response():
                    async with client.stream(
                        "POST",
                        f"{config.UPSTREAM_API_URL}/chat/completions",
                        json=request.dict(),
                        headers=headers
                    ) as response:
                        async for chunk in response.aiter_bytes():
                            yield chunk

                return StreamingResponse(
                    stream_response(),
                    media_type="text/event-stream"
                )
            else:
                # 非流式响应
                response = await client.post(
                    f"{config.UPSTREAM_API_URL}/chat/completions",
                    json=request.dict(),
                    headers=headers
                )
                result = response.json()

                # 缓存结果
                redis_client.setex(cache_key, config.CACHE_TTL, json.dumps(result))

                return JSONResponse(content=result)

        except httpx.HTTPError as e:
            raise HTTPException(status_code=500, detail=str(e))

if __name__ == "__main__":
    import uvicorn
    uvicorn.run(app, host="0.0.0.0", port=8000)
PYEOF

echo -e "${GREEN}✓ 示例应用创建完成${NC}"

# 创建Supervisor配置
cat > /etc/supervisord.d/llm-proxy.ini << 'SUPEOF'
[program:llm-proxy]
command=/var/www/llm-proxy/venv/bin/uvicorn main:app --host 0.0.0.0 --port 8000 --workers 2
directory=/var/www/llm-proxy
user=nginx
autostart=true
autorestart=true
stderr_logfile=/var/www/llm-proxy/logs/error.log
stdout_logfile=/var/www/llm-proxy/logs/access.log
SUPEOF

# 创建Nginx配置
cat > /etc/nginx/conf.d/llm-proxy.conf << 'NGINXEOF'
server {
    listen 80;
    server_name _;

    # 限制请求体大小（大模型请求可能较大）
    client_max_body_size 10M;

    # 超时设置
    proxy_connect_timeout 300s;
    proxy_send_timeout 300s;
    proxy_read_timeout 300s;

    location / {
        proxy_pass http://127.0.0.1:8000;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;

        # SSE支持（流式响应）
        proxy_buffering off;
        proxy_cache off;
    }

    # 健康检查端点
    location /health {
        proxy_pass http://127.0.0.1:8000/health;
        access_log off;
    }
}
NGINXEOF

# 设置权限
chown -R nginx:nginx $PROJECT_DIR
chmod -R 755 $PROJECT_DIR

# 测试Nginx配置
nginx -t

echo -e "\n${GREEN}=========================================${NC}"
echo -e "${GREEN}✓ 环境配置完成！${NC}"
echo -e "${GREEN}=========================================${NC}"
echo -e "\n下一步操作："
echo -e "1. ${YELLOW}编辑配置文件${NC}: vim $PROJECT_DIR/main.py"
echo -e "   - 修改 API_KEY 为你的实际密钥"
echo -e "   - 修改 UPSTREAM_API_URL 为目标平台"
echo -e "\n2. ${YELLOW}启动服务${NC}:"
echo -e "   systemctl start supervisord"
echo -e "   supervisorctl restart llm-proxy"
echo -e "   systemctl restart nginx"
echo -e "\n3. ${YELLOW}测试服务${NC}:"
echo -e "   curl http://localhost/health"
echo -e "\n4. ${YELLOW}配置SSL证书（可选）${NC}:"
echo -e "   使用 certbot --nginx 配置HTTPS"
echo -e "\n${YELLOW}📝 日志位置:${NC}"
echo -e "   应用日志: $PROJECT_DIR/logs/"
echo -e "   Nginx日志: /var/log/nginx/"
echo -e "\n${YELLOW}📊 监控命令:${NC}"
echo -e "   supervisorctl status    # 查看应用状态"
echo -e "   systemctl status nginx  # 查看Nginx状态"
echo -e "   systemctl status redis  # 查看Redis状态"
echo -e ""
