#!/bin/bash
# Claude Code 客户端一键配置脚本

API_KEY="sk-jU4cYDIDYoUgL0_2tQqdL1dCxrN2q-T9Qd3sGk4dMVw"
BASE_URL="http://59.110.40.73"

echo "=========================================="
echo "  Claude Code 客户端配置工具"
echo "=========================================="
echo ""
echo "📋 配置信息："
echo "   API Key: ${API_KEY:0:20}..."
echo "   Base URL: $BASE_URL"
echo ""

# 检测操作系统
if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    OS="linux"
elif [[ "$OSTYPE" == "darwin"* ]]; then
    OS="macos"
else
    OS="windows"
fi

echo "🖥️  检测到操作系统: $OS"
echo ""

# 选择shell
if [ "$OS" != "windows" ]; then
    if [ -n "$ZSH_VERSION" ]; then
        SHELL_FILE="$HOME/.zshrc"
        echo "📌 使用Shell: Zsh"
    else
        SHELL_FILE="$HOME/.bashrc"
        echo "📌 使用Shell: Bash"
    fi
fi

echo ""
echo "请选择配置方式："
echo "1) 自动配置环境变量（推荐）"
echo "2) 生成Python测试脚本"
echo "3) 显示详细配置说明"
echo "4) 测试连接"
echo "5) 全部执行"
echo ""
read -p "请选择 [1-5]: " choice

case $choice in
    1)
        echo ""
        echo "🔧 配置环境变量..."

        if [ "$OS" == "windows" ]; then
            echo "请手动在PowerShell中运行："
            echo ""
            echo "[System.Environment]::SetEnvironmentVariable('ANTHROPIC_API_KEY', '$API_KEY', 'User')"
            echo "[System.Environment]::SetEnvironmentVariable('ANTHROPIC_BASE_URL', '$BASE_URL', 'User')"
            echo ""
            echo "然后重启终端"
        else
            # 检查是否已配置
            if grep -q "ANTHROPIC_API_KEY" "$SHELL_FILE" 2>/dev/null; then
                echo "⚠️  检测到已存在的配置，将备份旧配置"
                cp "$SHELL_FILE" "${SHELL_FILE}.backup.$(date +%Y%m%d_%H%M%S)"
            fi

            # 添加配置
            echo "" >> "$SHELL_FILE"
            echo "# Claude Code API Configuration (Added $(date))" >> "$SHELL_FILE"
            echo "export ANTHROPIC_API_KEY=\"$API_KEY\"" >> "$SHELL_FILE"
            echo "export ANTHROPIC_BASE_URL=\"$BASE_URL\"" >> "$SHELL_FILE"

            echo "✅ 配置已添加到 $SHELL_FILE"
            echo ""
            echo "请执行以下命令使配置生效："
            echo "   source $SHELL_FILE"
            echo ""
            echo "或重启终端"
        fi
        ;;

    2)
        echo ""
        echo "📝 生成Python测试脚本..."

        cat > "$HOME/test_claude_api.py" << 'EOF'
#!/usr/bin/env python3
"""
Claude API 测试脚本
测试与代理服务器的连接
"""
import os
import sys

# API配置
API_KEY = "sk-jU4cYDIDYoUgL0_2tQqdL1dCxrN2q-T9Qd3sGk4dMVw"
BASE_URL = "http://59.110.40.73"

# 从环境变量读取（如果设置）
api_key = os.getenv("ANTHROPIC_API_KEY", API_KEY)
base_url = os.getenv("ANTHROPIC_BASE_URL", BASE_URL)

print("=" * 60)
print("  Claude API 连接测试")
print("=" * 60)
print()
print(f"API Key: {api_key[:20]}...")
print(f"Base URL: {base_url}")
print()

try:
    from anthropic import Anthropic

    client = Anthropic(
        api_key=api_key,
        base_url=base_url
    )

    print("✅ Anthropic库已安装")
    print()
    print("🧪 测试API调用...")
    print("-" * 60)

    response = client.messages.create(
        model="claude-sonnet-4.5",
        max_tokens=200,
        messages=[
            {"role": "user", "content": "你好！请用中文简短地介绍一下你自己"}
        ]
    )

    print()
    print("✅ API调用成功！")
    print()
    print("📝 AI回复:")
    print("-" * 60)
    print(response.content[0].text)
    print("-" * 60)
    print()
    print("📊 Token使用:")
    print(f"   输入: {response.usage.input_tokens}")
    print(f"   输出: {response.usage.output_tokens}")
    print(f"   总计: {response.usage.input_tokens + response.usage.output_tokens}")
    print()
    print("=" * 60)
    print("  ✅ 测试完成！配置正确。")
    print("=" * 60)

