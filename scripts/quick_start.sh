#!/bin/bash
# ========================================
# 快速配置脚本
# ========================================

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}  大模型API代理服务 - 配置向导${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""

# 检查是否已安装
if [ ! -f "/var/www/llm-proxy/main.py" ]; then
    echo -e "${RED}❌ 服务尚未安装${NC}"
    echo -e "请先运行: bash /root/setup_llm_proxy.sh"
    exit 1
fi

echo -e "${GREEN}当前配置状态:${NC}\n"

# 1. 选择默认平台
echo -e "${YELLOW}[1/4] 选择默认平台${NC}"
echo "1) OpenAI"
echo "2) 通义千问"
echo "3) 智谱AI (ChatGLM)"
echo "4) DeepSeek"
echo "5) 百度文心一言"
read -p "请选择 [1-5]: " platform_choice

case $platform_choice in
    1)
        PLATFORM="openai"
        API_URL="https://api.openai.com/v1"
        ;;
    2)
        PLATFORM="qwen"
        API_URL="https://dashscope.aliyuncs.com/api/v1"
        ;;
    3)
        PLATFORM="chatglm"
        API_URL="https://open.bigmodel.cn/api/paas/v4"
        ;;
    4)
        PLATFORM="deepseek"
        API_URL="https://api.deepseek.com/v1"
        ;;
    5)
        PLATFORM="baidu"
        API_URL="https://aip.baidubce.com/rpc/2.0/ai_custom/v1/wenxinworkshop/chat"
        ;;
    *)
        echo -e "${RED}无效选择${NC}"
        exit 1
        ;;
esac

echo -e "${GREEN}✓ 已选择: $PLATFORM${NC}\n"

# 2. 输入API密钥
echo -e "${YELLOW}[2/4] 配置API密钥${NC}"
read -p "请输入 $PLATFORM 的API密钥: " api_key

if [ -z "$api_key" ]; then
    echo -e "${RED}❌ API密钥不能为空${NC}"
    exit 1
fi

echo -e "${GREEN}✓ API密钥已配置${NC}\n"

# 3. 是否启用多平台
echo -e "${YELLOW}[3/4] 是否启用多平台支持？${NC}"
read -p "启用多平台支持? [y/N]: " multi_platform

if [ "$multi_platform" = "y" ] || [ "$multi_platform" = "Y" ]; then
    # 使用多平台示例
    cp /root/multi_platform_example.py /var/www/llm-proxy/main.py
    echo -e "${GREEN}✓ 已启用多平台支持${NC}"
else
    # 使用单平台配置
    cat > /var/www/llm-proxy/main.py << 'EOF'
from fastapi import FastAPI, HTTPException
from fastapi.responses import StreamingResponse, JSONResponse
import httpx
import redis
import json
from typing import Optional
from pydantic import BaseModel

app = FastAPI(title="LLM API Proxy")
redis_client = redis.Redis(host='localhost', port=6379, db=0, decode_responses=True)

class Config:
    UPSTREAM_API_URL = "API_URL_PLACEHOLDER"
    API_KEY = "API_KEY_PLACEHOLDER"
    CACHE_TTL = 3600

config = Config()

class ChatRequest(BaseModel):
    model: str
    messages: list
    temperature: Optional[float] = 0.7
    max_tokens: Optional[int] = 2000
    stream: Optional[bool] = False

@app.get("/")
async def root():
    return {"service": "LLM API Proxy", "status": "running"}

@app.get("/health")
async def health():
    return {"status": "healthy"}

@app.post("/v1/chat/completions")
async def chat_completions(request: ChatRequest):
    cache_key = f"chat:{request.model}:{hash(str(request.messages))}"
    cached = redis_client.get(cache_key)
    if cached and not request.stream:
        return JSONResponse(content=json.loads(cached))

    headers = {
        "Authorization": f"Bearer {config.API_KEY}",
        "Content-Type": "application/json"
    }

    async with httpx.AsyncClient(timeout=60.0) as client:
        try:
            if request.stream:
                async def stream_response():
                    async with client.stream(
                        "POST",
                        f"{config.UPSTREAM_API_URL}/chat/completions",
                        json=request.dict(),
                        headers=headers
                    ) as response:
                        async for chunk in response.aiter_bytes():
                            yield chunk
                return StreamingResponse(stream_response(), media_type="text/event-stream")
            else:
                response = await client.post(
                    f"{config.UPSTREAM_API_URL}/chat/completions",
                    json=request.dict(),
                    headers=headers
                )
                result = response.json()
                redis_client.setex(cache_key, config.CACHE_TTL, json.dumps(result))
                return JSONResponse(content=result)
        except Exception as e:
            raise HTTPException(status_code=500, detail=str(e))

