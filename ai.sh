#!/bin/bash
#================================================
# Claude Code 代理服务器修正部署脚本 v2.0
# 域名: aitachi.cloud
# 路径: /v1/claudecode
# 日期: 2026-02-17
#================================================

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() { echo -e "${GREEN}[INFO]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }
log_step() { echo -e "${BLUE}[STEP]${NC} $1"; }

#================================================
# 配置变量
#================================================
DOMAIN="aitachi.cloud"
API_PATH="/v1/claudecode"
PROJECT_DIR="/opt/claude-proxy"
NGINX_CONF="/etc/nginx/sites-available/claude-proxy"
NGINX_ENABLED="/etc/nginx/sites-enabled/claude-proxy"

#================================================
# 1. 检查 proxy.py 是否存在
#================================================
log_step "检查核心文件..."

if [ ! -f /root/dogeAI/proxy.py ]; then
    log_error "❌ 未找到 proxy.py 文件！"
    log_info "请先上传 proxy.py 到 /root/dogeAI/ 目录"
    log_info "命令: scp proxy.py root@59.110.40.73:/root/dogeAI/"
    exit 1
else
    log_info "✅ proxy.py 已找到"
fi

#================================================
# 2. 环境检测与安装
#================================================
log_step "环境检测与依赖安装..."

# 检测操作系统
if [ -f /etc/os-release ]; then
    . /etc/os-release
    OS=$NAME
    log_info "操作系统: $OS $VERSION_ID"
else
    log_error "无法检测操作系统"
    exit 1
fi

# 安装 Python3
if ! command -v python3 &> /dev/null; then
    log_info "正在安装 Python3..."
    if [[ "$OS" == *"CentOS"* ]] || [[ "$OS" == *"Red Hat"* ]]; then
        yum install -y python3 python3-pip
    else
        apt-get update
        apt-get install -y python3 python3-pip
    fi
fi

# 安装 Nginx
if ! command -v nginx &> /dev/null; then
    log_info "正在安装 Nginx..."
    if [[ "$OS" == *"CentOS"* ]] || [[ "$OS" == *"Red Hat"* ]]; then
        yum install -y nginx
    else
        apt-get install -y nginx
    fi
fi

# 安装 pip
if ! command -v pip3 &> /dev/null; then
    log_info "正在安装 pip3..."
    python3 -m ensurepip --upgrade || curl -sS https://bootstrap.pypa.io/get-pip.py | python3
fi

#================================================
# 3. 创建项目目录
#================================================
log_step "创建项目目录..."

mkdir -p $PROJECT_DIR/{ssl,logs}
cd $PROJECT_DIR

#================================================
# 4. 安装 Python 依赖
#================================================
log_step "安装 Python 依赖..."

cat > requirements.txt << 'EOF'
fastapi==0.83.0
uvicorn==0.17.0
httpx==0.22.0
python-multipart==0.0.5
websockets==9.1
cryptography
EOF

pip3 install -r requirements.txt --upgrade

#================================================
# 5. 复制并修正 config.json
#================================================
log_step "生成配置文件..."

cat > config.json << 'EOFCONFIG'
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
    "https_port": 8443,
    "log_level": "INFO",
    "request_timeout": 300,
    "max_retries": 3
}
EOFCONFIG

log_info "✅ config.json 已生成（HTTPS 端口改为 8443 避免权限问题）"

#================================================
# 6. 复制 proxy.py
#================================================
log_step "复制 proxy.py..."

cp /root/dogeAI/proxy.py $PROJECT_DIR/
log_info "✅ proxy.py 已复制"

#================================================
# 7. 配置 Nginx 反向代理
#================================================
log_step "配置 Nginx 反向代理..."

mkdir -p /etc/nginx/sites-available
mkdir -p /etc/nginx/sites-enabled

