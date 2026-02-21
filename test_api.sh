#!/bin/bash
#
# Claude Code Proxy - 完整API测试脚本
#

set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 测试统计
TOTAL=0
PASSED=0
FAILED=0

# 测试函数
test_api() {
    local name="$1"
    local method="$2"
    local url="$3"
    local data="$4"
    local expected="$5"

    TOTAL=$((TOTAL + 1))
    echo -e "\n${BLUE}测试 $TOTAL: $name${NC}"
    echo "请求: $method $url"

    if [ -z "$data" ]; then
        response=$(curl -s -k -X "$method" "$url" \
            -H "Content-Type: application/json" \
            -H "x-api-key: test-key" 2>&1)
    else
        response=$(curl -s -k -X "$method" "$url" \
            -H "Content-Type: application/json" \
            -H "x-api-key: test-key" \
            -d "$data" 2>&1)
    fi

    # 检查是否包含预期内容
    if echo "$response" | grep -q "$expected"; then
        echo -e "${GREEN}✅ 通过${NC}"
        echo "响应: $(echo "$response" | head -c 100)..."
        PASSED=$((PASSED + 1))
        return 0
    else
        echo -e "${RED}❌ 失败${NC}"
        echo "预期包含: $expected"
        echo "实际响应: $response"
        FAILED=$((FAILED + 1))
        return 1
    fi
}

# 显示测试结果摘要
show_summary() {
    echo ""
    echo "========================================"
    echo -e "📊 测试结果摘要"
    echo "========================================"
    echo -e "总计: ${BLUE}$TOTAL${NC}"
    echo -e "通过: ${GREEN}$PASSED${NC}"
    echo -e "失败: ${RED}$FAILED${NC}"
    echo "========================================"

    if [ $FAILED -eq 0 ]; then
        echo -e "${GREEN}🎉 所有测试通过！${NC}"
        return 0
    else
        echo -e "${RED}⚠️  有 $FAILED 个测试失败${NC}"
        return 1
    fi
}

# ========================================
# 开始测试
# ========================================

echo "========================================"
echo "🧪 Claude Code Proxy - API测试"
echo "========================================"
echo "测试目标: 127.0.0.1"
echo "测试端口: 443 (HTTPS)"
echo "========================================"

# ========================================
# 1. 基础健康检查
# ========================================
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "1️⃣  基础健康检查"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

test_api \
    "健康检查 - /api/hello" \
    "GET" \
    "https://127.0.0.1/api/hello" \
    "" \
    '"message":"hello"'

test_api \
    "健康检查 - 根路径" \
    "GET" \
    "https://127.0.0.1/" \
    "" \
    '"status":"ok"'

# ========================================
# 2. 模型列表
# ========================================
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "2️⃣  模型管理"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

test_api \
    "模型列表" \
    "GET" \
    "https://127.0.0.1/v1/models" \
    "" \
    '"object":"list"'

test_api \
    "模型详情" \
    "GET" \
    "https://127.0.0.1/v1/models/claude-sonnet-4-5-20250929" \
    "" \
    '"id":"claude-sonnet-4-5-20250929"'

# ========================================
# 3. Claude Code专用端点
# ========================================
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "3️⃣  Claude Code端点"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

test_api \
    "Claude Code设置" \
    "GET" \
    "https://127.0.0.1/api/claude_code/settings" \
    "" \
    '"allowed_tools"'

test_api \
    "策略限制" \
    "GET" \
    "https://127.0.0.1/api/claude_code/policy_limits" \
    "" \
    '"rate_limits"'

test_api \
    "Penguin模式" \
    "GET" \
    "https://127.0.0.1/api/claude_code/penguin_mode" \
    "" \
    '"enabled":false'

# ========================================
# 4. 平台API端点
# ========================================
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "4️⃣  平台API"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

test_api \
    "Bootstrap" \
    "GET" \
    "https://127.0.0.1/api/bootstrap" \
    "" \
    '"account"'

test_api \
    "认证信息" \
    "GET" \
    "https://127.0.0.1/api/auth" \
    "" \
    '"account"'

test_api \
    "账户信息" \
    "GET" \
    "https://127.0.0.1/api/account" \
    "" \
    '"email":"proxy@local.dev"'

test_api \
    "组织列表" \
    "GET" \
    "https://127.0.0.1/api/organizations" \
    "" \
    '"data":'

test_api \
    "V1组织" \
    "GET" \
    "https://127.0.0.1/v1/organizations" \
    "" \
    '"data":'

# ========================================
# 5. OAuth端点
# ========================================
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "5️⃣  OAuth认证"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

test_api \
    "OAuth Hello" \
    "GET" \
    "https://127.0.0.1/v1/oauth/hello" \
    "" \
    '"message":"hello"'

test_api \
    "OIDC配置" \
    "GET" \
    "https://127.0.0.1/.well-known/openid-configuration" \
    "" \
    '"issuer":'

# ========================================
# 6. 核心消息API - 非流式
# ========================================
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "6️⃣  核心消息API - 非流式"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

test_api \
    "简单对话 - 1+1=?" \
    "POST" \
    "https://127.0.0.1/v1/messages" \
    '{"model":"claude-sonnet-4-5-20250929","max_tokens":50,"messages":[{"role":"user","content":"1+1=?"}]}' \
    '"1 + 1 = 2"'

test_api \
    "Token计数" \
    "POST" \
    "https://127.0.0.1/v1/messages/count_tokens" \
    '{"model":"claude-sonnet-4-5-20250929","messages":[{"role":"user","content":"hello"}]}' \
    '"input_tokens":'

# ========================================
# 7. 用户信息端点
# ========================================
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "7️⃣  用户信息"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

test_api \
    "用户信息 /api/me" \
    "GET" \
    "https://127.0.0.1/api/me" \
    "" \
    '"uuid":"user_'

test_api \
    "用户信息 /v1/me" \
    "GET" \
    "https://127.0.0.1/v1/me" \
    "" \
    '"uuid":"user_'

test_api \
    "用户信息 /userinfo" \
    "GET" \
    "https://127.0.0.1/userinfo" \
    "" \
    '"uuid":"user_'

# ========================================
# 8. Usage统计
# ========================================
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "8️⃣  Usage统计"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

test_api \
    "账单Usage" \
    "GET" \
    "https://127.0.0.1/v1/dashboard/billing/usage" \
    "" \
    '"daily_costs":'

test_api \
    "Usage" \
    "GET" \
    "https://127.0.0.1/v1/usage" \
    "" \
    '"daily_costs":'

# ========================================
# 9. 工具调用测试
# ========================================
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "9️⃣  工具调用测试"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

test_api \
    "工具调用消息" \
    "POST" \
    "https://127.0.0.1/v1/messages" \
    '{"model":"claude-sonnet-4-5-20250929","max_tokens":100,"tools":[{"name":"calculator","description":"Calculate","input_schema":{"type":"object","properties":{"expression":{"type":"string"}}}}],"messages":[{"role":"user","content":"What is 2+2?"}]}' \
    '"type":"message"'

# ========================================
# 10. 系统提示测试
# ========================================
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🔟 系统提示测试"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

test_api \
    "系统提示词" \
    "POST" \
    "https://127.0.0.1/v1/messages" \
    '{"model":"claude-sonnet-4-5-20250929","max_tokens":50,"system":"You are a helpful assistant. Answer in one word.","messages":[{"role":"user","content":"What is 2+2?"}]}' \
    '"4"'

# ========================================
# 显示结果
# ========================================
show_summary

exit $?