except ImportError:
    print("❌ Anthropic库未安装")
    print()
    print("请运行: pip install anthropic")
    sys.exit(1)
except Exception as e:
    print(f"❌ 错误: {e}")
    sys.exit(1)
EOF

        chmod +x "$HOME/test_claude_api.py"
        echo "✅ 测试脚本已创建: $HOME/test_claude_api.py"
        echo ""
        echo "运行测试："
        echo "   python3 ~/test_claude_api.py"
        ;;

    3)
        echo ""
        echo "📖 详细配置说明"
        echo "=" * 60)
        echo ""
        echo "1. 环境变量配置："
        echo "   export ANTHROPIC_API_KEY=\"$API_KEY\""
        echo "   export ANTHROPIC_BASE_URL=\"$BASE_URL\""
        echo ""
        echo "2. Python代码："
        echo "   from anthropic import Anthropic"
        echo "   client = Anthropic("
        echo "       api_key=\"$API_KEY\","
        echo "       base_url=\"$BASE_URL\""
        echo "   )"
        echo ""
        echo "3. cURL测试："
        echo "   curl -X POST $BASE_URL/v1/messages \\"
        echo "     -H \"x-api-key: $API_KEY\" \\"
        echo "     -H \"content-type: application/json\" \\"
        echo "     -d '{\"model\":\"claude-sonnet-4.5\",\"max_tokens\":100,\"messages\":[{\"role\":\"user\",\"content\":\"Hi\"}]}'"
        echo ""
        echo "4. 健康检查："
        echo "   curl $BASE_URL/health"
        echo ""
        ;;

    4)
        echo ""
        echo "🧪 测试连接..."
        echo ""

        # 检查curl
        if command -v curl &> /dev/null; then
            echo "1️⃣  健康检查..."
            if curl -s -m 10 "$BASE_URL/health" > /dev/null 2>&1; then
                echo "   ✅ 服务器在线"
            else
                echo "   ❌ 无法连接到服务器"
            fi

            echo ""
            echo "2️⃣  API调用测试..."
            response=$(curl -s -X POST "$BASE_URL/v1/messages" \
                -H "x-api-key: $API_KEY" \
                -H "content-type: application/json" \
                -d '{"model":"claude-sonnet-4.5","max_tokens":50,"messages":[{"role":"user","content":"Hi"}]}' \
                -m 30 2>&1)

            if echo "$response" | grep -q "content"; then
                echo "   ✅ API调用成功"
                echo ""
                echo "   📝 回复:"
                echo "$response" | python3 -c "import sys,json; d=json.load(sys.stdin); print('   ' + d['content'][0]['text'][:80] + '...')" 2>/dev/null || echo "   (无法解析)"
            else
                echo "   ❌ API调用失败"
                echo "   响应: $response" | head -3
            fi
        else
            echo "❌ curl未安装，无法测试"
            echo "请安装curl: apt-get install curl 或 yum install curl"
        fi
        ;;

    5)
        # 执行所有步骤
        echo ""
        echo "🚀 执行全部配置..."
        echo ""

        # 配置环境变量
        if [ "$OS" != "windows" ]; then
            if [ -n "$SHELL_FILE" ]; then
                echo "1️⃣  配置环境变量..."
                if ! grep -q "ANTHROPIC_API_KEY=$API_KEY" "$SHELL_FILE" 2>/dev/null; then
                    echo "" >> "$SHELL_FILE"
                    echo "# Claude Code API Configuration" >> "$SHELL_FILE"
                    echo "export ANTHROPIC_API_KEY=\"$API_KEY\"" >> "$SHELL_FILE"
                    echo "export ANTHROPIC_BASE_URL=\"$BASE_URL\"" >> "$SHELL_FILE"
                    echo "   ✅ 已配置 $SHELL_FILE"
                else
                    echo "   ⏭️  已配置，跳过"
                fi
            fi
        fi

        echo ""
        echo "2️⃣  测试连接..."
        if curl -s -m 10 "$BASE_URL/health" > /dev/null 2>&1; then
            echo "   ✅ 服务器连接正常"
        else
            echo "   ⚠️  无法连接到服务器"
        fi

        echo ""
        echo "✅ 配置完成！"
        echo ""
        echo "请执行: source $SHELL_FILE"
        echo "或重启终端使配置生效"
        ;;

    *)
        echo "无效选择"
        exit 1
        ;;
esac

echo ""
echo "=========================================="
echo "  配置完成！"
echo "=========================================="
echo ""
echo "📋 快速参考："
echo "   API Key: $API_KEY"
echo "   Base URL: $BASE_URL"
echo "   健康检查: curl $BASE_URL/health"
echo "   模型列表: curl $BASE_URL/v1/models"
echo ""
