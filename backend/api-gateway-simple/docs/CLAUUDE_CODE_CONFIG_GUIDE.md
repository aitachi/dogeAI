# Claude Code 调用配置方案

## 服务器信息

| 项目 | 值 |
|------|-----|
| 服务器 IP | `192.168.48.173` |
| 服务端口 | `8080` |
| 基础 URL | `http://192.168.48.173:8080` |
| 测试 API Key | `sk-ant-api03-CG0BG1GN0D4Q3CRWCREL0DIT2BHZCWSMQUF9SYOCU8V6K6A6Q5GA` |
| 防火墙状态 | `inactive` (未启用) |

---

## 一、Linux 配置方案

### 方式 1: 环境变量配置（推荐）

#### 1.1 临时配置（当前会话有效）

```bash
# 设置环境变量
export ANTHROPIC_API_KEY="sk-ant-api03-CG0BG1GN0D4Q3CRWCREL0DIT2BHZCWSMQUF9SYOCU8V6K6A6Q5GA"
export ANTHROPIC_BASE_URL="http://192.168.48.173:8080"
export ANTHROPIC_AUTH_TOKEN="sk-ant-api03-CG0BG1GN0D4Q3CRWCREL0DIT2BHZCWSMQUF9SYOCU8V6K6A6Q5GA"

# 验证配置
echo $ANTHROPIC_BASE_URL

# 启动 Claude Code
claude
```

#### 1.2 永久配置（写入 Shell 配置文件）

**Bash 用户：**
```bash
# 编辑 ~/.bashrc
nano ~/.bashrc

# 在文件末尾添加以下内容：
# >>> Claude Code 代理配置 <<<
export ANTHROPIC_API_KEY="sk-ant-api03-CG0BG1GN0D4Q3CRWCREL0DIT2BHZCWSMQUF9SYOCU8V6K6A6Q5GA"
export ANTHROPIC_BASE_URL="http://192.168.48.173:8080"
export ANTHROPIC_AUTH_TOKEN="sk-ant-api03-CG0BG1GN0D4Q3CRWCREL0DIT2BHZCWSMQUF9SYOCU8V6K6A6Q5GA"
# <<< Claude Code 代理配置 <<<

# 保存后执行
source ~/.bashrc
```

**Zsh 用户：**
```bash
# 编辑 ~/.zshrc
nano ~/.zshrc

# 在文件末尾添加相同内容
# ...

# 保存后执行
source ~/.zshrc
```

### 方式 2: settings.json 配置

```bash
# 创建配置目录
mkdir -p ~/.config/claude-code

# 创建 settings.json
cat > ~/.config/claude-code/settings.json << 'EOF'
{
  "env": {
    "ANTHROPIC_API_KEY": "sk-ant-api03-CG0BG1GN0D4Q3CRWCREL0DIT2BHZCWSMQUF9SYOCU8V6K6A6Q5GA",
    "ANTHROPIC_BASE_URL": "http://192.168.48.173:8080",
    "ANTHROPIC_AUTH_TOKEN": "sk-ant-api03-CG0BG1GN0D4Q3CRWCREL0DIT2BHZCWSMQUF9SYOCU8V6K6A6Q5GA"
  }
}
EOF

# 验证配置
cat ~/.config/claude-code/settings.json
```

### 方式 3: 命令行临时指定

```bash
# 一次性启动，不修改配置
ANTHROPIC_API_KEY="sk-ant-api03-CG0BG1GN0D4Q3CRWCREL0DIT2BHZCWSMQUF9SYOCU8V6K6A6Q5GA" \
ANTHROPIC_BASE_URL="http://192.168.48.173:8080" \
ANTHROPIC_AUTH_TOKEN="sk-ant-api03-CG0BG1GN0D4Q3CRWCREL0DIT2BHZCWSMQUF9SYOCU8V6K6A6Q5GA" \
claude
```

---

## 二、Windows 配置方案

### 方式 1: 系统环境变量（推荐）

