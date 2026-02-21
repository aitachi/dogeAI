#!/bin/bash
#
# 快速测试 - 验证核心功能
#

echo "========================================"
echo "🚀 快速验证测试"
echo "========================================"

echo ""
echo "1️⃣  服务状态"
echo "----------------------------------------"
systemctl is-active claude-proxy && echo "✅ 代理服务运行中" || echo "❌ 代理服务未运行"
systemctl is-active nginx && echo "✅ Nginx运行中" || echo "❌ Nginx未运行"

echo ""
echo "2️⃣  端口监听"
echo "----------------------------------------"
ss -tlnp | grep ":3001 " > /dev/null && echo "✅ 3001端口监听中" || echo "❌ 3001端口未监听"
ss -tlnp | grep ":443 " > /dev/null && echo "✅ 443端口监听中" || echo "❌ 443端口未监听"

echo ""
echo "3️⃣  健康检查"
echo "----------------------------------------"
response=$(curl -s -k https://127.0.0.1/api/hello 2>&1)
if echo "$response" | grep -q '"message":"hello"'; then
    echo "✅ 健康检查通过"
else
    echo "❌ 健康检查失败: $response"
fi

echo ""
echo "4️⃣  模型列表"
echo "----------------------------------------"
models=$(curl -s -k https://127.0.0.1/v1/models 2>&1 | grep -o '"id"' | wc -l)
echo "✅ 返回 $models 个模型"

echo ""
echo "5️⃣  核心API"
echo "----------------------------------------"
response=$(curl -s -k -X POST https://127.0.0.1/v1/messages \
  -H "Content-Type: application/json" \
  -d '{"model":"claude-sonnet-4-5-20250929","max_tokens":20,"messages":[{"role":"user","content":"hi"}]}' 2>&1)
if echo "$response" | grep -q '"content"'; then
    echo "✅ 核心API正常"
    echo "$response" | python3.11 -c "import sys,json; d=json.load(sys.stdin); print(f'   响应: {d[\"content\"][0][\"text\"][:50]}...')" 2>/dev/null || echo "   响应: $(echo $response | head -c 80)..."
else
    echo "❌ 核心API失败"
fi

echo ""
echo "6️⃣  外网访问"
echo "----------------------------------------"
http_code=$(curl -s -o /dev/null -w "%{http_code}" http://59.110.40.73:3001/api/hello 2>&1)
https_code=$(curl -s -o /dev/null -w "%{http_code}" -k https://59.110.40.73/api/hello 2>&1)

if [ "$http_code" = "200" ]; then
    echo "✅ HTTP外网可访问 (3001端口)"
else
    echo "❌ HTTP外网不可访问 (代码: $http_code)"
fi

if [ "$https_code" = "200" ]; then
    echo "✅ HTTPS外网可访问 (443端口)"
else
    echo "❌ HTTPS外网不可访问 (代码: $https_code)"
fi

echo ""
echo "========================================"
echo "✅ 快速验证完成"
echo "========================================"
echo ""
echo "运行完整测试: bash test_api.sh"
echo "查看文档: cat TEST_GUIDE.md"