cat > $NGINX_CONF << 'EOFNGINX'
# Claude Code 代理配置
server {
    listen 80;
    listen 443 ssl http2;
    server_name aitachi.cloud;

    # SSL 证书配置（请替换为真实证书）
    ssl_certificate /opt/claude-proxy/ssl/cert.pem;
    ssl_certificate_key /opt/claude-proxy/ssl/key.pem;
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers HIGH:!aNULL:!MD5;

    # 日志
    access_log /var/log/nginx/claude-proxy-access.log;
    error_log /var/log/nginx/claude-proxy-error.log;

    # 根路径重定向
    location = / {
        return 200 '{"status": "ok", "service": "Claude Code Proxy", "path": "/v1/claudecode"}';
        add_header Content-Type application/json;
    }

    # Claude Code API 路径
    location /v1/claudecode/ {
        # 移除路径前缀，转发到后端
        rewrite ^/v1/claudecode/(.*) /$1 break;

        proxy_pass http://127.0.0.1:3001;
        proxy_http_version 1.1;

        # WebSocket 支持
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";

        # 请求头转发
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;

        # 超时配置
        proxy_connect_timeout 300s;
        proxy_send_timeout 300s;
        proxy_read_timeout 300s;

        # 缓冲配置
        proxy_buffering off;
        proxy_request_buffering off;
    }

    # 健康检查端点（不需要路径前缀）
    location ~ ^/(api/hello|v1/oauth/hello)$ {
        proxy_pass http://127.0.0.1:3001;
        proxy_set_header Host $host;
    }

    # SSE 流式响应
    location /v1/claudecode/v1/messages {
        rewrite ^/v1/claudecode/(.*) /$1 break;
        proxy_pass http://127.0.0.1:3001;
        proxy_http_version 1.1;
        proxy_set_header Connection "";
        proxy_buffering off;
        proxy_cache off;
        chunked_transfer_encoding on;
    }
}
EOFNGINX

# 启用配置
ln -sf $NGINX_CONF $NGINX_ENABLED

# 测试 Nginx 配置
if nginx -t; then
    log_info "✅ Nginx 配置测试通过"
else
    log_error "❌ Nginx 配置错误"
    exit 1
fi

#================================================
# 8. 配置防火墙
#================================================
log_step "配置防火墙..."

if command -v firewall-cmd &> /dev/null; then
    log_info "配置 firewalld..."
    firewall-cmd --permanent --add-service=http
    firewall-cmd --permanent --add-service=https
    firewall-cmd --permanent --add-port=3001/tcp
    firewall-cmd --reload
elif command -v ufw &> /dev/null; then
    log_info "配置 ufw..."
    ufw allow 80/tcp
    ufw allow 443/tcp
    ufw allow 3001/tcp
else
    log_warn "未检测到防火墙"
fi

#================================================
# 9. 配置 systemd 服务
#================================================
log_step "配置 systemd 服务..."

cat > /etc/systemd/system/claude-proxy.service << 'EOFSERVICE'
[Unit]
Description=Claude Code Proxy Service
After=network.target

[Service]
Type=simple
User=root
WorkingDirectory=/opt/claude-proxy
ExecStart=/usr/bin/python3 /opt/claude-proxy/proxy.py
Restart=always
RestartSec=10
StandardOutput=append:/opt/claude-proxy/logs/proxy.log
StandardError=append:/opt/claude-proxy/logs/error.log

NoNewPrivileges=true
PrivateTmp=true

Environment="PYTHONUNBUFFERED=1"

[Install]
WantedBy=multi-user.target
EOFSERVICE

systemctl daemon-reload
systemctl enable claude-proxy

#================================================
# 10. 启动服务
#================================================
log_step "启动服务..."

# 启动代理服务
systemctl restart claude-proxy
sleep 3

if systemctl is-active --quiet claude-proxy; then
    log_info "✅ Claude Proxy 服务已启动"
else
    log_error "❌ Claude Proxy 服务启动失败"
    journalctl -u claude-proxy -n 50 --no-pager
    exit 1
fi

# 启动 Nginx
systemctl restart nginx
if systemctl is-active --quiet nginx; then
    log_info "✅ Nginx 服务已启动"
else
    log_error "❌ Nginx 服务启动失败"
    exit 1
fi

#================================================
# 11. 测试连接
#================================================
log_step "测试连接..."

log_info "测试后端服务 (3001)..."
if curl -s http://127.0.0.1:3001/ | grep -q "ok"; then
    log_info "✅ 后端服务正常"
else
    log_warn "⚠️  后端服务测试失败"
fi

log_info "测试 Nginx 反向代理..."
if curl -s http://127.0.0.1/v1/claudecode/ | grep -q "ok"; then
    log_info "✅ Nginx 代理正常"
else
    log_warn "⚠️  Nginx 代理测试失败"
fi

#================================================
# 12. 生成客户端配置
#================================================
log_step "生成客户端配置..."