#### 1.1 图形界面设置

1. 按 `Win + X`，选择「系统」
2. 点击「高级系统设置」
3. 点击「环境变量」
4. 在「用户变量」区域点击「新建」

添加以下三个变量：

| 变量名 | 变量值 |
|--------|--------|
| `ANTHROPIC_API_KEY` | `sk-ant-api03-CG0BG1GN0D4Q3CRWCREL0DIT2BHZCWSMQUF9SYOCU8V6K6A6Q5GA` |
| `ANTHROPIC_BASE_URL` | `http://192.168.48.173:8080` |
| `ANTHROPIC_AUTH_TOKEN` | `sk-ant-api03-CG0BG1GN0D4Q3CRWCREL0DIT2BHZCWSMQUF9SYOCU8V6K6A6Q5GA` |

5. 点击「确定」保存
6. **重启已打开的终端/PowerShell/CMD**
7. 验证：在 PowerShell 中运行 `echo $env:ANTHROPIC_BASE_URL`

#### 1.2 命令行设置（PowerShell - 当前用户）

```powershell
# 设置用户级环境变量（永久生效）
[System.Environment]::SetEnvironmentVariable('ANTHROPIC_API_KEY', 'sk-ant-api03-CG0BG1GN0D4Q3CRWCREL0DIT2BHZCWSMQUF9SYOCU8V6K6A6Q5GA', 'User')
[System.Environment]::SetEnvironmentVariable('ANTHROPIC_BASE_URL', 'http://192.168.48.173:8080', 'User')
[System.Environment]::SetEnvironmentVariable('ANTHROPIC_AUTH_TOKEN', 'sk-ant-api03-CG0BG1GN0D4Q3CRWCREL0DIT2BHZCWSMQUF9SYOCU8V6K6A6Q5GA', 'User')

# 重新加载环境变量
$env:ANTHROPIC_API_KEY = [System.Environment]::GetEnvironmentVariable('ANTHROPIC_API_KEY', 'User')
$env:ANTHROPIC_BASE_URL = [System.Environment]::GetEnvironmentVariable('ANTHROPIC_BASE_URL', 'User')
$env:ANTHROPIC_AUTH_TOKEN = [System.Environment]::GetEnvironmentVariable('ANTHROPIC_AUTH_TOKEN', 'User')

# 验证
echo $env:ANTHROPIC_BASE_URL
```

#### 1.3 命令行设置（CMD - 当前会话）

```cmd
REM 仅当前会话有效
set ANTHROPIC_API_KEY=sk-ant-api03-CG0BG1GN0D4Q3CRWCREL0DIT2BHZCWSMQUF9SYOCU8V6K6A6Q5GA
set ANTHROPIC_BASE_URL=http://192.168.48.173:8080
set ANTHROPIC_AUTH_TOKEN=sk-ant-api03-CG0BG1GN0D4Q3CRWCREL0DIT2BHZCWSMQUF9SYOCU8V6K6A6Q5GA

REM 验证
echo %ANTHROPIC_BASE_URL%
```

### 方式 2: settings.json 配置

```powershell
# 创建配置目录
mkdir $env:USERPROFILE\.config\claude-code -Force

# 创建 settings.json
@'
{
  "env": {
    "ANTHROPIC_API_KEY": "sk-ant-api03-CG0BG1GN0D4Q3CRWCREL0DIT2BHZCWSMQUF9SYOCU8V6K6A6Q5GA",
    "ANTHROPIC_BASE_URL": "http://192.168.48.173:8080",
    "ANTHROPIC_AUTH_TOKEN": "sk-ant-api03-CG0BG1GN0D4Q3CRWCREL0DIT2BHZCWSMQUF9SYOCU8V6K6A6Q5GA"
  }
}
'@ | Out-File -FilePath "$env:USERPROFILE\.config\claude-code\settings.json" -Encoding utf8

# 验证
cat $env:USERPROFILE\.config\claude-code\settings.json
```

