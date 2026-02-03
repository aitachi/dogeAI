#!/bin/bash
# Token中转平台 - 一键部署脚本

set -e

echo "╔════════════════════════════════════════════════════════════════════╗"
echo "║                                                                  ║"
echo "║              Token中转平台 - 自动部署脚本 v2.0                    ║"
echo "║                                                                  ║"
echo "╚════════════════════════════════════════════════════════════════════╝"
echo ""

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# 检查是否为root
if [ "$EUID" -ne 0 ]; then
    echo -e "${RED}错误: 请使用root用户运行此脚本${NC}"
    exit 1
fi

# 1. 检查系统环境
echo -e "${GREEN}[1/8] 检查系统环境...${NC}"

# 检查Python
if ! command -v python3 &> /dev/null; then
    echo -e "${RED}错误: 未安装Python3${NC}"
    exit 1
fi
echo -e "  ✅ Python3: $(python3 --version)"

# 检查pip
if ! command -v pip3 &> /dev/null; then
    echo -e "${YELLOW}警告: 未找到pip3，尝试安装...${NC}"
    dnf install -y python3-pip || yum install -y python3-pip || apt-get install -y python3-pip
fi

# 2. 安装依赖
echo -e "${GREEN}[2/8] 安装Python依赖...${NC}"
pip3 install -q fastapi uvicorn httpx pydantic || {
    echo -e "${RED}错误: 依赖安装失败${NC}"
    exit 1
}
echo -e "  ✅ 依赖安装完成"

# 3. 创建必要目录
echo -e "${GREEN}[3/8] 创建目录结构...${NC}"
mkdir -p /var/lib/token-gateway
mkdir -p /var/log/token-gateway
chmod 755 /var/lib/token-gateway
chmod 755 /var/log/token-gateway
echo -e "  ✅ 目录创建完成"

# 4. 初始化数据库
echo -e "${GREEN}[4/8] 初始化数据库...${NC}"
python3 /root/init_token_gateway.py --init
echo -e "  ✅ 数据库初始化完成"

# 5. 设置权限
echo -e "${GREEN}[5/8] 设置文件权限...${NC}"
chmod +x /root/token_gateway_v2.py
chmod +x /root/init_token_gateway.py
chmod +x /root/token_gateway_manager.py
chmod 644 /etc/systemd/system/token-gateway.service
echo -e "  ✅ 权限设置完成"

# 6. 停止旧服务（如果存在）
echo -e "${GREEN}[6/8] 检查旧服务...${NC}"
if systemctl is-active --quiet anthropic-proxy.service 2>/dev/null; then
    echo -e "  ${YELLOW}停止旧的anthropic-proxy服务...${NC}"
    systemctl stop anthropic-proxy.service
    systemctl disable anthropic-proxy.service
fi
echo -e "  ✅ 服务检查完成"

# 7. 启动新服务
echo -e "${GREEN}[7/8] 启动Token Gateway服务...${NC}"
systemctl daemon-reload
systemctl enable token-gateway.service
systemctl start token-gateway.service

sleep 3

if systemctl is-active --quiet token-gateway.service; then
    echo -e "  ✅ 服务启动成功"
else
    echo -e "${RED}  ❌ 服务启动失败${NC}"
    systemctl status token-gateway.service
    exit 1
fi

# 8. 显示状态
echo -e "${GREEN}[8/8] 服务状态...${NC}"
systemctl status token-gateway.service --no-pager -l | head -15

echo ""
echo "╔════════════════════════════════════════════════════════════════════╗"
echo "║                     部署完成！                                    ║"
echo "╚════════════════════════════════════════════════════════════════════╝"
echo ""
echo -e "${GREEN}服务信息:${NC}"
echo "  • 服务名称: token-gateway.service"
echo "  • 服务地址: http://0.0.0.0:8000"
echo "  • API文档: http://localhost:8000/docs"
echo ""
echo -e "${GREEN}常用命令:${NC}"
echo "  • 查看状态: systemctl status token-gateway"
echo "  • 启动服务: systemctl start token-gateway"
echo "  • 停止服务: systemctl stop token-gateway"
echo "  • 重启服务: systemctl restart token-gateway"
echo "  • 查看日志: journalctl -u token-gateway -f"
echo ""
echo -e "${GREEN}管理工具:${NC}"
echo "  • 系统状态: python3 /root/token_gateway_manager.py --status"
echo "  • Top Keys: python3 /root/token_gateway_manager.py --top-keys"
echo "  • 错误日志: python3 /root/token_gateway_manager.py --errors"
echo "  • 实时监控: python3 /root/token_gateway_manager.py --watch"
echo ""
echo -e "${YELLOW}下一步操作:${NC}"
echo "  1. 创建管理员Token:"
echo "     python3 /root/init_token_gateway.py --create-admin admin"
echo ""
echo "  2. 添加后端API Keys:"
echo "     python3 /root/init_token_gateway.py --add-key NAME API_KEY WEIGHT PRIORITY"
echo ""
echo "  3. 查看系统状态:"
echo "     python3 /root/token_gateway_manager.py --status"
echo ""
