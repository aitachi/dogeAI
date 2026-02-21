#!/bin/bash
#
# Claude Code Proxy - 服务管理脚本
#

SCRIPT_DIR="/root/dogeAI"
SERVICE_NAME="claude-proxy"

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

case "$1" in
    start)
        echo -e "${GREEN}启动 Claude Code Proxy 服务...${NC}"
        systemctl start $SERVICE_NAME
        sleep 2
        systemctl status $SERVICE_NAME --no-pager | /usr/bin/head -10
        ;;

    stop)
        echo -e "${YELLOW}停止 Claude Code Proxy 服务...${NC}"
        systemctl stop $SERVICE_NAME
        echo "服务已停止"
        ;;

    restart)
        echo -e "${YELLOW}重启 Claude Code Proxy 服务...${NC}"
        systemctl restart $SERVICE_NAME
        sleep 2
        systemctl status $SERVICE_NAME --no-pager | /usr/bin/head -10
        ;;

    status)
        echo -e "${GREEN}Claude Code Proxy 服务状态:${NC}"
        systemctl status $SERVICE_NAME --no-pager
        ;;

    enable)
        echo -e "${GREEN}设置开机自启...${NC}"
        systemctl enable $SERVICE_NAME
        echo "已设置为开机自启"
        ;;

    disable)
        echo -e "${YELLOW}取消开机自启...${NC}"
        systemctl disable $SERVICE_NAME
        echo "已取消开机自启"
        ;;

    logs)
        echo -e "${GREEN}最近日志 (按Ctrl+C退出):${NC}"
        journalctl -u $SERVICE_NAME -f
        ;;

    log)
        echo -e "${GREEN}最近50条日志:${NC}"
        journalctl -u $SERVICE_NAME -n 50 --no-pager
        ;;

    test)
        echo -e "${GREEN}测试服务端点:${NC}"
        echo ""
        echo "1. 健康检查:"
        curl -s http://127.0.0.1:3001/api/hello
        echo ""
        echo ""
        echo "2. 服务状态:"
        curl -s http://127.0.0.1:3001/ | grep status
        echo ""
        echo ""
        echo "3. 模型列表:"
        curl -s http://127.0.0.1:3001/v1/models | grep -o '"id"' | wc -l
        echo "个模型"
        echo ""
        ;;

    *)
        echo "Claude Code Proxy 服务管理"
        echo ""
        echo "用法: $0 {start|stop|restart|status|enable|disable|logs|log|test}"
        echo ""
        echo "命令说明:"
        echo "  start    - 启动服务"
        echo "  stop     - 停止服务"
        echo "  restart  - 重启服务"
        echo "  status   - 查看服务状态"
        echo "  enable   - 设置开机自启"
        echo "  disable  - 取消开机自启"
        echo "  logs     - 实时查看日志"
        echo "  log      - 查看最近50条日志"
        echo "  test     - 测试服务端点"
        echo ""
        exit 1
        ;;
esac

exit 0
