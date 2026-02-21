# Claude Code Proxy - 部署文档

## 📋 目录结构

```
/root/dogeAI/
├── main.py                 # 主入口 (99行)
├── config.py               # 配置管理 (47行)
├── utils.py                # 工具函数 (117行)
├── converter.py            # 请求/响应转换 (268行)
├── streamer.py             # 流式SSE处理 (218行)
├── ssl_helper.py           # SSL证书生成 (90行)
├── routes/                 # 路由模块
│   ├── __init__.py         # 路由注册 (17行)
│   ├── base.py             # 基础路由 (100行)
│   ├── api.py              # 平台API (117行)
│   ├── oauth.py            # OAuth认证 (128行)
│   └── claude.py           # Claude Code + 核心路由 (312行)
├── config.json             # 配置文件 ⭐
├── start.sh                # 启动脚本 ⭐
├── stop.sh                 # 停止脚本 ⭐
├── proxy.py.backup         # 原版备份
├── README_MODULAR.md       # 模块化架构说明
└── TEST_REPORT.md          # 完整测试报告
```

## ⚙️ 配置文件详解 (config.json)

### 完整配置

```json
{
    "api_base": "https://dashscope.aliyuncs.com/compatible-mode/v1",
    "api_key": "sk-a9a4edb1b4214016baa11c9be3b9fec4",
    "model_map": {
        "claude-sonnet-4-5-20250929": "qwen-plus-latest",
        "claude-sonnet-4-20250514": "qwen-plus-latest",
        "claude-opus-4-20250514": "qwen-max-latest",
        "claude-3-5-sonnet-20241022": "qwen-plus-latest",
        "claude-3-5-sonnet-20240620": "qwen-plus-latest",
        "claude-3-opus-20240229": "qwen-max-latest",
        "claude-3-haiku-20240307": "qwen-turbo-latest",
        "claude-3-5-haiku-20241022": "qwen-turbo-latest",
        "claude-haiku-4-5-20251001": "qwen-turbo-latest"
    },
    "max_output_tokens_limit": {
        "qwen-plus-latest": 8192,
        "qwen-max-latest": 8192,
        "qwen-turbo-latest": 8192
    },
    "default_model": "qwen-plus-latest",
    "default_max_tokens": 8192,
    "default_params": {
        "temperature": 0.7,
        "top_p": 0.9
    },
    "http_port": 3001,
    "https_port": 443,
    "log_level": "INFO",
    "request_timeout": 300,
    "max_retries": 3
}
```

### 配置项说明

| 配置项 | 类型 | 说明 | 默认值 |
|--------|------|------|--------|
| **api_base** | string | 上游API地址 | DashScope兼容模式 |
| **api_key** | string | DashScope API密钥 | 必填 |
| **model_map** | object | Claude→Qwen模型映射 | 9个模型 |
| **max_output_tokens_limit** | object | 各模型输出上限 | 8192 |
| **default_model** | string | 默认模型 | qwen-plus-latest |
| **default_max_tokens** | int | 默认最大tokens | 8192 |
| **default_params** | object | 默认生成参数 | temp:0.7, top_p:0.9 |
| **http_port** | int | HTTP端口 | 3001 |
| **https_port** | int | HTTPS端口 | 443 |
| **log_level** | string | 日志级别 | INFO |
| **request_timeout** | int | 请求超时(秒) | 300 |
| **max_retries** | int | 最大重试次数 | 3 |

### 模型映射说明

| Claude模型 | Qwen模型 | 用途 |
|-----------|---------|------|
| claude-sonnet-4-5-20250929 | qwen-plus-latest | 最新Sonnet 4.5 |
| claude-sonnet-4-20250514 | qwen-plus-latest | Sonnet 4 |
| claude-opus-4-20250514 | qwen-max-latest | Opus 4 |
| claude-3-5-sonnet-* | qwen-plus-latest | Sonnet 3.5系列 |
| claude-3-opus-* | qwen-max-latest | Opus 3 |
| claude-3-haiku-* | qwen-turbo-latest | Haiku 3 |

## 🚀 快速启动

### 方式1: 使用启动脚本 (推荐)

```bash
cd /root/dogeAI
./start.sh
```

启动脚本会自动：
- 检查配置文件
- 检查Python版本
- 检查端口占用
- 创建日志目录
- 启动服务并记录PID

