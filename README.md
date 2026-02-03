# DogeAI - Anthropic API 中转代理服务

一个高性能的 Anthropic API 中转代理服务，支持用户自助申请API密钥、流量统计、并发控制等功能。

## 功能特性

- 🚀 **高性能**: 支持20并发请求处理
- 🔐 **安全认证**: API密钥认证、每日限额控制
- 📊 **统计分析**: 详细的调用统计和日志记录
- 🎯 **用户自助**: 用户可自助申请API密钥
- 📧 **邮件通知**: 自动发送欢迎邮件
- 🔄 **模型映射**: 支持新版 Claude 4.5 命名
- 📦 **易于部署**: 完整的 systemd 服务配置

## 技术栈

- **Python**: 3.6+
- **Web框架**: FastAPI + Uvicorn
- **数据库**: SQLite3
- **HTTP客户端**: httpx

## 目录结构

```
dogeAI/
├── src/                    # 源代码
│   └── anthropic_proxy_v2.py
├── config/                 # 配置文件
│   └── .env.example
├── scripts/                # 工具脚本
│   ├── init_db.py
│   ├── api-proxy-admin.sh
│   └── ...
├── systemd/                # 系统服务配置
│   └── anthropic-proxy.service
├── docs/                   # 文档
├── requirements.txt        # Python依赖
└── README.md              # 项目说明
```

## 快速开始

### 1. 环境要求

- Python 3.6 或更高版本
- Linux 系统（推荐 CentOS 7+/Ubuntu 18.04+）
- Root 权限（用于创建系统服务和目录）

### 2. 安装依赖

```bash
# 创建虚拟环境（推荐）
python3 -m venv /root/anthropic-proxy-v2
source /root/anthropic-proxy-v2/bin/activate

# 安装依赖
pip install -r requirements.txt
```

### 3. 配置环境变量

```bash
# 复制配置模板
cp config/.env.example .env

# 编辑配置文件，修改以下关键配置：
# - UPSTREAM_API_KEY: 上游API密钥
# - ADMIN_KEY: 管理员密钥（请修改为随机强密码）
# - SMTP_*: 邮件服务器配置
```

### 4. 初始化数据库

```bash
# 创建数据库目录
mkdir -p /var/lib/anthropic-proxy
mkdir -p /var/log/anthropic-proxy

# 初始化数据库
python scripts/init_db.py
```

### 5. 启动服务

#### 方式一：直接启动（测试）

```bash
python src/anthropic_proxy_v2.py
```

#### 方式二：使用 systemd 服务（生产环境）

```bash
# 复制服务文件
cp systemd/anthropic-proxy.service /etc/systemd/system/

# 重载并启动服务
systemctl daemon-reload
systemctl enable anthropic-proxy
systemctl start anthropic-proxy

# 查看服务状态
systemctl status anthropic-proxy
```

## API 接口

### 公开接口

| 端点 | 方法 | 说明 |
|------|------|------|
| `/` | GET | 服务信息 |
| `/health` | GET | 健康检查 |
| `/v1/models` | GET | 获取可用模型列表 |
| `/v1/messages` | POST | 创建消息（API调用） |
| `/apply` | POST | 申请API密钥 |
| `/apply/check-availability/{field}/{value}` | GET | 检查邮箱/用户名可用性 |

### 管理接口（需要 x-admin-key 头部）

| 端点 | 方法 | 说明 |
|------|------|------|
| `/admin/stats` | GET | 获取统计信息 |
| `/admin/keys` | GET | 列出所有API密钥 |
| `/admin/keys/create` | POST | 创建新API密钥 |
| `/admin/toggle-token` | POST | 切换密钥状态 |
| `/admin/token-stats` | GET | 获取密钥详细统计 |
| `/admin/recent-calls` | GET | 获取最近调用记录 |

## 使用示例

### 申请 API 密钥

```bash
curl -X POST http://your-server:8080/apply \
  -H "Content-Type: application/json" \
  -d '{
    "email": "user@example.com",
    "username": "john_doe",
    "full_name": "John Doe",
    "reason": "Personal AI projects"
  }'
```

### 调用 API

```bash
curl -X POST http://your-server:8080/v1/messages \
  -H "x-api-key: sk-your-api-key" \
  -H "Content-Type: application/json" \
  -H "anthropic-version: 2023-06-01" \
  -d '{
    "model": "claude-sonnet-4.5",
    "max_tokens": 1024,
    "messages": [
      {"role": "user", "content": "Hello, Claude!"}
    ]
  }'
```

### 查看统计信息

```bash
curl -X GET http://your-server:8080/admin/stats \
  -H "x-admin-key: admin-change-this-key"
```

## 模型映射

服务支持新版 Claude 4.5 命名，会自动映射到上游正确的模型：

| 请求模型 | 实际模型 |
|---------|---------|
| `claude-sonnet-4.5` | `claude-3-5-sonnet-20241022` |
| `claude-opus-4.5` | `claude-3-opus-20240229` |
| `claude-3.5-sonnet` | `claude-3-5-sonnet-20241022` |
| `claude-3.5-haiku` | `claude-3-5-haiku-20241022` |

## 配置说明

### 并发控制

- 全局并发限制：20个请求
- 用户级并发限制：1个请求（每个用户同时只能有一个请求在处理）

### 限额设置

- 默认每日限额：10,000 tokens
- 可通过管理接口为每个用户设置不同的限额
- 有效期默认365天

### 日志

- 日志目录：`/var/log/anthropic-proxy/`
- 日志文件大小限制：100MB
- 保留10个历史文件

## 维护命令

```bash
# 查看服务日志
tail -f /var/log/anthropic-proxy/api-proxy.log

# 查看系统服务日志
journalctl -u anthropic-proxy -f

# 重启服务
systemctl restart anthropic-proxy

# 停止服务
systemctl stop anthropic-proxy

# 数据库备份
cp /var/lib/anthropic-proxy/stats.db /var/lib/anthropic-proxy/stats.db.backup.$(date +%Y%m%d_%H%M%S)
```

## 安全建议

1. **修改默认密钥**: 务必修改 `.env` 中的 `ADMIN_KEY`
2. **限制CORS**: 生产环境应配置具体的允许域名
3. **使用HTTPS**: 建议在前端使用 Nginx 配置 SSL
4. **定期备份**: 定期备份数据库文件
5. **监控日志**: 关注异常访问和错误日志

## 故障排除

### 服务无法启动

```bash
# 检查端口占用
netstat -tlnp | grep 8080

# 检查权限
ls -la /var/lib/anthropic-proxy/
ls -la /var/log/anthropic-proxy/
```

### API调用失败

```bash
# 查看实时日志
tail -f /var/log/anthropic-proxy/api-proxy.log

# 检查上游API配置
curl -H "x-api-key: YOUR_UPSTREAM_KEY" https://open.bigmodel.cn/api/anthropic/v1/messages
```

### 数据库问题

```bash
# 检查数据库文件
ls -lh /var/lib/anthropic-proxy/stats.db

# 重新初始化
python scripts/init_db.py
```

## 许可证

MIT License

## 贡献

欢迎提交 Issue 和 Pull Request！

## 联系方式

- 项目主页: https://github.com/aitachi/dogeAI
- 邮箱: contact@aitachi.cloud
