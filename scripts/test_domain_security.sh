#!/bin/bash
# 测试域名访问和API key验证

NEW_API_KEY="sk-JvflJvETSAx-50xJ_aAXdXESwfWgQX6bLZrjpJSbeFg"
OLD_API_KEY="sk-invalid-key-for-testing"
DOMAIN="aitachi.cloud"

echo "=========================================="
echo "  域名和安全验证测试"
echo "=========================================="
echo ""
echo "🌐 域名: $DOMAIN"
echo "🔑 新API Key: ${NEW_API_KEY:0:20}..."
echo ""

echo "1️⃣  测试IP访问（应重定向到域名）"
echo "----------------------------------------"
curl -s -I http://59.110.40.73/health | grep -E "HTTP|Location"
echo ""

echo "2️⃣  测试域名访问（健康检查）"
echo "----------------------------------------"
curl -s http://$DOMAIN/health | python3 -m json.tool
echo ""

echo "3️⃣  测试安全验证：正确的API Key"
echo "----------------------------------------"
response=$(curl -s -X POST http://$DOMAIN/v1/messages \
  -H "x-api-key: $NEW_API_KEY" \
  -H "anthropic-version: 2023-06-01" \
  -H "content-type: application/json" \
  -d '{"model":"claude-sonnet-4.5","max_tokens":50,"messages":[{"role":"user","content":"Hi"}]}')

if echo "$response" | grep -q "content"; then
    echo "✅ 正确的API Key：访问成功"
    echo "$response" | python3 -c "import sys,json; d=json.load(sys.stdin); print('📝 AI回复: ' + d['content'][0]['text'][:50] + '...')"
else
    echo "❌ 访问失败"
fi
echo ""

echo "4️⃣  测试安全验证：错误的API Key"
echo "----------------------------------------"
response=$(curl -s -X POST http://$DOMAIN/v1/messages \
  -H "x-api-key: $OLD_API_KEY" \
  -H "content-type: application/json" \
  -d '{"model":"claude-sonnet-4.5","max_tokens":50,"messages":[{"role":"user","content":"Hi"}]}')

if echo "$response" | grep -q "Invalid API key"; then
    echo "✅ 安全验证：正确拦截了错误的API Key"
    echo "🚫 错误信息: $(echo "$response" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d['detail'])" 2>/dev/null)"
else
    echo "⚠️  安全验证可能有问题"
fi
echo ""

echo "5️⃣  测试安全验证：缺少API Key"
echo "----------------------------------------"
response=$(curl -s -X POST http://$DOMAIN/v1/messages \
  -H "content-type: application/json" \
  -d '{"model":"claude-sonnet-4.5","max_tokens":50,"messages":[{"role":"user","content":"Hi"}]}')

if echo "$response" | grep -q "Missing API key"; then
    echo "✅ 安全验证：正确拦截了无API Key的请求"
    echo "🚫 错误信息: $(echo "$response" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d['detail'])" 2>/dev/null)"
else
    echo "⚠️  安全验证可能有问题"
fi
echo ""

echo "6️⃣  测试模型列表（需要API Key）"
echo "----------------------------------------"
response=$(curl -s http://$DOMAIN/v1/models \
  -H "x-api-key: $NEW_API_KEY")

if echo "$response" | grep -q "claude-sonnet-4.5"; then
    echo "✅ 模型列表访问成功"
    echo "$response" | python3 -c "import sys,json; d=json.load(sys.stdin); print('可用模型: ' + ', '.join([m['id'] for m in d['data']]))"
else
    echo "❌ 模型列表访问失败"
fi
echo ""

echo "=========================================="
echo "  ✅ 测试完成！"
echo "=========================================="