**配置文件路径：** `C:\Users\你的用户名\.config\claude-code\settings.json`

### 方式 3: PowerShell Profile（每次启动自动加载）

```powershell
# 编辑 PowerShell Profile
notepad $PROFILE

# 在文件中添加以下内容：
$env:ANTHROPIC_API_KEY = "sk-ant-api03-CG0BG1GN0D4Q3CRWCREL0DIT2BHZCWSMQUF9SYOCU8V6K6A6Q5GA"
$env:ANTHROPIC_BASE_URL = "http://192.168.48.173:8080"
$env:ANTHROPIC_AUTH_TOKEN = "sk-ant-api03-CG0BG1GN0D4Q3CRWCREL0DIT2BHZCWSMQUF9SYOCU8V6K6A6Q5GA"

# 保存后重新加载
. $PROFILE
```

---

## 三、测试验证

### 3.1 在 Linux 上测试

```bash
# 测试连接
curl -s http://192.168.48.173:8080/health

# 测试模型列表
curl -s http://192.168.48.173:8080/v1/models \
  -H "x-api-key: sk-ant-api03-CG0BG1GN0D4Q3CRWCREL0DIT2BHZCWSMQUF9SYOCU8V6K6A6Q5GA"

# 测试消息 API
curl -s -X POST http://192.168.48.173:8080/v1/messages \
  -H "x-api-key: sk-ant-api03-CG0BG1GN0D4Q3CRWCREL0DIT2BHZCWSMQUF9SYOCU8V6K6A6Q5GA" \
  -H "anthropic-version: 2023-06-01" \
  -H "content-type: application/json" \
  -d '{
    "model": "claude-sonnet-4-20250514",
    "max_tokens": 50,
    "messages": [{"role": "user", "content": "Say: 测试成功"}]
  }'
```

### 3.2 在 Windows 上测试（PowerShell）

```powershell
# 测试连接
Invoke-RestMethod -Uri "http://192.168.48.173:8080/health"

# 测试消息 API
$body = @{
    model = "claude-sonnet-4-20250514"
    max_tokens = 50
    messages = @(
        @{ role = "user"; content = "Say: 测试成功" }
    )
} | ConvertTo-Json

$headers = @{
    "x-api-key" = "sk-ant-api03-CG0BG1GN0D4Q3CRWCREL0DIT2BHZCWSMQUF9SYOCU8V6K6A6Q5GA"
    "anthropic-version" = "2023-06-01"
}

Invoke-RestMethod -Uri "http://192.168.48.173:8080/v1/messages" `
    -Method POST -Body $body -Headers $headers -ContentType "application/json"
```

---

## 四、故障排查

### 4.1 连接超时

**问题：** 无法连接到 `192.168.48.173:8080`

**检查项：**

1. **服务器服务状态**
```bash
# 在服务器上检查
ps aux | grep api-gateway-simple
netstat -tlnp | grep 8080
```

2. **防火墙设置**
```bash
# 在服务器上开放端口
sudo ufw allow 8080
# 或
sudo iptables -I INPUT -p tcp --dport 8080 -j ACCEPT
```

3. **网络连通性**
```bash
# 从客户端测试 ping
ping 192.168.48.173

# 测试端口
telnet 192.168.48.173 8080
# 或
nc -zv 192.168.48.173 8080
```

### 4.2 认证失败

**问题：** 返回 `authentication_error`

**解决：**
- 确认 API Key 格式正确（`sk-ant-api03-` 开头）
- 确认用户余额充足
- 检查 API Key 是否在数据库中存在

```bash
# 在服务器上检查
psql -h localhost -U postgres -d api_gateway -c "SELECT api_key, user_id, is_active FROM user_api_keys WHERE api_key LIKE 'sk-ant-api03-%';"
```

### 4.3 配置未生效

**Windows：** 修改环境变量后需要重启终端

**Linux：** 执行 `source ~/.bashrc` 或 `source ~/.zshrc`

---

## 五、快速配置脚本

### Linux 快速配置

```bash
#!/bin/bash
# 保存为 setup-claude-proxy.sh 并运行

