# Claude Code 代理客户端快速配置脚本 (Windows PowerShell)
# 使用方法: .\setup-client-windows.ps1

# ========================================
# 配置参数（公网访问配置）
# ========================================
$ServerIP = "115.190.62.87"
$ServerPort = "80"  # Nginx 对外端口
$ApiKey = "sk-ant-api03-CG0BG1GN0D4Q3CRWCREL0DIT2BHZCWSMQUF9SYOCU8V6K6A6Q5GA"
$Domain = "aitachi.top"  # 可选域名

# ========================================
# 颜色输出函数
# ========================================
function Write-ColorOutput {
    param([string]$Message, [string]$Color = "White")
    Write-Host $Message -ForegroundColor $Color
}

Write-ColorOutput "========================================" "Cyan"
Write-ColorOutput "   Claude Code 代理配置 (Windows)" "Cyan"
Write-ColorOutput "========================================" "Cyan"
Write-Host ""
Write-ColorOutput "公网IP: " "White" -NoNewline
Write-ColorOutput "${ServerIP}:${ServerPort}" "Yellow"
Write-ColorOutput "域名: " "White" -NoNewline
Write-ColorOutput "${Domain}" "Yellow" -NoNewline
Write-ColorOutput " (如果DNS解析正常)" "Gray"
Write-ColorOutput "API Key: " "White" -NoNewline
Write-ColorOutput "$($ApiKey.Substring(0, 25))..." "Yellow"
Write-Host ""

# ========================================
# 1. 设置系统环境变量
# ========================================
Write-ColorOutput "[1/4] 设置系统环境变量..." "Cyan"

try {
    $baseUrl = "http://${ServerIP}"
    [System.Environment]::SetEnvironmentVariable('ANTHROPIC_API_KEY', $ApiKey, 'User')
    [System.Environment]::SetEnvironmentVariable('ANTHROPIC_BASE_URL', $baseUrl, 'User')
    [System.Environment]::SetEnvironmentVariable('ANTHROPIC_AUTH_TOKEN', $ApiKey, 'User')
    Write-ColorOutput "   ✓ 环境变量已设置" "Green"
    Write-ColorOutput "     ANTHROPIC_BASE_URL = $baseUrl" "Gray"
} catch {
    Write-ColorOutput "   ✗ 设置环境变量失败: $_" "Red"
    exit 1
}
Write-Host ""

# ========================================
# 2. 创建 settings.json
# ========================================
Write-ColorOutput "[2/4] 创建 settings.json..." "Cyan"

try {
    $configDir = "$env:USERPROFILE\.config\claude-code"
    New-Item -ItemType Directory -Path $configDir -Force | Out-Null

    $baseUrl = "http://${ServerIP}"
    $jsonContent = @"
{
  "env": {
    "ANTHROPIC_API_KEY": "$ApiKey",
    "ANTHROPIC_BASE_URL": "$baseUrl",
    "ANTHROPIC_AUTH_TOKEN": "$ApiKey"
  }
}
"@
    Set-Content -Path "$configDir\settings.json" -Value $jsonContent -Encoding UTF8
    Write-ColorOutput "   ✓ settings.json 已创建" "Green"
    Write-ColorOutput "     位置: $configDir\settings.json" "Gray"
} catch {
    Write-ColorOutput "   ✗ 创建 settings.json 失败: $_" "Red"
}
Write-Host ""

# ========================================
# 3. 测试连接
# ========================================
Write-ColorOutput "[3/4] 测试服务器连接..." "Cyan"

try {
    $response = Invoke-WebRequest -Uri "http://${ServerIP}/v1/models" -UseBasicParsing -TimeoutSec 10 -ErrorAction Stop
    Write-ColorOutput "   ✓ 公网IP连接正常 (${ServerIP})" "Green"
} catch {
    Write-ColorOutput "   ✗ 无法连接到公网IP" "Red"
    Write-ColorOutput "     请检查:" "Yellow"
    Write-ColorOutput "       - 服务器是否运行" "White"
    Write-ColorOutput "       - 网络是否连通" "White"
    Write-ColorOutput "       - 防火墙设置" "White"
}

# 测试域名解析
try {
    $dns = Resolve-DnsName -Name $Domain -ErrorAction Stop
    Write-ColorOutput "   ✓ 域名 ${Domain} 可以解析" "Green"
    Write-ColorOutput "     提示: 如果解析到正确IP，可以在配置中使用域名" "Yellow"
} catch {
    Write-ColorOutput "   ! 域名 ${Domain} 无法解析，请使用公网IP" "Yellow"
}
Write-Host ""

# ========================================
# 4. 显示测试命令
# ========================================
Write-ColorOutput "[4/4] 测试命令..." "Cyan"
Write-Host ""
Write-ColorOutput "   # 测试模型列表 (PowerShell)" "Yellow"
Write-Host "   Invoke-RestMethod -Uri `"http://${ServerIP}/v1/models`" `"
Write-Host "     -Headers @{@{`"x-api-key`"=`"${ApiKey}`"}"
Write-Host ""
Write-ColorOutput "   # 测试消息API" "Yellow"
Write-Host "   `$body = @{"
Write-Host "     model = `"claude-sonnet-4-20250514`""
Write-Host "     max_tokens = 50"
Write-Host "     messages = @(@{role = `"user`"; content = `"Hi`"})"
Write-Host "   } | ConvertTo-Json"
Write-Host "   Invoke-RestMethod -Uri `"http://${ServerIP}/v1/messages`" `"
Write-Host "     -Method POST -Body `$body -Headers @{@{`"x-api-key`"=`"${ApiKey}`"}} `"
Write-Host "     -ContentType `"application/json`""
Write-Host ""

# ========================================
# 完成
# ========================================
Write-ColorOutput "========================================" "Cyan"
Write-ColorOutput "         配置完成！" "Cyan"
Write-ColorOutput "========================================" "Cyan"
Write-Host ""
Write-ColorOutput "下一步操作:" "Green"
Write-Host ""
Write-ColorOutput "1. 关闭当前 PowerShell 窗口" "White"
Write-ColorOutput "2. 重新打开 PowerShell" "White"
Write-ColorOutput "3. 验证配置:" "White"
Write-ColorOutput "   echo `$env:ANTHROPIC_BASE_URL" "Yellow"
Write-Host ""
Write-ColorOutput "4. 启动 Claude Code:" "White"
Write-ColorOutput "   claude" "Yellow"
Write-Host ""
Write-ColorOutput "========================================" "Cyan"
Write-Host ""

# 重新加载当前会话的环境变量
$env:ANTHROPIC_API_KEY = [System.Environment]::GetEnvironmentVariable('ANTHROPIC_API_KEY', 'User')
$env:ANTHROPIC_BASE_URL = [System.Environment]::GetEnvironmentVariable('ANTHROPIC_BASE_URL', 'User')
$env:ANTHROPIC_AUTH_TOKEN = [System.Environment]::GetEnvironmentVariable('ANTHROPIC_AUTH_TOKEN', 'User')

Write-ColorOutput "当前会话环境变量已加载！" "Green"
Write-ColorOutput "你可以直接运行: claude" "Yellow"
Write-Host ""
Write-ColorOutput "注意: 如果使用域名访问，请确保DNS解析正确，或使用公网IP" "Gray"
