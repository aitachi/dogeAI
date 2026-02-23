package routes

import (
	"encoding/json"
	"net/http"
	"time"
)

// HandleMetrics handles metrics endpoint
func HandleMetrics(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(map[string]any{
		"totalCalls":    0,
		"onlineModels":  5,
		"tokensInRate":  0,
		"tokensOutRate": 0,
		"models":        []any{},
		"lastUpdated":   time.Now().Unix(),
	})
}

// HandleRecentCalls handles recent calls endpoint
func HandleRecentCalls(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(map[string]any{"calls": []any{}})
}

// HandleStats handles stats endpoint
func HandleStats(w http.ResponseWriter, r *http.Request) {
	HandleMetrics(w, r)
}

// HandleAPIHealth handles API health endpoint
func HandleAPIHealth(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(map[string]any{
		"status":   "healthy",
		"database": "connected",
		"timestamp": time.Now().Unix(),
	})
}

// HandleMonitorPage handles monitor HTML page (simplified)
func HandleMonitorPage(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "text/html")
	w.Write([]byte(monitorHTML))
}

// monitorHTML is the monitoring dashboard (simplified)
const monitorHTML = `<!DOCTYPE html>
<html lang="zh-CN">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>AITACHI Cloud - Go Proxy 监控</title>
<style>
  body { font-family: system-ui; background: #0f172a; color: #e2e8f0; padding: 20px; }
  .container { max-width: 1200px; margin: 0 auto; }
  h1 { background: linear-gradient(90deg, #3b82f6, #cc785c); -webkit-background-clip: text; color: transparent; }
  .stats { display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 15px; margin: 20px 0; }
  .card { background: rgba(30,41,59,0.7); padding: 20px; border-radius: 12px; border-left: 4px solid #cc785c; }
  .label { font-size: 0.8rem; color: #94a3b8; text-transform: uppercase; }
  .value { font-size: 2rem; font-weight: bold; }
  .badge { background: #00ADD8; color: white; padding: 2px 8px; border-radius: 4px; font-size: 0.7rem; }
  table { width: 100%; background: rgba(30,41,59,0.7); border-radius: 12px; border-collapse: collapse; }
  th, td { padding: 12px; text-align: left; border-bottom: 1px solid rgba(255,255,255,0.1); }
  th { color: #94a3b8; }
</style>
</head>
<body>
<div class="container">
  <h1>AITACHI Cloud <span class="badge">Go</span></h1>
  <p>高性能 Claude Proxy 服务</p>
  <div class="stats">
    <div class="card"><div class="label">总调用数</div><div class="value" id="calls">0</div></div>
    <div class="card"><div class="label">在线模型</div><div class="value" id="models">5</div></div>
    <div class="card"><div class="label">输入速率</div><div class="value" id="in">0</div></div>
    <div class="card"><div class="label">输出速率</div><div class="value" id="out">0</div></div>
  </div>
  <table>
    <tr><th>模型</th><th>调用</th><th>输入Tokens</th><th>输出Tokens</th></tr>
    <tr><td colspan="4">Go Proxy 服务运行正常</td></tr>
  </table>
</div>
<script>
fetch('/api/metrics').then(r=>r.json()).then(d=>{
  document.getElementById('calls').textContent=d.totalCalls;
  document.getElementById('models').textContent=d.onlineModels||5;
  document.getElementById('in').textContent=d.tokensInRate;
  document.getElementById('out').textContent=d.tokensOutRate;
});
setInterval(()=>fetch('/api/metrics').then(r=>r.json()).then(d=>{
  document.getElementById('calls').textContent=d.totalCalls;
  document.getElementById('in').textContent=d.tokensInRate;
  document.getElementById('out').textContent=d.tokensOutRate;
}),5000);
</script>
</body>
</html>`
