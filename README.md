# AITACHI Cloud - Claude Code Proxy (Go)

高性能 Claude Code API 中转服务，使用 Go 语言实现，内存占用极低。

## 特性

- 🚀 **高性能** - Go 语言实现，响应迅速
- 💾 **低内存** - 仅占用 ~13 MB 内存
- 🔄 **完整兼容** - 兼容 Claude Code 所有 API
- 📊 **监控面板** - 内置监控页面
- 🔐 **SSL 支持** - Let's Encrypt 自动证书

## 技术栈

| 组件 | 技术 |
|-----|------|
| 语言 | Go 1.21 |
| 框架 | Gorilla Mux |
| 代理 | Nginx |
| 上游 API | BigModel (智谱 GLM) |

## 支持的模型

| Claude 模型 | 上游模型 |
|-------------|---------|
| `claude-sonnet-4-6-20250514` | sonnet |
| `claude-sonnet-4-6-1m` | sonnet[1m] |
| `claude-opus-4-6-20251101` | opus |
| `claude-opus-4-6-1m` | opus[1m] |
| `claude-haiku-4-5-20251001` | haiku |

## 快速开始

### 编译

```bash
# 安装依赖
cd /root/go-proxy
go mod download

# 编译
go build -o go-proxy ./cmd/main.go

# 运行
./go-proxy
```

### Systemd 服务

```bash
# 启动
systemctl start go-proxy

# 停止
systemctl stop go-proxy

# 重启
systemctl restart go-proxy

# 查看状态
systemctl status go-proxy
```

## 配置

配置文件路径：`/root/dogeAI/config.json`

```json
{
    "api_base": "https://open.bigmodel.cn/api/anthropic",
    "api_key": "your-api-key",
    "model_map": {
        "claude-sonnet-4-6-20250514": "sonnet",
        "claude-opus-4-6-20251101": "opus",
        "claude-haiku-4-5-202501001": "haiku"
    },
    "default_model": "sonnet"
}
```

## API 端点

| 端点 | 方法 | 说明 |
|-----|------|------|
| `/v1/models` | GET | 获取模型列表 |
| `/v1/messages` | POST | 发送消息 |
| `/v1/messages/count_tokens` | POST | Token 计数 |
| `/api/metrics` | GET | 监控指标 |
| `/api/health` | GET | 健康检查 |
| `/monitor.html` | GET | 监控页面 |
| `/oauth/*` | * | OAuth 认证 |

## Claude Code 配置

```bash
# 方式 1: 环境变量
export ANTHROPIC_BASE_URL="https://aitachi.cloud"
export ANTHROPIC_API_KEY="sk-any-key"

# 方式 2: 配置文件
cat > ~/.claude-code/config.json << EOF
{
  "apiUrl": "https://aitachi.cloud",
  "apiKey": "sk-any-key"
}
EOF
```

## 部署架构

```
客户端
  │
  ├─ HTTPS (443) ──► Nginx (SSL终止)
  │                    │
  │                    └──► Go Proxy (3001)
  │                                    │
  └─────────────────────────────── BigModel API
```

## 监控

- **监控页面**: `https://aitachi.cloud/monitor.html`
- **健康检查**: `https://aitachi.cloud/api/health`
- **API 指标**: `https://aitachi.cloud/api/metrics`

## 内存优化

| 服务 | 内存占用 |
|-----|---------|
| Go Proxy | ~13 MB |
| Nginx | ~9 MB |
| **总计** | **~22 MB** |

## 日志

- Go 服务: `journalctl -u go-proxy -f`
- Nginx 访问: `/var/log/nginx/access.log`
- Nginx 错误: `/var/log/nginx/error.log`

## License

MIT
