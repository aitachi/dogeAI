#!/bin/bash
# Claude Code 代理客户端快速配置脚本 (Linux)
# 使用方法: bash setup-client-linux.sh

set -e

# ========================================
# 配置参数（公网访问配置）
# ========================================
SERVER_IP="115.190.62.87"
SERVER_PORT="80"  # Nginx 对外端口
API_KEY="sk-ant-api03-CG0BG1GN0D4Q3CRWCREL0DIT2BHZCWSMQUF9SYOCU8V6K6A6Q5GA"
DOMAIN="aitachi.top"  # 可选域名

# ========================================
# 颜色定义
# ========================================
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
RED='\033[0;31m'
NC='\033[0m'

echo -e "${CYAN}========================================${NC}"
echo -e "${CYAN}   Claude Code 代理配置 (Linux)${NC}"
echo -e "${CYAN}========================================${NC}"
echo ""
echo -e "公网IP: ${YELLOW}${SERVER_IP}:${SERVER_PORT}${NC}"
echo -e "域名: ${YELLOW}${DOMAIN}${NC} (如果DNS解析正常)"
echo -e "API Key: ${YELLOW}${API_KEY:0:25}...${NC}"
echo ""

# ========================================
# 1. 检测 Shell 类型
# ========================================
echo -e "${CYAN}[1/5]${NC} 检测 Shell 类型..."
if [ -n "${ZSH_VERSION:-}" ]; then
    RC_FILE="$HOME/.zshrc"
    SHELL_NAME="Zsh"
elif [ -n "${BASH_VERSION:-}" ]; then
    if [[ "$OSTYPE" == "darwin"* ]]; then
        RC_FILE="$HOME/.bash_profile"
    else
        RC_FILE="$HOME/.bashrc"
    fi
    SHELL_NAME="Bash"
else
    RC_FILE="$HOME/.profile"
    SHELL_NAME="Unknown"
fi
echo -e "   检测到: ${GREEN}${SHELL_NAME}${NC}"
echo -e "   配置文件: ${YELLOW}${RC_FILE}${NC}"
echo ""

# ========================================
# 2. 配置 Shell 环境变量
# ========================================
echo -e "${CYAN}[2/5]${NC} 配置环境变量..."

BLOCK_START="# >>> CLAUDE CODE PROXY BEGIN >>>"
BLOCK_END="# <<< CLAUDE CODE PROXY END <<<"

BLOCK_CONTENT=$(cat <<EOF
$BLOCK_START
# Claude Code 代理配置 - 公网访问
export ANTHROPIC_API_KEY="$API_KEY"
export ANTHROPIC_BASE_URL="http://$SERVER_IP"
# 如果域名解析正常，可以使用下面这行代替上面:
# export ANTHROPIC_BASE_URL="https://$DOMAIN"
export ANTHROPIC_AUTH_TOKEN="$API_KEY"
$BLOCK_END
EOF
)

# 移除旧配置（如果存在）
if grep -q "$BLOCK_START" "$RC_FILE" 2>/dev/null; then
    echo -e "   移除旧配置..."
    temp_file="${RC_FILE}.tmp"
    > "$temp_file"

    while IFS= read -r line || [[ -n "$line" ]]; do
        if [[ "$line" == "$BLOCK_START" ]]; then
            echo "$BLOCK_CONTENT" >> "$temp_file"
            while IFS= read -r line; do
                if [[ "$line" == "$BLOCK_END" ]]; then
                    break
                fi
            done
        else
            echo "$line" >> "$temp_file"
        fi
    done < "$RC_FILE"

    mv "$temp_file" "$RC_FILE"
else
    echo -e "   添加新配置..."
    echo "" >> "$RC_FILE"
    echo "$BLOCK_CONTENT" >> "$RC_FILE"
fi

echo -e "   ${GREEN}✓${NC} 环境变量已写入 ${RC_FILE}"
echo ""

