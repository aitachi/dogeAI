#!/bin/bash
set -e

echo "========================================"
echo "  Aitachi 计费系统 - 快速部署修复脚本"
echo "========================================"
echo ""

# 1. 配置 nginx
echo "[1/5] 配置 nginx..."
echo 'server { listen 80; location / { proxy_pass http://127.0.0.1:3000; } }' | sudo tee /etc/nginx/sites-available/default > /dev/null

# 2. 测试并重启 nginx
echo "[2/5] 重启 nginx..."
sudo nginx -t > /dev/null 2>&1
sudo systemctl reload nginx > /dev/null 2>&1
echo "✓ nginx 已配置并重启"

# 3. 进入项目目录
echo "[3/5] 进入项目目录..."
cd /root/aitachi/billing-system

# 4. 停止旧进程
echo "[4/5] 停止旧进程..."
pkill -f billing-gateway 2>/dev/null || true
sleep 2

# 5. 启动应用
echo "[5/5] 启动应用..."
nohup cargo run --release > app.log 2>&1 &
sleep 3

echo ""
echo "========================================"
echo "  部署完成！"
echo "========================================"
echo ""
echo "检查服务状态:"

# 检查nginx
echo ""
echo "▶ nginx:"
sudo systemctl is-active nginx && echo "  ✓ 运行中" || echo "  ✗ 未运行"

# 检查应用
echo "▶ 应用进程:"
if ps aux | grep -v grep | grep billing-gateway > /dev/null 2>&1; then
    echo "  ✓ 运行中 (PID: $(pgrep -o billing-gateway | head -1))"
else
    echo "  ✗ 未运行，正在启动..."
    nohup cargo run --release > app.log 2>&1 &
    sleep 5
    if ps aux | grep -v grep | grep billing-gateway > /dev/null 2>&1; then
        echo "  ✓ 已启动"
    else
        echo "  ✗ 启动失败，请检查日志:"
        echo "  tail -20 /root/aitachi/billing-system/app.log"
        exit 1
    fi
fi

echo ""
echo "端口监听:"
ss -tuln | grep :3000 && echo "  ✓ 3000端口已监听" || echo "  ✗ 3000端口未监听"

echo ""
echo "服务测试:"
curl -s http://localhost:3000/health | head -5 || echo "  ✗ 本地服务无响应"

echo ""
echo "========================================"
echo "  访问地址"
echo "========================================"
echo "  门户入口: http://115.190.62.87/"
echo "  客户端:   http://115.190.62.87/client.html"
echo "  管理端:   http://115.190.62.87/admin.html"
echo ""
echo "如需查看日志: tail -f /root/aitachi/billing-system/app.log"
echo "如需停止服务: pkill -f billing-gateway"
echo "========================================"
