#!/usr/bin/env bash
set -euo pipefail

# ========================================
# Claude Code 一键配置脚本
# 用于配置 API 网关作为 Claude Code 的代理
# ========================================

# 默认配置
DEFAULT_SERVER_ADDR="0.0.0.0:8080"
DEFAULT_API_KEY=""  # 用户需要提供自己的 API Key

# 颜色定义
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
RED='\033[0;31m'
NC='\033[0m'

echo -e "${CYAN}========================================${NC}"
echo -e "${CYAN}   Claude Code 代理配置工具${NC}"
echo -e "${CYAN}========================================${NC}"
echo ""

# 检查 API 网关是否已运行
check_gateway() {
    if curl -s http://localhost:8081/health > /dev/null 2>&1; then
        return 0
    else
        return 1
    fi
}

# 启动 API 网关
start_gateway() {
    echo -e "${YELLOW}[INFO]${NC} 启动 API 网关..."

    cd /root/api-gateway-simple

    # 检查 Docker 是否可用
    if command -v docker &> /dev/null; then
        echo -e "${YELLOW}[INFO]${NC} 使用 Docker 启动..."
        docker-compose up -d
    else
        echo -e "${YELLOW}[INFO]${NC} 使用本地启动..."
        cargo build --release 2>/dev/null || {
            echo -e "${RED}[ERROR]${NC} 编译失败，请检查 Rust 环境"
            exit 1
        }
        ./target/release/api-gateway-simple &
    fi

    # 等待服务启动
    echo -e "${YELLOW}[INFO]${NC} 等待服务启动..."
    for i in {1..30}; do
        if check_gateway; then
            echo -e "${GREEN}[SUCCESS]${NC} API 网关已启动"
            return 0
        fi
        sleep 1
    done

    echo -e "${RED}[ERROR]${NC} API 网关启动失败"
    return 1
}

# 生成或获取 API Key
setup_api_key() {
    echo ""
    echo -e "${CYAN}========== API Key 设置 ==========${NC}"
    echo ""

    # 检查是否已有 API Key
    existing_key=$(psql -h localhost -U postgres -d api_gateway -t -c "SELECT api_key FROM user_api_keys LIMIT 1;" 2>/dev/null | tr -d ' ')

    if [ -n "$existing_key" ]; then
        echo -e "${GREEN}[INFO]${NC} 检测到已有 API Key:"
        echo -e "${YELLOW}$existing_key${NC}"
        echo ""
        read -p "$(echo -e ${CYAN}"是否使用此 API Key? [Y/n]: "${NC})" use_existing
        if [[ ! "$use_existing" =~ ^[Nn]$ ]]; then
            API_KEY="$existing_key"
            return 0
        fi
    fi

    # 生成新的 API Key
    echo -e "${YELLOW}[INFO]${NC} 生成新的 Anthropic 格式 API Key..."

    # 生成 52 位随机字符串
    random_string=$(cat /dev/urandom | tr -dc 'ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789' | fold -w 52 | head -n 1)
    API_KEY="sk-ant-api03-${random_string}"

    echo -e "${GREEN}[SUCCESS]${NC} API Key 已生成:"
    echo -e "${YELLOW}$API_KEY${NC}"
    echo ""

    # 保存到文件
    echo "$API_KEY" > /root/api-gateway-simple/.api_key
    chmod 600 /root/api-gateway-simple/.api_key

    echo -e "${YELLOW}[INFO]${NC} API Key 已保存到: /root/api-gateway-simple/.api_key"
}

# 配置 Claude Code
configure_claude_code() {
    echo ""
    echo -e "${CYAN}========== Claude Code 配置 ==========${NC}"
    echo ""

    # 获取服务器地址
    if [ -z "${SERVER_ADDR:-}" ]; then
        # 自动检测公网 IP
        PUBLIC_IP=$(curl -s ifconfig.me 2>/dev/null || curl -s icanhazip.com 2>/dev/null || echo "YOUR_SERVER_IP")
        read -p "$(echo -e ${CYAN}"服务器地址 [默认: $PUBLIC_IP:8081]: "${NC})" server_input
        SERVER_ADDR=${server_input:-"$PUBLIC_IP:8081"}
    fi

    # 解析服务器地址
    if [[ "$SERVER_ADDR" == *":"* ]]; then
        SERVER_HOST=$(echo "$SERVER_ADDR" | cut -d: -f1)
        SERVER_PORT=$(echo "$SERVER_ADDR" | cut -d: -f2)
    else
        SERVER_HOST="$SERVER_ADDR"
        SERVER_PORT="8081"
    fi

    BASE_URL="http://${SERVER_HOST}:${SERVER_PORT}"

    echo ""
    echo -e "${YELLOW}[INFO]${NC} 配置信息:"
    echo -e "  服务器: ${CYAN}${SERVER_HOST}:${SERVER_PORT}${NC}"
    echo -e "  API 地址: ${CYAN}${BASE_URL}${NC}"
    echo ""

    # 检测 Shell 类型
    if [ -n "${ZSH_VERSION:-}" ]; then
        RC_FILE="$HOME/.zshrc"
    elif [ -n "${BASH_VERSION:-}" ]; then
        if [[ "$OSTYPE" == "darwin"* ]]; then
            RC_FILE="$HOME/.bash_profile"
        else
            RC_FILE="$HOME/.bashrc"
        fi
    else
        RC_FILE="$HOME/.profile"
    fi

    echo -e "${YELLOW}[INFO]${NC} 配置文件: $RC_FILE"
    echo ""

    # 写入配置
    BLOCK_START="# >>> CLAUDE CODE GATEWAY BEGIN >>>"
    BLOCK_END="# <<< CLAUDE CODE GATEWAY END <<<"

    BLOCK_CONTENT=$(cat <<EOF
$BLOCK_START
# Claude Code 代理配置
export ANTHROPIC_API_KEY="$API_KEY"
export ANTHROPIC_BASE_URL="$BASE_URL"
export ANTHROPIC_AUTH_TOKEN="$API_KEY"
$BLOCK_END
EOF
)

    # 检查并更新配置文件
    if grep -q "$BLOCK_START" "$RC_FILE" 2>/dev/null; then
        echo "更新现有配置..."
        TEMP_FILE="$RC_FILE.tmp"
        > "$TEMP_FILE"

        while IFS= read -r line || [[ -n "$line" ]]; do
            if [[ "$line" == "$BLOCK_START" ]]; then
                echo "$BLOCK_CONTENT" >> "$TEMP_FILE"
                while IFS= read -r line; do
                    if [[ "$line" == "$BLOCK_END" ]]; then
                        break
                    fi
                done
            else
                echo "$line" >> "$TEMP_FILE"
            fi
        done < "$RC_FILE"

        mv "$TEMP_FILE" "$RC_FILE"
    else
        echo "添加新配置..."
        echo "" >> "$RC_FILE"
        echo "$BLOCK_CONTENT" >> "$RC_FILE"
    fi

    echo -e "${GREEN}[SUCCESS]${NC} 配置已写入 $RC_FILE"
    echo ""

    # 同时创建 settings.json
    SETTINGS_DIR="$HOME/.config/claude-code"
    mkdir -p "$SETTINGS_DIR"

    cat > "$SETTINGS_DIR/settings.json" <<EOF
{
  "env": {
    "ANTHROPIC_API_KEY": "$API_KEY",
    "ANTHROPIC_BASE_URL": "$BASE_URL",
    "ANTHROPIC_AUTH_TOKEN": "$API_KEY"
  }
}
EOF

    echo -e "${GREEN}[SUCCESS]${NC} settings.json 已创建: $SETTINGS_DIR/settings.json"
}

