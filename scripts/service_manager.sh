#!/bin/bash
# ========================================
# 服务管理脚本
# ========================================

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

case "$1" in
    start)
        echo -e "${GREEN}启动所有服务...${NC}"
        systemctl start redis
        systemctl start supervisord
        supervisorctl start llm-proxy
        systemctl start nginx
        echo -e "${GREEN}✓ 所有服务已启动${NC}"
        supervisorctl status
        ;;

    stop)
        echo -e "${YELLOW}停止所有服务...${NC}"
        systemctl stop nginx
        supervisorctl stop llm-proxy
        systemctl stop supervisord
        systemctl stop redis
        echo -e "${GREEN}✓ 所有服务已停止${NC}"
        ;;

    restart)
        echo -e "${YELLOW}重启所有服务...${NC}"
        supervisorctl restart llm-proxy
        systemctl restart nginx
        echo -e "${GREEN}✓ 所有服务已重启${NC}"
        supervisorctl status
        ;;

    status)
        echo -e "${GREEN}========== 服务状态 ==========${NC}"
        echo -e "\n${YELLOW}[Redis]${NC}"
        systemctl status redis --no-pager -l
        echo -e "\n${YELLOW}[Supervisor]${NC}"
        supervisorctl status
        echo -e "\n${YELLOW}[Nginx]${NC}"
        systemctl status nginx --no-pager -l
        ;;

    logs)
        echo -e "${GREEN}========== 实时日志 ==========${NC}"
        echo -e "${YELLOW}按Ctrl+C退出${NC}\n"
        tail -f /var/www/llm-proxy/logs/access.log
        ;;

    errors)
        echo -e "${GREEN}========== 错误日志 ==========${NC}"
        echo -e "${YELLOW}按Ctrl+C退出${NC}\n"
        tail -f /var/www/llm-proxy/logs/error.log
        ;;

    test)
        echo -e "${GREEN}========== 测试服务 ==========${NC}"
        echo -e "\n${YELLOW}[1] 健康检查${NC}"
        curl -s http://localhost/health | python3 -m json.tool

        echo -e "\n${YELLOW}[2] 聊天接口测试${NC}"
        curl -s -X POST http://localhost/v1/chat/completions \
          -H "Content-Type: application/json" \
          -d '{"model":"gpt-3.5-turbo","messages":[{"role":"user","content":"你好"}]}' \
          | python3 -m json.tool

        echo -e "\n${YELLOW}[3] Redis连接测试${NC}"
        redis-cli ping

        echo -e "\n${GREEN}✓ 测试完成${NC}"
        ;;

    update)
        echo -e "${GREEN}========== 更新应用 ==========${NC}"
        cd /var/www/llm-proxy
        source venv/bin/activate
        pip install --upgrade fastapi uvicorn httpx redis
        supervisorctl restart llm-proxy
        echo -e "${GREEN}✓ 更新完成${NC}"
        ;;

    *)
        echo "大模型API代理服务管理脚本"
        echo ""
        echo "用法: $0 {start|stop|restart|status|logs|errors|test|update}"
        echo ""
        echo "命令说明:"
        echo "  start   - 启动所有服务"
        echo "  stop    - 停止所有服务"
        echo "  restart - 重启所有服务"
        echo "  status  - 查看服务状态"
        echo "  logs    - 查看访问日志（实时）"
        echo "  errors  - 查看错误日志（实时）"
        echo "  test    - 测试服务接口"
        echo "  update  - 更新Python依赖"
        exit 1
        ;;
esac