SERVER_IP="192.168.48.173"
SERVER_PORT="8080"
API_KEY="sk-ant-api03-CG0BG1GN0D4Q3CRWCREL0DIT2BHZCWSMQUF9SYOCU8V6K6A6Q5GA"

# 检测 Shell 类型
if [ -n "$ZSH_VERSION" ]; then
    RC_FILE="$HOME/.zshrc"
elif [[ "$OSTYPE" == "darwin"* ]]; then
    RC_FILE="$HOME/.bash_profile"
else
    RC_FILE="$HOME/.bashrc"
fi

# 添加配置
BLOCK_START="# >>> CLAUDE CODE PROXY BEGIN >>>"
BLOCK_END="# <<< CLAUDE CODE PROXY END <<<"

BLOCK_CONTENT=$(cat <<EOF
$BLOCK_START
export ANTHROPIC_API_KEY="$API_KEY"
export ANTHROPIC_BASE_URL="http://$SERVER_IP:$SERVER_PORT"
export ANTHROPIC_AUTH_TOKEN="$API_KEY"
$BLOCK_END
EOF
)

# 移除旧配置
sed -i "/$BLOCK_START/,/$BLOCK_END/d" "$RC_FILE"

# 添加新配置
echo "" >> "$RC_FILE"
echo "$BLOCK_CONTENT" >> "$RC_FILE"

# 创建 settings.json
mkdir -p ~/.config/claude-code
cat > ~/.config/claude-code/settings.json <<EOF
{
  "env": {
    "ANTHROPIC_API_KEY": "$API_KEY",
    "ANTHROPIC_BASE_URL": "http://$SERVER_IP:$SERVER_PORT",
    "ANTHROPIC_AUTH_TOKEN": "$API_KEY"
  }
}
EOF

echo "配置完成！"
echo "请执行: source $RC_FILE"
echo "然后启动: claude"
```

### Windows PowerShell 快速配置

```powershell
# 保存为 setup-claude-proxy.ps1 并运行

$ServerIP = "192.168.48.173"
$ServerPort = "8080"
$ApiKey = "sk-ant-api03-CG0BG1GN0D4Q3CRWCREL0DIT2BHZCWSMQUF9SYOCU8V6K6A6Q5GA"

# 设置环境变量（用户级）
[System.Environment]::SetEnvironmentVariable('ANTHROPIC_API_KEY', $ApiKey, 'User')
[System.Environment]::SetEnvironmentVariable('ANTHROPIC_BASE_URL', "http://${ServerIP}:${ServerPort}", 'User')
[System.Environment]::SetEnvironmentVariable('ANTHROPIC_AUTH_TOKEN', $ApiKey, 'User')

# 创建配置目录
$configDir = "$env:USERPROFILE\.config\claude-code"
New-Item -ItemType Directory -Path $configDir -Force | Out-Null

# 创建 settings.json
$jsonContent = @"
{
  "env": {
    "ANTHROPIC_API_KEY": "$ApiKey",
    "ANTHROPIC_BASE_URL": "http://${ServerIP}:${ServerPort}",
    "ANTHROPIC_AUTH_TOKEN": "$ApiKey"
  }
}
"@
Set-Content -Path "$configDir\settings.json" -Value $jsonContent -Encoding UTF8

Write-Host "配置完成！" -ForegroundColor Green
Write-Host "请重启 PowerShell，然后运行: claude" -ForegroundColor Yellow
```

---

## 七、配置文件位置总结

| 平台 | 环境变量配置位置 | settings.json 位置 |
|------|------------------|-------------------|
| Linux (Bash) | `~/.bashrc` | `~/.config/claude-code/settings.json` |
| Linux (Zsh) | `~/.zshrc` | `~/.config/claude-code/settings.json` |
| Windows | 系统环境变量 / `$PROFILE` | `C:\Users\用户名\.config\claude-code\settings.json` |