### 方式2: 直接运行

```bash
cd /root/dogeAI
python3.11 main.py
```

### 停止服务

```bash
cd /root/dogeAI
./stop.sh
```

## 📊 服务端口

服务启动后监听以下端口：

| 协议 | 端口 | 用途 |
|------|------|------|
| HTTP | 3001 | HTTP API |
| HTTPS | 443 | HTTPS API (nginx代理) |

**外网访问**：
- HTTP: `http://59.110.40.73:3001`
- HTTPS: `https://59.110.40.73`

## 🔧 系统配置

### Nginx反向代理

```nginx
# HTTP (端口3001直接访问)
# HTTPS (通过nginx 443端口代理)

location /v1/claudecode/ {
    proxy_pass http://127.0.0.1:3001/;
    proxy_set_header Host $host;
    proxy_set_header X-Real-IP $remote_addr;
    proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    proxy_set_header X-Forwarded-Proto $scheme;
}
```

### 防火墙配置

```bash
# firewalld
firewall-cmd --permanent --add-port=3001/tcp
firewall-cmd --permanent --add-port=443/tcp
firewall-cmd --reload

# iptables
iptables -A INPUT -p tcp --dport 3001 -j ACCEPT
iptables -A INPUT -p tcp --dport 443 -j ACCEPT
service iptables save
```

### 阿里云安全组

在阿里云控制台开放：
- TCP 3001 (HTTP)
- TCP 443 (HTTPS)

## 📝 日志管理

### 日志位置

```
/root/dogeAI/logs/
├── proxy.log          # 服务日志
└── error.log          # 错误日志
```

### 查看日志

```bash
# 实时查看
tail -f /root/dogeAI/logs/proxy.log

# 查看最近100行
tail -n 100 /root/dogeAI/logs/proxy.log

# 搜索错误
grep ERROR /root/dogeAI/logs/proxy.log
```

## 🧪 测试验证

### 健康检查

```bash
curl http://127.0.0.1:3001/api/hello
# 返回: {"message":"hello"}
```

### 模型列表

```bash
curl http://127.0.0.1:3001/v1/models
# 返回9个Claude模型
```

### 消息测试

```bash
curl -X POST http://127.0.0.1:3001/v1/messages \
  -H "Content-Type: application/json" \
  -H "x-api-key: test" \
  -d '{
    "model": "claude-sonnet-4-5-20250929",
    "max_tokens": 100,
    "messages": [{"role": "user", "content": "1+1=?"}]
  }'
```

### 外网测试

```bash
# HTTP
curl http://59.110.40.73:3001/api/hello

# HTTPS
curl https://59.110.40.73/v1/claudecode/api/hello
```

## 🔄 版本切换

### 使用模块化版本 (默认)

```bash
cd /root/dogeAI
./start.sh
```

### 回滚到原版

```bash
cd /root/dogeAI
cp proxy.py.backup proxy.py
./stop.sh
python3.11 proxy.py
```

## 📈 性能优化

### 推荐配置

- **Python版本**: 3.11+ (性能提升20-30%)
- **Worker进程**: 根据CPU核心数调整
- **内存**: 建议1GB+
- **网络**: 100Mbps+

### 监控指标

- 服务启动时间: ~3秒
- 平均响应延迟: <500ms
- 流式首token: <300ms
- 内存占用: ~80MB

## 🛠️ 故障排查

### 端口被占用

```bash
# 查看占用
lsof -i :3001
netstat -tlnp | grep 3001

# 停止占用进程
./stop.sh
```

### 配置错误

```bash
# 验证JSON格式
python3.11 -m json.tool config.json

# 检查配置
cat config.json | grep api_key
```

### SSL证书问题

```bash
# 重新生成证书
rm -rf /root/dogeAI/ssl/*
./start.sh  # 自动重新生成
```

### 查看详细日志

```bash
# 停止服务
./stop.sh

# 前台运行（查看输出）
python3.11 main.py
```

## 📞 支持

- **测试报告**: TEST_REPORT.md
- **架构文档**: README_MODULAR.md
- **配置文件**: config.json
- **日志目录**: logs/

---

**版本**: v3.3 (Modular)
**更新时间**: 2026-02-17
**测试状态**: ✅ 全部通过 (26/26)