# 测试连接
test_connection() {
    echo ""
    echo -e "${CYAN}========== 连接测试 ==========${NC}"
    echo ""

    # 测试健康检查
    echo -e "${YELLOW}[INFO]${NC} 测试健康检查..."
    if curl -s http://localhost:8081/health | grep -q "healthy"; then
        echo -e "${GREEN}[SUCCESS]${NC} 健康检查通过"
    else
        echo -e "${RED}[ERROR]${NC} 健康检查失败"
        return 1
    fi

    # 测试模型列表
    echo -e "${YELLOW}[INFO]${NC} 测试模型列表..."
    if curl -s http://localhost:8081/v1/models -H "x-api-key: $API_KEY" | grep -q "claude"; then
        echo -e "${GREEN}[SUCCESS]${NC} 模型列表获取成功"
    else
        echo -e "${RED}[ERROR]${NC} 模型列表获取失败"
        return 1
    fi

    # 测试 Messages API
    echo -e "${YELLOW}[INFO]${NC} 测试 Messages API..."
    response=$(curl -s -X POST http://localhost:8081/v1/messages \
        -H "x-api-key: $API_KEY" \
        -H "anthropic-version: 2023-06-01" \
        -H "content-type: application/json" \
        -d '{
            "model": "claude-sonnet-4-20250514",
            "max_tokens": 50,
            "messages": [{"role": "user", "content": "Hello"}]
        }')

    if echo "$response" | grep -q "Hello"; then
        echo -e "${GREEN}[SUCCESS]${NC} Messages API 测试成功"
    else
        echo -e "${RED}[ERROR]${NC} Messages API 测试失败"
        echo "Response: $response"
        return 1
    fi

    echo ""
    echo -e "${GREEN}[SUCCESS]${NC} 所有测试通过！"
}

# 显示使用说明
show_instructions() {
    echo ""
    echo -e "${CYAN}========================================${NC}"
    echo -e "${CYAN}         配置完成！${NC}"
    echo -e "${CYAN}========================================${NC}"
    echo ""
    echo -e "${GREEN}你的 API Key:${NC}"
    echo -e "${YELLOW}   $API_KEY${NC}"
    echo ""
    echo -e "${GREEN}代理地址:${NC}"
    echo -e "${YELLOW}   $BASE_URL${NC}"
    echo ""
    echo -e "${CYAN}下一步操作:${NC}"
    echo ""
    echo -e "1. 重新加载配置文件:"
    if [ -n "${ZSH_VERSION:-}" ]; then
        echo -e "   ${GREEN}source ~/.zshrc${NC}"
    elif [[ "$OSTYPE" == "darwin"* ]]; then
        echo -e "   ${GREEN}source ~/.bash_profile${NC}"
    else
        echo -e "   ${GREEN}source ~/.bashrc${NC}"
    fi
    echo ""
    echo -e "2. 启动 Claude Code:"
    echo -e "   ${GREEN}claude${NC}"
    echo ""
    echo -e "3. 验证配置:"
    echo -e "   ${GREEN}echo \$ANTHROPIC_BASE_URL${NC}"
    echo ""
    echo -e "${CYAN}========================================${NC}"
}

# ========== 主流程 ==========

main() {
    # 1. 检查并启动网关
    if ! check_gateway; then
        echo -e "${YELLOW}[INFO]${NC} API 网关未运行，正在启动..."
        start_gateway || exit 1
    else
        echo -e "${GREEN}[INFO]${NC} API 网关已运行"
    fi

    # 2. 设置 API Key
    setup_api_key

    # 3. 配置 Claude Code
    configure_claude_code

    # 4. 测试连接
    test_connection || {
        echo ""
        echo -e "${RED}[ERROR]${NC} 连接测试失败，请检查配置"
        exit 1
    }

    # 5. 显示使用说明
    show_instructions
}

main "$@"