# ========================================
# 3. 创建 settings.json
# ========================================
echo -e "${CYAN}[3/5]${NC} 创建 settings.json..."

SETTINGS_DIR="$HOME/.config/claude-code"
mkdir -p "$SETTINGS_DIR"

cat > "$SETTINGS_DIR/settings.json" <<EOF
{
  "env": {
    "ANTHROPIC_API_KEY": "$API_KEY",
    "ANTHROPIC_BASE_URL": "http://$SERVER_IP",
    "ANTHROPIC_AUTH_TOKEN": "$API_KEY"
  }
}
EOF

echo -e "   ${GREEN}✓${NC} settings.json 已创建: ${YELLOW}${SETTINGS_DIR}/settings.json${NC}"
echo ""

# ========================================
# 4. 测试连接
# ========================================
echo -e "${CYAN}[4/5]${NC} 测试服务器连接..."

if command -v curl &> /dev/null; then
    echo -e "   测试公网IP访问..."
    if curl -s --connect-timeout 10 "http://${SERVER_IP}/v1/models" > /dev/null 2>&1; then
        echo -e "   ${GREEN}✓${NC} 公网IP连接正常 (${SERVER_IP})"
    else
        echo -e "   ${RED}✗${NC} 无法连接到公网IP，请检查:"
        echo -e "     - 服务器是否运行"
        echo -e "     - Nginx 是否运行: systemctl status nginx"
        echo -e "     - 防火墙是否开放 80 端口"
    fi

    # 测试域名
    if command -v nslookup &> /dev/null; then
        echo ""
        echo -e "   测试域名解析..."
        if nslookup "$DOMAIN" > /dev/null 2>&1; then
            echo -e "   ${GREEN}✓${NC} 域名 ${DOMAIN} 可以解析"
            echo -e "   ${YELLOW}提示: 如果解析到正确IP，可以在配置中使用域名${NC}"
        else
            echo -e "   ${YELLOW}!${NC} 域名 ${DOMAIN} 无法解析，请使用公网IP"
        fi
    fi
else
    echo -e "   ${YELLOW}!${NC} 未安装 curl，跳过连接测试"
fi
echo ""

# ========================================
# 5. 显示测试命令
# ========================================
echo -e "${CYAN}[5/5]${NC} 测试命令..."
echo ""
echo -e "   ${YELLOW}# 测试模型列表${NC}"
echo "   curl http://${SERVER_IP}/v1/models \\"
echo "     -H \"x-api-key: ${API_KEY:0:20}...\""
echo ""
echo -e "   ${YELLOW}# 测试消息API${NC}"
echo "   curl -X POST http://${SERVER_IP}/v1/messages \\"
echo "     -H \"x-api-key: ${API_KEY:0:20}...\" \\"
echo "     -H \"content-type: application/json\" \\"
echo "     -d '{\"model\":\"claude-sonnet-4-20250514\",\"max_tokens\":50,\"messages\":[{\"role\":\"user\",\"content\":\"Hi\"}]}'"
echo ""

# ========================================
# 完成
# ========================================
echo -e "${CYAN}========================================${NC}"
echo -e "${CYAN}         配置完成！${NC}"
echo -e "${CYAN}========================================${NC}"
echo ""
echo -e "${GREEN}下一步操作:${NC}"
echo ""
echo -e "1. 重新加载配置文件:"
echo -e "   ${GREEN}source ${RC_FILE}${NC}"
echo ""
echo -e "2. 验证配置:"
echo -e "   ${GREEN}echo \$ANTHROPIC_BASE_URL${NC}"
echo ""
echo -e "3. 启动 Claude Code:"
echo -e "   ${GREEN}claude${NC}"
echo ""
echo -e "${YELLOW}注意: 如果使用 Claude Code 时遇到连接问题，请尝试:${NC}"
echo -e "   - 确认网络可以访问 ${SERVER_IP}"
echo -e "   - 使用域名替代IP (如果DNS解析正常)"
echo ""
echo -e "${CYAN}========================================${NC}"
