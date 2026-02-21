"""
监控API端点 - 提供模型调用统计和实时数据
"""

import time
import uuid
from fastapi import Request
from fastapi.responses import JSONResponse

from config import logger
from database import (
    record_call_start,
    record_call_end,
    get_metrics,
    get_recent_calls
)


# 存储当前活跃的调用
_active_calls: dict = {}


def register_monitor_routes(app):
    """注册监控路由"""

    @app.get("/api/metrics")
    async def api_metrics(request: Request):
        """获取监控指标"""
        logger.debug("[Monitor] GET /api/metrics")
        data = get_metrics()
        return JSONResponse(content=data)

    @app.get("/api/calls/recent")
    async def api_recent_calls(request: Request):
        """获取最近的调用记录"""
        limit = request.query_params.get("limit", 100)
        try:
            limit = int(limit)
            limit = min(max(1, limit), 1000)  # 限制在1-1000之间
        except ValueError:
            limit = 100

        logger.debug(f"[Monitor] GET /api/calls/recent?limit={limit}")
        calls = get_recent_calls(limit)
        return JSONResponse(content={"calls": calls})

    @app.get("/api/stats")
    async def api_stats(request: Request):
        """获取统计信息"""
        logger.debug("[Monitor] GET /api/stats")

        metrics = get_metrics()
        stats = {
            "totalCalls": metrics["totalCalls"],
            "onlineModels": metrics["onlineModels"],
            "tokensInRate": metrics["tokensInRate"],
            "tokensOutRate": metrics["tokensOutRate"],
            "models": {m["name"]: m for m in metrics["models"]},
            "lastUpdated": metrics["lastUpdated"]
        }
        return JSONResponse(content=stats)

    @app.get("/api/health")
    async def api_health(request: Request):
        """健康检查端点"""
        from database import get_db

        try:
            with get_db() as conn:
                conn.cursor().execute("SELECT 1")
            return JSONResponse(content={
                "status": "healthy",
                "database": "connected",
                "timestamp": time.time()
            })
        except Exception as e:
            return JSONResponse(
                status_code=503,
                content={
                    "status": "unhealthy",
                    "database": "disconnected",
                    "error": str(e),
                    "timestamp": time.time()
                }
            )

    @app.get("/monitor.html")
    async def monitor_page(request: Request):
        """监控页面"""
        from fastapi.responses import HTMLResponse

        html_content = """<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>AITACHI Cloud - Claude Proxy 实时监控</title>
    <style>
        :root {
            --primary: #2563eb;
            --success: #10b981;
            --warning: #f59e0b;
            --danger: #ef4444;
            --dark: #1e293b;
            --light: #f1f5f9;
            --border: #e2e8f0;
        }
        * { margin: 0; padding: 0; box-sizing: border-box; }
        body {
            font-family: 'Segoe UI', system-ui, -apple-system, sans-serif;
            background: linear-gradient(135deg, #0f172a 0%, #1e293b 100%);
            color: #e2e8f0;
            line-height: 1.6;
            padding: 20px;
            min-height: 100vh;
        }
        .container {
            max-width: 1600px;
            margin: 0 auto;
        }
        header {
            text-align: center;
            margin-bottom: 30px;
            padding: 20px;
            background: rgba(30, 41, 59, 0.7);
            border-radius: 12px;
            backdrop-filter: blur(10px);
            border: 1px solid var(--border);
        }
        h1 {
            font-size: 2.5rem;
            margin-bottom: 10px;
            background: linear-gradient(90deg, #3b82f6, #8b5cf6);
            -webkit-background-clip: text;
            background-clip: text;
            color: transparent;
        }
        .subtitle {
            color: #94a3b8;
            font-size: 1.1rem;
        }
        .stats-grid {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
            gap: 15px;
            margin-bottom: 30px;
        }
        .stat-card {
            background: rgba(30, 41, 59, 0.7);
            border-radius: 12px;
            padding: 15px;
            border: 1px solid var(--border);
            transition: transform 0.2s, box-shadow 0.2s;
        }
        .stat-card:hover {
            transform: translateY(-2px);
            box-shadow: 0 10px 25px rgba(0, 0, 0, 0.3);
        }
        .stat-title {
            font-size: 0.8rem;
            color: #94a3b8;
            margin-bottom: 8px;
            text-transform: uppercase;
            letter-spacing: 1px;
        }
        .stat-value {
            font-size: 1.8rem;
            font-weight: 700;
            margin-bottom: 5px;
        }
        .stat-change {
            font-size: 0.8rem;
            color: #4ade80;
        }
        .models-table {
            background: rgba(30, 41, 59, 0.7);
            border-radius: 12px;
            overflow-x: auto;
            border: 1px solid var(--border);
            margin-bottom: 30px;
        }
        table {
            width: 100%;
            border-collapse: collapse;
            white-space: nowrap;
        }
        th {
            background: rgba(30, 41, 59, 0.9);
            padding: 12px 16px;
            text-align: left;
            font-weight: 600;
            color: #94a3b8;
            border-bottom: 2px solid var(--border);
            font-size: 0.85rem;
        }
        td {
            padding: 12px 16px;
            border-bottom: 1px solid rgba(226, 232, 240, 0.1);
            font-size: 0.9rem;
        }
        tr:last-child td {
            border-bottom: none;
        }
        tr:hover {
            background: rgba(59, 130, 246, 0.1);
        }
        .model-name {
            font-weight: 600;
            color: #60a5fa;
        }
        .response-time {
            font-family: 'SF Mono', Monaco, monospace;
        }
        .response-time .median {
            color: #4ade80;
            font-weight: 600;
        }
        .response-time .range {
            color: #94a3b8;
            font-size: 0.8rem;
        }
        .status-badge {
            padding: 4px 10px;
            border-radius: 20px;
            font-size: 0.75rem;
            font-weight: 600;
        }
        .status-online {
            background: rgba(16, 185, 129, 0.2);
            color: #4ade80;
        }
        .status-idle {
            background: rgba(147, 197, 253, 0.2);
            color: #3b82f6;
        }
        .no-data {
            text-align: center;
            padding: 40px;
            color: #94a3b8;
        }
        .footer {
            text-align: center;
            margin-top: 30px;
            color: #94a3b8;
            font-size: 0.9rem;
        }
        .last-updated {
            text-align: right;
            margin-bottom: 15px;
            color: #94a3b8;
            font-size: 0.9rem;
        }
        .ttft-badge {
            display: inline-block;
            padding: 2px 6px;
            background: rgba(245, 158, 11, 0.2);
            color: #fbbf24;
            border-radius: 4px;
            font-size: 0.7rem;
            margin-left: 4px;
        }
        @media (max-width: 768px) {
            .stats-grid {
                grid-template-columns: 1fr;
            }
            h1 {
                font-size: 2rem;
            }
        }
    </style>
</head>
<body>
    <div class="container">
        <header>
            <h1>AITACHI Cloud · Claude Proxy 监控中心</h1>
            <p class="subtitle">实时同步 · 毫秒级更新 · 全链路可观测</p>
        </header>

        <div class="last-updated">最后更新: <span id="last-update">--:--:--</span></div>

        <div class="stats-grid">
            <div class="stat-card">
                <div class="stat-title">总调用数</div>
                <div class="stat-value" id="total-calls">0</div>
                <div class="stat-change">最近1小时</div>
            </div>
            <div class="stat-card">
                <div class="stat-title">在线模型</div>
                <div class="stat-value" id="online-models">0</div>
                <div class="stat-change">活跃中</div>
            </div>
            <div class="stat-card">
                <div class="stat-title">Token 输入速率</div>
                <div class="stat-value" id="tokens-in-rate">0</div>
                <div class="stat-change">tokens/sec</div>
            </div>
            <div class="stat-card">
                <div class="stat-title">Token 输出速率</div>
                <div class="stat-value" id="tokens-out-rate">0</div>
                <div class="stat-change">tokens/sec</div>
            </div>
        </div>

        <div class="models-table">
            <table>
                <thead>
                    <tr>
                        <th>模型名称</th>
                        <th>调用</th>
                        <th>并发</th>
                        <th>输入 Tokens</th>
                        <th>输出 Tokens</th>
                        <th>中位响应</th>
                        <th>响应范围</th>
                        <th>首Token(TTFT)</th>
                        <th>状态</th>
                    </tr>
                </thead>
                <tbody id="models-tbody">
                    <tr><td colspan="9" class="no-data">加载中...</td></tr>
                </tbody>
            </table>
        </div>

        <div class="footer">
            <p>AITACHI Cloud · Claude Code Proxy Monitoring v2.3 | 数据每 5 秒自动同步</p>
        </div>
    </div>

    <script>
        async function fetchMetrics() {
            try {
                const response = await fetch('/api/metrics');
                if (!response.ok) {
                    throw new Error(`HTTP error! status: ${response.status}`);
                }
                const data = await response.json();

                if (data.error) {
                    console.error('API Error:', data.error);
                    return {
                        totalCalls: 0,
                        onlineModels: 0,
                        tokensInRate: 0,
                        tokensOutRate: 0,
                        models: []
                    };
                }

                return data;
            } catch (error) {
                console.error('Fetch error:', error);
                return {
                    totalCalls: 0,
                    onlineModels: 0,
                    tokensInRate: 0,
                    tokensOutRate: 0,
                    models: []
                };
            }
        }

        function formatResponseTime(model) {
            const median = model.median_response || 0;
            const min = model.min_response || 0;
            const max = model.max_response || 0;

            if (median === 0) {
                return '<span class="response-time">-</span>';
            }

            let html = '<span class="response-time">';
            html += `<span class="median">${median}ms</span>`;
            if (min > 0 || max > 0) {
                html += `<br><span class="range">min:${min}ms max:${max}ms</span>`;
            }
            html += '</span>';
            return html;
        }

        function formatTTFT(model) {
            const avg = model.avg_ttft || 0;
            const min = model.min_ttft || 0;
            const max = model.max_ttft || 0;

            if (avg === 0) {
                return '<span style="color: #64748b;">-</span>';
            }

            let html = `<span class="response-time">`;
            html += `<span class="median">${avg}ms</span>`;
            if (min > 0 || max > 0) {
                html += `<br><span class="range">${min}-${max}ms</span>`;
            }
            html += `</span>`;
            return html;
        }

        async function updateDashboard() {
            const now = new Date();
            document.getElementById('last-update').textContent = now.toLocaleTimeString('zh-CN');

            const data = await fetchMetrics();

            // Update stats cards
            document.getElementById('total-calls').textContent = data.totalCalls.toLocaleString();
            document.getElementById('online-models').textContent = data.onlineModels;
            document.getElementById('tokens-in-rate').textContent = data.tokensInRate;
            document.getElementById('tokens-out-rate').textContent = data.tokensOutRate;

            // Update models table
            const tbody = document.getElementById('models-tbody');
            tbody.innerHTML = '';

            if (!data.models || data.models.length === 0) {
                tbody.innerHTML = '<tr><td colspan="9" class="no-data">暂无数据，等待模型调用...</td></tr>';
                return;
            }

            data.models.forEach(model => {
                const row = document.createElement('tr');
                row.innerHTML = `
                    <td><span class="model-name">${model.name}</span></td>
                    <td>${model.calls.toLocaleString()}</td>
                    <td><span class="status-badge status-${model.status}">${model.concurrent}</span></td>
                    <td>${model.tokens_in.toLocaleString()}</td>
                    <td>${model.tokens_out.toLocaleString()}</td>
                    <td>${formatResponseTime(model)}</td>
                    <td>${formatResponseTime(model)}</td>
                    <td>${formatTTFT(model)}</td>
                    <td><span class="status-badge status-${model.status}">${model.status === 'online' ? '在线' : '空闲'}</span></td>
                `;
                tbody.appendChild(row);
            });
        }

        // Initial load + auto-refresh every 5 seconds
        updateDashboard();
        setInterval(updateDashboard, 5000);
    </script>
</body>
</html>"""
        return HTMLResponse(content=html_content)


# 工具函数：开始记录调用
def start_call_recording(model_name: str, original_model: str, stream: bool) -> tuple:
    """开始记录调用，返回 (request_id, call_id)"""
    request_id = f"req_{uuid.uuid4().hex[:24]}"
    call_id = record_call_start(request_id, model_name, original_model, stream)
    return request_id, call_id


# 工具函数：结束记录调用
def end_call_recording(
    call_id: int,
    status: str,
    input_tokens: int = 0,
    output_tokens: int = 0,
    error_message: str = None,
    first_token_time: float = None
):
    """结束记录调用"""
    record_call_end(call_id, status, input_tokens, output_tokens, error_message, first_token_time)
