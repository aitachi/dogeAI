#!/bin/bash
# Anthropic API 代理管理工具

BASE_URL="http://59.110.40.73"
ADMIN_KEY="admin-change-this-key"

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

show_menu() {
    echo ""
    echo "=========================================="
    echo "  Anthropic API 代理管理工具"
    echo "=========================================="
    echo "1. 查看服务状态"
    echo "2. 查看所有API密钥"
    echo "3. 创建新API密钥"
    echo "4. 查看调用统计"
    echo "5. 查看特定用户统计"
    echo "6. 查看实时日志"
    echo "7. 重启服务"
    echo "8. 查看并发情况"
    echo "0. 退出"
    echo "=========================================="
}

check_status() {
    echo -e "${GREEN}📊 服务状态${NC}"
    curl -s "${BASE_URL}/health" | python3 -m json.tool
    echo ""
}

list_keys() {
    echo -e "${GREEN}🔑 所有API密钥${NC}"
    curl -s "${BASE_URL}/admin/keys" \
        -H "x-admin-key: ${ADMIN_KEY}" | python3 -m json.tool
    echo ""
}

create_key() {
    echo -e "${YELLOW}创建新API密钥${NC}"
    read -p "用户名: " user
    read -p "每日限额(tokens): " limit
    read -p "有效天数(0=永不过期): " days

    result=$(curl -s -X POST "${BASE_URL}/admin/keys/create?user=${user}&limit=${limit}&days=${days}" \
        -H "x-admin-key: ${ADMIN_KEY}")

    echo "$result" | python3 -m json.tool

    # 提取token
    token=$(echo "$result" | python3 -c "import sys, json; print(json.load(sys.stdin).get('token', ''))" 2>/dev/null)

    if [ -n "$token" ]; then
        echo -e "${GREEN}✅ 密钥创建成功！${NC}"
        echo "Token: $token"
        echo ""
        echo "客户端配置:"
        echo "export ANTHROPIC_API_KEY=\"$token\""
        echo "export ANTHROPIC_BASE_URL=\"${BASE_URL}\""
    fi
}

show_stats() {
    echo -e "${GREEN}📈 调用统计${NC}"
    read -p "统计天数(默认7): " days
    days=${days:-7}

    curl -s "${BASE_URL}/admin/stats?days=${days}" \
        -H "x-admin-key: ${ADMIN_KEY}" | python3 -m json.tool
    echo ""
}

show_user_stats() {
    echo -e "${GREEN}👤 用户统计${NC}"
    read -p "API Token: " api_key
    read -p "统计天数(默认7): " days
    days=${days:-7}

    curl -s "${BASE_URL}/admin/stats?api_key=${api_key}&days=${days}" \
        -H "x-admin-key: ${ADMIN_KEY}" | python3 -m json.tool
    echo ""
}

show_logs() {
    echo -e "${GREEN}📝 实时日志 (Ctrl+C退出)${NC}"
    tail -f /var/log/anthropic-proxy/api-proxy.log
}

restart_service() {
    echo -e "${YELLOW}🔄 重启服务...${NC}"
    systemctl restart anthropic-proxy
    sleep 2

    if systemctl is-active --quiet anthropic-proxy; then
        echo -e "${GREEN}✅ 服务重启成功${NC}"
        check_status
    else
        echo -e "${RED}❌ 服务重启失败${NC}"
        journalctl -u anthropic-proxy -n 20 --no-pager
    fi
}

show_concurrency() {
    echo -e "${GREEN}⚡ 并发情况${NC}"
    curl -s "${BASE_URL}/health" | python3 -c "
import sys, json
data = json.load(sys.stdin)
print(f\"活跃请求: {data['active_requests']}/{data['max_concurrent']}\")
print(f\"可用槽位: {data['available_slots']}\")
"
    echo ""
}

# 主循环
while true; do
    show_menu
    read -p "请选择 [0-8]: " choice

    case $choice in
        1) check_status ;;
        2) list_keys ;;
        3) create_key ;;
        4) show_stats ;;
        5) show_user_stats ;;
        6) show_logs ;;
        7) restart_service ;;
        8) show_concurrency ;;
        0) echo "再见！"; exit 0 ;;
        *) echo -e "${RED}无效选择${NC}" ;;
    esac

    read -p "按Enter继续..."
done
