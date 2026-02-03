# 大模型API代理服务器 - 快速开始指南

## 📦 环境清单

### 已安装组件
- ✅ Python 3.9+
- ✅ Node.js
- ⚠️  Nginx (需安装)
- ⚠️  Redis (需安装)
- ⚠️  Supervisor (需安装)

### 推荐架构
```
Internet → Nginx (80/443) → FastAPI (8000) → 上游大模型API
                ↓
            Redis (缓存)
```

## 🚀 快速安装

### 方式1: 自动安装（推荐）
```bash
# 执行自动安装脚本
bash /root/setup_llm_proxy.sh
```

### 方式2: 手动安装
```bash
# 1. 安装Nginx
yum install -y nginx
systemctl enable nginx

# 2. 安装Redis
yum install -y redis
systemctl enable redis --now

# 3. 安装Python依赖
pip3 install fastapi uvicorn httpx redis python-multipart

# 4. 安装Supervisor
yum install -y supervisor
systemctl enable supervisord
```

## 🔧 配置说明

### 1. 应用配置
编辑 `/var/www/llm-proxy/main.py`:

```python
class Config:
    # 修改为你的目标API
    UPSTREAM_API_URL = "https://api.openai.com/v1"  # 或其他平台
    API_KEY = "your-api-key-here"  # 你的API密钥
    CACHE_TTL = 3600  # 缓存时间（秒）
```

### 2. 支持的大模型平台

#### OpenAI
```python
UPSTREAM_API_URL = "https://api.openai.com/v1"
API_KEY = "sk-xxx..."
```

#### 通义千问（阿里云）
```python
UPSTREAM_API_URL = "https://dashscope.aliyuncs.com/api/v1"
API_KEY = "sk-xxx..."
```

#### 智谱AI (ChatGLM)
```python
UPSTREAM_API_URL = "https://open.bigmodel.cn/api/paas/v4"
API_KEY = "xxx..."
```

#### 百度文心一言
```python
UPSTREAM_API_URL = "https://aip.baidubce.com/rpc/2.0/ai_custom/v1/wenxinworkshop/chat"
API_KEY = "xxx..."
```

#### DeepSeek
```python
UPSTREAM_API_URL = "https://api.deepseek.com/v1"
API_KEY = "sk-xxx..."
```

## 🎯 启动服务

```bash
# 1. 启动所有服务
systemctl start redis
systemctl start supervisord
supervisorctl restart llm-proxy
systemctl restart nginx

# 2. 查看状态
supervisorctl status
systemctl status nginx

# 3. 测试服务
curl http://localhost/health
# 预期输出: {"status":"healthy"}
```

## 📝 API使用示例

### 测试聊天接口
```bash
curl -X POST http://your-server-ip/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-3.5-turbo",
    "messages": [{"role": "user", "content": "你好"}],
    "temperature": 0.7
  }'
```

### Python客户端示例
```python
import requests

response = requests.post(
    "http://your-server-ip/v1/chat/completions",
    json={
        "model": "gpt-3.5-turbo",
        "messages": [{"role": "user", "content": "你好"}]
    }
)
print(response.json())
```

### 流式响应示例
```python
import requests

response = requests.post(
    "http://your-server-ip/v1/chat/completions",
    json={
        "model": "gpt-3.5-turbo",
        "messages": [{"role": "user", "content": "写一首诗"}],
        "stream": True
    },
    stream=True
)

for line in response.iter_lines():
    if line:
        print(line.decode('utf-8'))
```

## 🔒 配置SSL证书（HTTPS）

### 使用Let's Encrypt免费证书
```bash
# 1. 安装certbot
yum install -y certbot python3-certbot-nginx

# 2. 申请证书（替换为你的域名）
certbot --nginx -d your-domain.com

# 3. 自动续期
certbot renew --dry-run
```

## 📊 性能优化建议

### 1. 调整工作进程数
编辑 `/etc/supervisord.d/llm-proxy.ini`:
```ini
# 根据CPU核心数调整（建议为核心数*2+1）
command=/var/www/llm-proxy/venv/bin/uvicorn main:app --workers 4
```

### 2. 调整Nginx缓冲
编辑 `/etc/nginx/nginx.conf`:
```nginx
# 在http块添加
client_body_buffer_size 128k;
client_max_body_size 10m;
```

### 3. Redis内存优化
编辑 `/etc/redis.conf`:
```conf
maxmemory 512mb
maxmemory-policy allkeys-lru
```

## 🛠️ 故障排查

### 服务无法启动
```bash
# 查看应用日志
tail -f /var/www/llm-proxy/logs/error.log

# 查看Nginx日志
tail -f /var/log/nginx/error.log

# 查看Supervisor状态
supervisorctl status
```

### 端口被占用
```bash
# 查看端口占用
netstat -tunlp | grep 8000
netstat -tunlp | grep 80
```

### Redis连接失败
```bash
# 测试Redis连接
redis-cli ping
# 应返回: PONG
```

## 🔍 监控命令

```bash
# 实时监控日志
tail -f /var/www/llm-proxy/logs/access.log

# 查看请求统计
tail -f /var/log/nginx/access.log | grep -v "200"

# 查看系统资源
htop

# 查看Redis使用情况
redis-cli info stats
```

## 📈 扩展功能

### 1. 添加API密钥验证
编辑 `main.py`:
```python
from fastapi import Header, HTTPException

async def verify_api_key(x_api_key: str = Header(...)):
    if x_api_key != "your-client-api-key":
        raise HTTPException(status_code=403, detail="Invalid API Key")
    return x_api_key

@app.post("/v1/chat/completions", dependencies=[Depends(verify_api_key)])
async def chat_completions(...):
    ...
```

### 2. 添加请求限流
```python
from fastapi_limiter import FastAPILimiter
from fastapi_limiter.depends import RateLimiter

@app.post("/v1/chat/completions")
@limiter.limit("10/minute")  # 每分钟最多10次
async def chat_completions(...):
    ...
```

### 3. 添加多模型支持
```python
MODEL_MAPPING = {
    "gpt-3.5": "https://api.openai.com/v1",
    "qwen": "https://dashscope.aliyuncs.com/api/v1",
    "chatglm": "https://open.bigmodel.cn/api/paas/v4"
}

def get_upstream_url(model: str):
    return MODEL_MAPPING.get(model, DEFAULT_UPSTREAM_URL)
```

## 📞 常见问题

**Q: 如何支持多个大模型平台？**
A: 修改 `main.py`，根据model参数动态选择UPSTREAM_API_URL

**Q: 如何处理并发请求？**
A: 调整uvicorn的workers参数，建议设置为CPU核心数*2

**Q: 如何降低成本？**
A: 启用Redis缓存，相同请求直接返回缓存结果

**Q: 内存不足怎么办？**
A: 减少workers数量，或升级服务器配置

## 📚 参考资料
- FastAPI文档: https://fastapi.tiangolo.com/
- Nginx文档: https://nginx.org/en/docs/
- Redis文档: https://redis.io/docs/

---

**部署完成后，建议:**1. 配置防火墙规则，限制访问来源
2. 启用HTTPS加密
3. 设置监控告警
4. 定期备份数据和配置
