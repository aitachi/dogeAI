#!/bin/bash
#
# Claude Code Proxy 停止脚本
#

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PID_FILE="$SCRIPT_DIR/proxy.pid"

echo "========================================"
echo "  Claude Code Proxy - 停止中"
echo "========================================"

if [ ! -f "$PID_FILE" ]; then
    echo "未找到PID文件，服务可能未运行"
    echo "尝试查找进程..."

    # 查找可能的进程
    PIDS=$(pgrep -f "python.*main.py")
    if [ -n "$PIDS" ]; then
        echo "找到运行中的进程: $PIDS"
        echo "正在停止..."
        kill $PIDS
        sleep 2
        if pgrep -f "python.*main.py" > /dev/null; then
            echo "强制停止..."
            kill -9 $PIDS
        fi
        echo "✅ 服务已停止"
    else
        echo "未找到运行中的服务"
    fi
    exit 0
fi

PID=$(cat "$PID_FILE")

if ps -p "$PID" > /dev/null 2>&1; then
    echo "正在停止服务 (PID: $PID)..."
    kill "$PID"

    # 等待进程结束
    for i in {1..10}; do
        if ! ps -p "$PID" > /dev/null 2>&1; then
            break
        fi
        sleep 1
    done

    # 如果还在运行，强制停止
    if ps -p "$PID" > /dev/null 2>&1; then
        echo "强制停止..."
        kill -9 "$PID"
        sleep 1
    fi

    rm -f "$PID_FILE"
    echo "✅ 服务已停止"
else
    echo "进程 $PID 未运行"
    rm -f "$PID_FILE"
fi

echo "========================================"
