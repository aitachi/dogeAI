#!/bin/bash

# 性能测试脚本
# 对比 Python 版本和 Go 版本的性能

PYTHON_DIR="C:/Users/ASUS/Desktop/dogeai/dogeAI"
GO_DIR="C:/Users/ASUS/Desktop/dogeai/goDogeAI"
RESULTS_DIR="$GO_DIR/benchmark_results"

mkdir -p "$RESULTS_DIR"

echo "========================================"
echo "性能测试: Python vs Go"
echo "========================================"
echo ""

# 测试1: 启动时间
echo "测试1: 启动时间"
echo "--------"

# Python 版本启动时间
cd "$PYTHON_DIR"
start_python=$(date +%s%3N)
python main.py &
PYTHON_PID=$!
sleep 1
end_python=$(date +%s%3N)
python_startup=$((end_python - start_python))
kill $PYTHON_PID 2>/dev/null
sleep 1

# Go 版本启动时间
cd "$GO_DIR"
start_go=$(date +%s%3N)
./dogeai.exe &
GO_PID=$!
sleep 1
end_go=$(date +%s%3N)
go_startup=$((end_go - start_go))
kill $GO_PID 2>/dev/null
sleep 1

echo "Python 启动时间: ${python_startup}ms"
echo "Go 启动时间: ${go_startup}ms"
echo "Go 快了 $((python_startup - go_startup))ms"
echo ""

# 启动 Go 服务器进行测试
cd "$GO_DIR"
./dogeai.exe &
GO_PID=$!
sleep 2

echo "测试2: 内存使用"
echo "--------"
tasklist | grep dogeai.exe | head -1
echo ""

echo "测试3: 简单请求响应时间"
echo "--------"

# 测试100次请求
echo "测试 GET / 100次..."
total_time=0
for i in {1..100}; do
    start=$(date +%s%3N)
    curl -s http://localhost:3001/ > /dev/null
    end=$(date +%s%3N)
    time=$((end - start))
    total_time=$((total_time + time))
done
avg_time=$((total_time / 100))
echo "平均响应时间: ${avg_time}ms"
echo ""

echo "测试4: 消息API性能 (流式)"
echo "--------"

start=$(date +%s%3N)
response=$(curl -s -X POST http://localhost:3001/v1/messages \
    -H "Content-Type: application/json" \
    -H "x-api-key: test" \
    -d '{
        "model": "claude-sonnet-4-5-20250929",
        "max_tokens": 50,
        "messages": [{"role": "user", "content": "Say hello"}]
    }' \
    -w "\n%{http_code}\n%{time_total}秒\n")
end=$(date +%s%3N)

echo "流式消息请求时间: $((end - start))ms"
echo ""

echo "测试5: 并发请求"
echo "--------"

# 使用 ab (Apache Bench) 进行并发测试
if command -v ab &> /dev/null; then
    echo "并发测试: 100个请求, 10并发"
    ab -n 100 -c 10 -q http://localhost:3001/ > "$RESULTS_DIR/ab_results.txt"
    echo "结果已保存到 $RESULTS_DIR/ab_results.txt"
else
    echo "ab 未安装，跳过并发测试"
fi
echo ""

echo "测试6: JSON解析性能"
echo "--------"

# 测试大 JSON 处理
large_json='{"model":"claude-sonnet-4-5-20250929","max_tokens":4096,"stream":false,"messages":['
for i in {1..50}; do
    large_json+='{"role":"user","content":"This is a test message number '$i' with some additional text to make it longer. "},'
done
large_json="${large_json%,}]}"

start=$(date +%s%3N)
curl -s -X POST http://localhost:3001/v1/messages \
    -H "Content-Type: application/json" \
    -H "x-api-key: test" \
    -d "$large_json" > /dev/null
end=$(date +%s%3N)
echo "大JSON请求 (50条消息) 处理时间: $((end - start))ms"
echo ""

# 清理
kill $GO_PID 2>/dev/null

echo "========================================"
echo "测试完成！"
echo "========================================"
