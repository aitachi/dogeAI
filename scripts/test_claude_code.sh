#!/bin/bash
# 测试Claude Code兼容性

API_KEY="sk-JvflJvETSAx-50xJ_aAXdXESwfWgQX6bLZrjpJSbeFg"
BASE_URL="http://aitachi.cloud"

echo "=========================================="
echo "  Claude Code 兼容性测试"
echo "=========================================="
echo ""
echo "🔑 API Key: ${API_KEY:0:20}..."
echo "🌐 Base URL: $BASE_URL"
echo ""

# 测试1：基本对话（非流式）
echo "1️⃣  测试基本对话（非流式）"
echo "----------------------------------------"
response=$(curl -s -X POST "$BASE_URL/v1/messages" \
  -H "x-api-key: $API_KEY" \
  -H "anthropic-version: 2023-06-01" \
  -H "content-type: application/json" \
  -d '{
    "model": "claude-sonnet-4.5",
    "max_tokens": 100,
    "messages": [{"role": "user", "content": "请用中文简单介绍一下你自己"}]
  }')

echo "$response" | python3 -c "
import sys, json
try:
    d = json.load(sys.stdin)
    if 'content' in d:
        print('✅ 基本对话成功')
        print('📝 回复:', d['content'][0]['text'][:100] + '...')
        print('📊 Tokens:', d['usage']['input_tokens'], '+', d['usage']['output_tokens'], '=', d['usage']['input_tokens'] + d['usage']['output_tokens'])
        print('🤖 返回模型:', d.get('model', 'unknown'))
    else:
        print('❌ 响应格式错误')
        print('响应:', d)
except Exception as e:
    print('❌ 解析错误:', e)
"
echo ""

# 测试2：流式响应
echo "2️⃣  测试流式响应"
echo "----------------------------------------"
response=$(curl -s -X POST "$BASE_URL/v1/messages" \
  -H "x-api-key: $API_KEY" \
  -H "anthropic-version: 2023-06-01" \
  -H "content-type: application/json" \
  -d '{
    "model": "claude-sonnet-4.5",
    "max_tokens": 50,
    "stream": true,
    "messages": [{"role": "user", "content": "Hi"}]
  }' --max-time 10)

if [ -n "$response" ]; then
    if echo "$response" | grep -q "data: "; then
        echo "✅ 流式响应正常"
        echo "📝 流式数据预览:"
        echo "$response" | head -5
    else
        echo "⚠️  流式响应可能有问题"
        echo "响应长度: $(echo "$response" | wc -c) 字节"
    fi
else
    echo "❌ 流式响应超时或失败"
fi
echo ""

# 测试3：模型列表
echo "3️⃣  测试模型列表"
echo "----------------------------------------"
response=$(curl -s "$BASE_URL/v1/models" -H "x-api-key: $API_KEY")
echo "$response" | python3 -c "
import sys, json
try:
    d = json.load(sys.stdin)
    print('✅ 可用模型数量:', len(d['data']))
    for model in d['data'][:5]:
        print('  ✓', model['id'], '-', model['name'])
except Exception as e:
    print('❌ 错误:', e)
"
echo ""

# 测试4：长文本处理
echo "4️⃣  测试长文本处理"
echo "----------------------------------------"
response=$(curl -s -X POST "$BASE_URL/v1/messages" \
  -H "x-api-key: $API_KEY" \
  -H "anthropic-version: 2023-06-01" \
  -H "content-type: application/json" \
  -d '{
    "model": "claude-sonnet-4.5",
    "max_tokens": 300,
    "messages": [{"role": "user", "content": "请详细解释一下什么是人工智能，包括它的定义、主要技术和应用领域"}]
  }')

echo "$response" | python3 -c "
import sys, json
try:
    d = json.load(sys.stdin)
    if 'content' in d:
        text = d['content'][0]['text']
        print('✅ 长文本处理成功')
        print('📝 回复长度:', len(text), '字符')
        print('📊 Tokens:', d['usage']['total_tokens'])
    else:
        print('❌ 响应格式错误')
except Exception as e:
    print('❌ 错误:', e)
"
echo ""

# 测试5：代码生成
echo "5️⃣  测试代码生成"
echo "----------------------------------------"
response=$(curl -s -X POST "$BASE_URL/v1/messages" \
  -H "x-api-key: $API_KEY" \
  -H "anthropic-version: 2023-06-01" \
  -H "content-type: application/json" \
  -d '{
    "model": "claude-sonnet-4.5",
    "max_tokens": 200,
    "messages": [{"role": "user", "content": "请用Python写一个计算斐波那契数列的函数"}]
  }')

echo "$response" | python3 -c "
import sys, json
try:
    d = json.load(sys.stdin)
    if 'content' in d:
        text = d['content'][0]['text']
        print('✅ 代码生成成功')
        print('📝 代码预览:')
        print('-' * 60)
        print(text[:300] + '...')
        print('-' * 60)
    else:
        print('❌ 响应格式错误')
except Exception as e:
    print('❌ 错误:', e)
"
echo ""

echo "=========================================="
echo "  ✅ 测试完成"
echo "=========================================="
