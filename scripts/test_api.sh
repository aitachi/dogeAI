#!/bin/bash
# API代理服务完整测试

BASE_URL="http://59.110.40.73"
API_KEY="sk-dMUtFZ9IWedPSgSd0WMkH7XhP0idZVtAVnp48MYgNLo"
ADMIN_KEY="admin-change-this-key"

echo "=========================================="
echo "  Anthropic API 代理服务测试"
echo "=========================================="
echo ""

# 1. 健康检查
echo "1️⃣  健康检查"
echo "----------------------------------------"
curl -s "${BASE_URL}/health" | python3 -m json.tool
echo ""

# 2. 模型列表
echo "2️⃣  模型列表"
echo "----------------------------------------"
curl -s "${BASE_URL}/v1/models" | python3 -m json.tool
echo ""

# 3. Sonnet 4.5 测试
echo "3️⃣  Sonnet 4.5 测试"
echo "----------------------------------------"
curl -s -X POST "${BASE_URL}/v1/messages" \
  -H "x-api-key: ${API_KEY}" \
  -H "anthropic-version: 2023-06-01" \
  -H "content-type: application/json" \
  -d '{"model":"claude-sonnet-4.5","max_tokens":50,"messages":[{"role":"user","content":"用中文说你好"}]}' | python3 -c "
import sys, json
try:
    data = json.load(sys.stdin)
    if 'content' in data:
        text = data['content'][0]['text']
        print(f'✅ 成功: {text[:100]}...')
    else:
        print('❌ 失败')
except:
    print('❌ 解析错误')
"
echo ""

# 4. Opus 4.5 测试
echo "4️⃣  Opus 4.5 测试"
echo "----------------------------------------"
curl -s -X POST "${BASE_URL}/v1/messages" \
  -H "x-api-key: ${API_KEY}" \
  -H "anthropic-version: 2023-06-01" \
  -H "content-type: application/json" \
  -d '{"model":"claude-opus-4.5","max_tokens":50,"messages":[{"role":"user","content":"Hi"}]}' | python3 -c "
import sys, json
try:
    data = json.load(sys.stdin)
    if 'content' in data:
        print('✅ 成功')
    else:
        print('❌ 失败')
except:
    print('❌ 解析错误')
"
echo ""

# 5. 统计信息
echo "5️⃣  调用统计"
echo "----------------------------------------"
curl -s "${BASE_URL}/admin/stats?days=1" \
  -H "x-admin-key: ${ADMIN_KEY}" | python3 -c "
import sys, json
try:
    data = json.load(sys.stdin)
    print(f\"总调用次数: {data['global']['total_calls']}\")
    print(f\"总Token数: {data['global']['total_tokens']}\")
except:
    print('❌ 获取统计失败')
"
echo ""

# 6. 并发测试
echo "6️⃣  并发能力"
echo "----------------------------------------"
curl -s "${BASE_URL}/health" | python3 -c "
import sys, json
try:
    data = json.load(sys.stdin)
    print(f\"最大并发: {data['max_concurrent']}\")
    print(f\"当前活跃: {data['active_requests']}\")
    print(f\"可用槽位: {data['available_slots']}\")
except:
    print('❌ 获取并发信息失败')
"
echo ""

echo "=========================================="
echo "  测试完成！"
echo "=========================================="