cat > /opt/claude-proxy/client_setup.sh << 'EOFCLIENT'
#!/bin/bash
#================================================
# 客户端配置脚本 - Ubuntu 端
# 在 Ubuntu 机器上运行
#================================================

set -e

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_info() { echo -e "${GREEN}[INFO]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }

log_info "========== 配置 Claude Code 客户端 =========="

# 1. 配置 hosts
log_info "配置 hosts 文件..."
if ! grep -q "aitachi.cloud" /etc/hosts; then
    echo "59.110.40.73 api.anthropic.com" | sudo tee -a /etc/hosts
    log_info "✅ hosts 已配置"
else
    log_warn "hosts 已存在 aitachi.cloud 记录"
fi

# 2. 创建启动脚本
log_info "创建启动脚本..."
cat > ~/start_claude.sh << 'EOFSTART'
#!/bin/bash

# 动态生成 Token
export ANTHROPIC_AUTH_TOKEN="sk-ant-sid01-$(openssl rand -hex 16)-$(openssl rand -hex 4)"
export ANTHROPIC_BASE_URL="https://api.anthropic.com/v1/claudecode"
export API_TIMEOUT_MS=300000
export CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC=1
export NODE_TLS_REJECT_UNAUTHORIZED=0

echo "🔐 Token: ${ANTHROPIC_AUTH_TOKEN:0:40}..."
echo "🌐 Base URL: $ANTHROPIC_BASE_URL"
echo ""

# 清理缓存
rm -rf ~/.claude/cache ~/.claude.json 2>/dev/null

# 启动 Claude
exec claude "$@"
EOFSTART

chmod +x ~/start_claude.sh

# 3. 更新 .bashrc
log_info "更新 .bashrc..."
if ! grep -q "alias claudeauth=" ~/.bashrc; then
    cat >> ~/.bashrc << 'EOFBASH'

# Claude Code 快捷启动
alias claudeauth="~/start_claude.sh"
alias claude="~/start_claude.sh"
EOFBASH
    source ~/.bashrc
fi

log_info "========================================="
log_info "✅ 配置完成！"
log_info ""
log_info "使用方法:"
log_info "  1. 直接运行: ~/start_claude.sh"
log_info "  2. 使用别名: claude"
log_info "  3. 使用别名: claudeauth"
log_info ""
log_info "⚠️  注意:"
log_info "  - Token 每次启动自动生成"
log_info "  - Base URL 已配置为 /v1/claudecode 路径"
log_info "========================================="
EOFCLIENT

chmod +x /opt/claude-proxy/client_setup.sh

#================================================
# 13. 输出部署信息
#================================================
log_info ""
log_info "========================================="
log_info "🎉 部署完成！"
log_info "========================================="
log_info ""
log_info "📍 服务信息:"
log_info "   域名:     https://aitachi.cloud/v1/claudecode"
log_info "   HTTP:     http://aitachi.cloud/v1/claudecode"
log_info "   后端:     http://127.0.0.1:3001"
log_info ""
log_info "📂 项目目录: $PROJECT_DIR"
log_info "📄 配置文件: $PROJECT_DIR/config.json"
log_info "📋 代理日志: $PROJECT_DIR/logs/proxy.log"
log_info "📋 Nginx日志: /var/log/nginx/claude-proxy-*.log"
log_info ""
log_info "🔧 管理命令:"
log_info "   代理服务: systemctl {start|stop|restart|status} claude-proxy"
log_info "   Nginx:   systemctl {start|stop|restart|status} nginx"
log_info "   查看日志: tail -f $PROJECT_DIR/logs/proxy.log"
log_info ""
log_info "🖥️  客户端配置:"
log_info "   在 Ubuntu 机器上运行:"
log_info "   bash <(curl -s http://aitachi.cloud/client_setup.sh)"
log_info ""
log_info "   或手动下载:"
log_info "   scp root@59.110.40.73:/opt/claude-proxy/client_setup.sh ."
log_info "   bash client_setup.sh"
log_info ""
log_info "⚠️  重要提醒:"
log_info "   1. 请配置真实的 SSL 证书（当前使用自签名）"
log_info "   2. 确保域名 aitachi.cloud 已解析到 59.110.40.73"
log_info "   3. 修改 config.json 中的 api_key"
log_info ""
log_info "========================================="
