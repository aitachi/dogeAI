#!/bin/bash
# 验证公开API密钥

API_KEY="sk-jU4cYDIDYoUgL0_2tQqdL1dCxrN2q-T9Qd3sGk4dMVw"
BASE_URL="http://59.110.40.73"

echo "=========================================="
echo "  公开API密钥验证"
echo "=========================================="
echo ""
echo "🔑 API Key: ${API_KEY:0:20}..."
echo "🌐 Base URL: $BASE_URL"
echo ""

echo "1️⃣  健康检查"
echo "----------------------------------------"
curl -s "$BASE_URL/health" | python3 -m json.tool
echo ""

echo "2️⃣  模型列表"
echo "----------------------------------------"
curl -s "$BASE_URL/v1/models" | python3 -c "
import sys, json
data = json.load(sys.stdin)
print(f\"可用模型数量: {len(data['data'])}\")
for model in data['data']:
    print(f\"  ✓ {model['id']} - {model['name']}\")
"
echo ""

echo "3️⃣  API调用测试（Sonnet 4.5）"
echo "----------------------------------------"
response=$(curl -s -X POST "$BASE_URL/v1/messages" \
  -H "x-api-key: $API_KEY" \
  -H "anthropic-version: 2023-06-01" \
  -H "content-type: application/json" \
  -d '{"model":"claude-sonnet-4.5","max_tokens":100,"messages":[{"role":"user","content":"请用中文简短介绍你自己，说明你通过API代理服务运行"}]}' )

echo "$response" | python3 -c "
import sys, json
try:
    data = json.load(sys.stdin)
    if 'content' in data:
        text = data['content'][0]['text']
        print('✅ API调用成功！')
        print('')
        print('📝 AI回复:')
        print('-' * 60)
        print(text[:200] + '...')
        print('-' * 60)
        print('')
        print('📊 Token使用:')
        print(f\"   输入: {data['usage']['input_tokens']}\")
        print(f\"   输出: {data['usage']['output_tokens']}\")
        print(f\"   总计: {data['usage']['input_tokens'] + data['usage']['output_tokens']}\")
    else:
        print('❌ 响应格式错误')
except Exception as e:
    print(f'❌ 错误: {e}')
"
echo ""

echo "4️⃣  查看密钥限额"
echo "----------------------------------------"
curl -s "http://127.0.0.1:8080/admin/keys" \
  -H "x-admin-key: admin-change-this-key" | python3 -c "
import sys, json
data = json.load(sys.stdin)
for key in data['keys']:
    if 'claude_code_public' in key.get('token', ''):
        print(f\"用户: {key['user']}\")
        print(f\"每日限额: {key['daily_limit']:,} tokens\")
        print(f\"有效期: {key['expires_at']}\")
        print(f\"状态: {'✅ 激活' if key['is_active'] else '❌ 禁用'}\")
"
echo ""

echo "=========================================="
echo "  ✅ 密钥验证完成！"
echo "=========================================="
echo ""
echo "📤 分发文件："
echo "   1. /root/CLIENT-SETUP-GUIDE.md     (完整指南)"
echo "   2. /root/setup-client.sh            (配置脚本)"
echo "   3. /root/SHARE-INSTRUCTIONS.md     (分享说明)"
echo ""
echo "📧 用户需要的信息："
echo "   API Key: sk-jU4cYDIDYoUgL0_2tQqdL1dCxrN2q-T9Qd3sGk4dMVw"
echo "   Base URL: http://59.110.40.73"
echo ""
echo "=========================================="