if __name__ == "__main__":
    import uvicorn
    uvicorn.run(app, host="0.0.0.0", port=8000)
EOF

    # 替换占位符
    sed -i "s|API_URL_PLACEHOLDER|$API_URL|g" /var/www/llm-proxy/main.py
    sed -i "s|API_KEY_PLACEHOLDER|$api_key|g" /var/www/llm-proxy/main.py
    echo -e "${GREEN}✓ 已配置单平台模式${NC}"
fi

# 4. 配置环境变量（多平台）
if [ "$multi_platform" = "y" ] || [ "$multi_platform" = "Y" ]; then
    echo -e "\n${YELLOW}配置其他平台的API密钥（可选，直接跳过则不启用）${NC}"

    echo -ne "OpenAI API Key: "
    read openai_key
    [ -n "$openai_key" ] && echo "export OPENAI_API_KEY=$openai_key" >> /var/www/llm-proxy/.env

    echo -ne "通义千问 API Key: "
    read qwen_key
    [ -n "$qwen_key" ] && echo "export QWEN_API_KEY=$qwen_key" >> /var/www/llm-proxy/.env

    echo -ne "智谱AI API Key: "
    read chatglm_key
    [ -n "$chatglm_key" ] && echo "export CHATGLM_API_KEY=$chatglm_key" >> /var/www/llm-proxy/.env

    echo -ne "DeepSeek API Key: "
    read deepseek_key
    [ -n "$deepseek_key" ] && echo "export DEEPSEEK_API_KEY=$deepseek_key" >> /var/www/llm-proxy/.env

    echo -ne "百度 API Key: "
    read baidu_key
    [ -n "$baidu_key" ] && echo "export BAIDU_API_KEY=$baidu_key" >> /var/www/llm-proxy/.env

    # 设置当前选择的主平台密钥
    case $PLATFORM in
        openai) echo "export OPENAI_API_KEY=$api_key" >> /var/www/llm-proxy/.env ;;
        qwen) echo "export QWEN_API_KEY=$api_key" >> /var/www/llm-proxy/.env ;;
        chatglm) echo "export CHATGLM_API_KEY=$api_key" >> /var/www/llm-proxy/.env ;;
        deepseek) echo "export DEEPSEEK_API_KEY=$api_key" >> /var/www/llm-proxy/.env ;;
        baidu) echo "export BAIDU_API_KEY=$api_key" >> /var/www/llm-proxy/.env ;;
    esac
fi

# 5. 重启服务
echo -e "\n${YELLOW}[4/4] 重启服务${NC}"
systemctl restart redis
supervisorctl restart llm-proxy
systemctl restart nginx

sleep 2

echo -e "\n${GREEN}========================================${NC}"
echo -e "${GREEN}✓ 配置完成！${NC}"
echo -e "${GREEN}========================================${NC}\n"

echo -e "${YELLOW}服务信息:${NC}"
echo -e "  服务地址: http://$(curl -s ifconfig.me)"
echo -e "  健康检查: http://$(curl -s ifconfig.me)/health"
echo -e "  API端点: http://$(curl -s ifconfig.me)/v1/chat/completions"
echo -e ""

echo -e "${YELLOW}测试命令:${NC}"
echo -e "  curl http://$(curl -s ifconfig.me)/health"
echo -e ""

echo -e "${YELLOW}管理命令:${NC}"
echo -e "  bash /root/service_manager.sh status"
echo -e "  bash /root/service_manager.sh logs"
echo -e "  bash /root/service_manager.sh test"
echo -e ""

if [ ! -f "/etc/letsencrypt/live/$(hostname -f)/fullchain.pem" ]; then
    echo -e "${YELLOW}建议: 配置HTTPS证书${NC}"
    echo -e "  certbot --nginx -d your-domain.com"
    echo -e ""
fi

echo -e "${YELLOW}详细文档:${NC}"
echo -e "  cat /root/README.md"
echo -e ""
