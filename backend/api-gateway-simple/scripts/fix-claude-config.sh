#!/bin/bash
# Claude Code 客户端配置修复脚本
# 使用方法: bash fix-claude-config.sh

echo "╔══════════════════════════════════════════════════════════════╗"
echo "║     Claude Code 客户端配置修复                                ║"
echo "╚══════════════════════════════════════════════════════════════╝"
echo ""

# 正确的配置
API_KEY="sk-ant-api03-CG0BG1GN0D4Q3CRWCREL0DIT2BHZCWSMQUF9SYOCU8V6K6A6Q5GA"
BASE_URL="http://115.190.62.87"  # 注意：不要包含 /v1

# 检测 Shell 配置文件
if [ -n "${ZSH_VERSION:-}" ]; then
    RC_FILE="$HOME/.zshrc"
else
    RC_FILE="$HOME/.bashrc"
fi

echo "配置文件: $RC_FILE"
echo ""

# 1. 备份原配置
echo "[1/4] 备份原配置..."
cp "$RC_FILE" "${RC_FILE}.backup.$(date +%Y%m%d_%H%M%S)"
echo "   ✓ 已备份到 ${RC_FILE}.backup.$(date +%Y%m%d_%H%M%S)"
echo ""

# 2. 删除旧的 Claude Code 配置
echo "[2/4] 清理旧配置..."
# 删除包含 ANTHROPIC 的行
sed -i '/ANTHROPIC_API_KEY/d' "$RC_FILE"
sed -i '/ANTHROPIC_BASE_URL/d' "$RC_FILE"
sed -i '/ANTHROPIC_AUTH_TOKEN/d' "$RC_FILE"
sed -i '/Claude Code 配置/d' "$RC_FILE"
sed -i '/Claude API 配置/d' "$RC_FILE"
echo "   ✓ 已清理旧配置"
echo ""

# 3. 添加新配置
echo "[3/4] 添加新配置..."
cat >> "$RC_FILE" << 'EOF'

# Claude Code 代理配置 (公网访问)
export ANTHROPIC_API_KEY="sk-ant-api03-CG0BG1GN0D4Q3CRWCREL0DIT2BHZCWSMQUF9SYOCU8V6K6A6Q5GA"
export ANTHROPIC_BASE_URL="http://115.190.62.87"
export ANTHROPIC_AUTH_TOKEN="sk-ant-api03-CG0BG1GN0D4Q3CRWCREL0DIT2BHZCWSMQUF9SYOCU8V6K6A6Q5GA"
EOF

echo "   ✓ 已添加新配置"
echo ""

# 4. 验证配置
echo "[4/4] 验证配置..."
echo ""
echo "测试服务器连接..."
if curl -s --connect-timeout 10 "$BASE_URL/v1/models" > /dev/null 2>&1; then
    echo "   ✓ 服务器连接正常"
else
    echo "   ✗ 服务器连接失败"
fi
echo ""

echo "╔══════════════════════════════════════════════════════════════╗"
echo "║              配置修复完成！                                    ║"
echo "╚══════════════════════════════════════════════════════════════╝"
echo ""
echo "请执行以下命令应用配置："
echo ""
echo "  source $RC_FILE"
echo ""
echo "然后启动 Claude Code："
echo ""
echo "  claude"
echo ""
echo "────────────────────────────────────────────────────────────────"
echo "正确的配置值："
echo "────────────────────────────────────────────────────────────────"
echo "  ANTHROPIC_API_KEY     = $API_KEY"
echo "  ANTHROPIC_BASE_URL   = $BASE_URL"
echo "  ANTHROPIC_AUTH_TOKEN = $API_KEY"
echo ""
echo "⚠️  注意: ANTHROPIC_BASE_URL 不要包含 /v1 后缀！"
echo "╚══════════════════════════════════════════════════════════════╝"
