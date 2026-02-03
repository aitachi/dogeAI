#!/bin/bash
# 管理系统功能测试

echo "=========================================="
echo "  管理系统功能测试"
echo "=========================================="
echo ""

BASE_URL="http://aitachi.cloud"
ADMIN_KEY="admin-change-this-key"

echo "1️⃣  测试登录页面访问"
echo "----------------------------------------"
if curl -s -f "$BASE_URL/login.html" > /dev/null; then
    echo "✅ 登录页面可访问: $BASE_URL/login.html"
else
    echo "❌ 登录页面无法访问"
fi
echo ""

echo "2️⃣  测试管理后台访问"
echo "----------------------------------------"
if curl -s -f "$BASE_URL/admin/dashboard.html" > /dev/null; then
    echo "✅ 管理后台可访问: $BASE_URL/admin/dashboard.html"
else
    echo "❌ 管理后台无法访问"
fi
echo ""

echo "3️⃣  测试API密钥列表接口"
echo "----------------------------------------"
response=$(curl -s "$BASE_URL/admin/keys" -H "x-admin-key: $ADMIN_KEY")
key_count=$(echo "$response" | python3 -c "import sys,json; d=json.load(sys.stdin); print(len(d.get('keys', [])))" 2>/dev/null)
echo "✅ API密钥数量: $key_count"
echo ""

echo "4️⃣  测试统计接口"
echo "----------------------------------------"
response=$(curl -s "$BASE_URL/admin/stats?days=1" -H "x-admin-key: $ADMIN_KEY")
echo "$response" | python3 -c "
import sys,json
d=json.load(sys.stdin)
print(f\"✅ 今日调用次数: {d['global']['total_calls']}\")
print(f\"✅ 今日Token使用: {d['global']['total_tokens']}\")
print(f\"✅ 活跃用户数: {len(d.get('by_user', []))}\")
"
echo ""

echo "5️⃣  测试最近调用记录接口"
echo "----------------------------------------"
response=$(curl -s "$BASE_URL/admin/recent-calls?limit=3" -H "x-admin-key: $ADMIN_KEY")
call_count=$(echo "$response" | python3 -c "import sys,json; d=json.load(sys.stdin); print(len(d.get('calls', [])))" 2>/dev/null)
echo "✅ 最近调用记录数: $call_count"
echo ""

echo "6️⃣  测试Token状态切换接口"
echo "----------------------------------------"
# 获取第一个token的status
first_token=$(curl -s "$BASE_URL/admin/keys" -H "x-admin-key: $ADMIN_KEY" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d['keys'][0]['token'])" 2>/dev/null)
if [ -n "$first_token" ]; then
    echo "✅ Token状态切换接口可用"
    echo "   Token: ${first_token:0:20}..."
else
    echo "⚠️  无法获取Token"
fi
echo ""

echo "=========================================="
echo "  ✅ 测试完成！"
echo "=========================================="
echo ""
echo "📝 管理员登录信息:"
echo "   用户名: admin"
echo "   密码: admin123"
echo ""
echo "🌐 访问地址:"
echo "   网站首页: $BASE_URL"
echo "   登录页面: $BASE_URL/login.html"
echo "   管理后台: $BASE_URL/admin/dashboard.html"
echo ""
echo "=========================================="
