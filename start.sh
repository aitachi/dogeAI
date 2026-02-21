#!/bin/bash
#
# Claude Code Proxy 启动脚本
#

# 设置脚本目录
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# 配置文件路径
CONFIG_FILE="$SCRIPT_DIR/config.json"
LOG_DIR="$SCRIPT_DIR/logs"
PID_FILE="$SCRIPT_DIR/proxy.pid"

# 创建日志目录
mkdir -p "$LOG_DIR"

# 检查配置文件
if [ ! -f "$CONFIG_FILE" ]; then
    echo "错误: 配置文件不存在: $CONFIG_FILE"
    exit 1
fi

# 提取配置
HTTP_PORT=$(grep -o '"http_port":[[:space:]]*[0-9]*' "$CONFIG_FILE" | grep -o '[0-9]*')
HTTPS_PORT=$(grep -o '"https_port":[[:space:]]*[0-9]*' "$CONFIG_FILE" | grep -o '[0-9]*')

echo "========================================"
echo "  Claude Code Proxy - 启动中"
echo "========================================"
echo "配置文件: $CONFIG_FILE"
echo "HTTP端口: $HTTP_PORT"
echo "HTTPS端口: $HTTPS_PORT"
echo "========================================"

# 检查Python版本
if command -v python3.11 &> /dev/null; then
    PYTHON_CMD="python3.11"
    echo "使用Python: $(python3.11 --version)"
elif command -v python3 &> /dev/null; then
    PYTHON_CMD="python3"
    echo "使用Python: $(python3 --version)"
else
    echo "错误: 未找到Python 3"
    exit 1
fi

# 检查是否已在运行
if [ -f "$PID_FILE" ]; then
    OLD_PID=$(cat "$PID_FILE")
    if ps -p "$OLD_PID" > /dev/null 2>&1; then
        echo "服务已在运行 (PID: $OLD_PID)"
        echo "如需重启，请先运行: ./stop.sh"
        exit 1
    else
        echo "清理旧的PID文件"
        rm -f "$PID_FILE"
    fi
fi

# 检查端口占用
if netstat -tln 2>/dev/null | grep -q ":$HTTP_PORT "; then
    echo "警告: HTTP端口 $HTTP_PORT 已被占用"
fi
if netstat -tln 2>/dev/null | grep -q ":$HTTPS_PORT "; then
    echo "警告: HTTPS端口 $HTTPS_PORT 已被占用"
fi

# 启动服务
echo ""
echo "启动服务..."
nohup $PYTHON_CMD main.py > "$LOG_DIR/proxy.log" 2>&1 &
PROXY_PID=$!
echo $PROXY_PID > "$PID_FILE"

# 等待启动
sleep 3

# 检查是否启动成功
if ps -p $PROXY_PID > /dev/null; then
    echo "========================================"
    echo "✅ 服务启动成功!"
    echo "========================================"
    echo "PID: $PROXY_PID"
    echo "HTTP:  http://0.0.0.0:$HTTP_PORT"
    echo "HTTPS: https://0.0.0.0:$HTTPS_PORT"
    echo ""
    echo "查看日志: tail -f $LOG_DIR/proxy.log"
    echo "停止服务: ./stop.sh"
    echo "========================================"
else
    echo "========================================"
    echo "❌ 服务启动失败!"
    echo "========================================"
    echo "请检查日志: $LOG_DIR/proxy.log"
    rm -f "$PID_FILE"
    exit 1
fi
