#!/bin/bash
# ========================================
# 生成新的客户端Token
# ========================================

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${YELLOW}========================================${NC}"
echo -e "${YELLOW}  生成新的客户端Token${NC}"
echo -e "${YELLOW}========================================${NC}"
echo ""

read -p "用户名: " user
read -p "每日限额 (token数，默认100000): " limit
read -p "有效期 (天数，默认365天): " days

limit=${limit:-100000}
days=${days:-365}

echo ""
echo -e "${GREEN}生成Token中...${NC}"

# 调用API生成token
response=$(curl -s -X POST "http://localhost:8080/admin/tokens/generate?user=${user}&limit=${limit}&days=${days}&admin_key=admin-change-this-key")

echo ""
echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}✓ Token生成成功！${NC}"
echo -e "${GREEN}========================================${NC}"
echo ""

echo "$response" | python3 -m json.tool

echo ""
echo -e "${YELLOW}客户端配置:${NC}"
echo ""
echo "export ANTHROPIC_API_KEY=\"$(echo $response | python3 -c "import sys, json; print(json.load(sys.stdin)['token'])")\""
echo "export ANTHROPIC_BASE_URL=\"http://59.110.40.73:8081\""
echo ""
