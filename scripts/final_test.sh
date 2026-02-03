#!/bin/bash
# 最终测试 - 无需端口号

BASE_URL="http://59.110.40.73"
API_KEY="sk-dMUtFZ9IWedPSgSd0WMkH7XhP0idZVtAVnp48MYgNLo"

echo "=========================================="
echo "  端口隐藏后测试"
echo "=========================================="
echo ""

echo "✅ 新的访问方式（无需端口号）："
echo "   Base URL: $BASE_URL"
echo ""

echo "1️⃣  健康检查"
echo "----------------------------------------"
curl -s "${BASE_URL}/health" | python3 -m json.tool
echo ""

echo "2️⃣  模型列表"
echo "----------------------------------------"
curl -s "${BASE_URL}/v1/models" | python3 -c "
import sys, json
data = json.load(sys.stdin)
for model in data['data']:
    print(f\"  ✓ {model['id']}\")
"
echo ""

echo "3️⃣  API调用测试（Sonnet 4.5）"
echo "----------------------------------------"
curl -s -X POST "${BASE_URL}/v1/messages" \
  -H "x-api-key: ${API_KEY}" \
  -H "anthropic-version: 2023-06-01" \
  -H "content-type: application/json" \
  -d '{"model":"claude-sonnet-4.5","max_tokens":50,"messages":[{"role":"user","content":"请用中文说：端口隐藏成功！"}]}' | python3 -c "
import sys, json
try:
    data = json.load(sys.stdin)
    if 'content' in data:
        text = data['content'][0]['text']
        print(f\"✅ 调用成功！\")
        print(f\"📝 回复: {text[:80]}...\")
        print(f\"📊 Tokens: {data['usage']['input_tokens']} + {data['usage']['output_tokens']} = {data['usage']['input_tokens'] + data['usage']['output_tokens']}\")
except:
    print('❌ 调用失败')
"
echo ""

echo "4️⃣  备用端口测试（8081仍可用）"
echo "----------------------------------------"
if curl -s -m 5 "http://59.110.40.73:8081/health" > /dev/null 2>&1; then
    echo "✅ 备用端口8081正常工作"
else
    echo "⚠️  备用端口8081无响应"
fi
echo ""

echo "=========================================="
echo "  ✅ 配置完成！"
echo "=========================================="
echo ""
echo "📌 客户端配置更新："
echo ""
echo "Python:"
echo "  client = Anthropic("
echo "    api_key=\"sk-dMUtFZ9IWedPSgSd0WMkH7XhP0idZVtAVnp48MYgNLo\","
echo "    base_url=\"http://59.110.40.73\"  # 无需端口号"
echo "  )"
echo ""
echo "环境变量:"
echo "  export ANTHROPIC_API_KEY=\"sk-dMUtFZ9IWedPSgSd0WMkH7XhP0idZVtAVnp48MYgNLo\""
echo "  export ANTHROPIC_BASE_URL=\"http://59.110.40.73\""
echo ""
echo "=========================================="
