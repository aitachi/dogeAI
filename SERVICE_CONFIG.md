# Claude Code Proxy - Systemd 服务配置

## 📋 服务配置完成

服务已配置为 **systemd 守护进程**，支持：
- ✅ 开机自动启动
- ✅ 崩溃自动重启
- ✅ 日志自动管理
- ✅ 资源限制保护

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

## 🔧 服务信息

**服务名称**: `claude-proxy.service`
**配置文件**: `/etc/systemd/system/claude-proxy.service`
**工作目录**: `/root/dogeAI`
**执行文件**: `/root/dogeAI/main_http.py`
**监听端口**: HTTP 3001 (HTTPS 443由nginx处理)

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

## 🚀 服务管理

### 方式1: 使用 systemctl (标准)

```bash
# 启动服务
systemctl start claude-proxy

# 停止服务
systemctl stop claude-proxy

# 重启服务
systemctl restart claude-proxy

# 查看状态
systemctl status claude-proxy

# 设置开机自启
systemctl enable claude-proxy

# 取消开机自启
systemctl disable claude-proxy

# 查看日志
journalctl -u claude-proxy -f

# 查看最近日志
journalctl -u claude-proxy -n 50
```

### 方式2: 使用管理脚本 (推荐)

```bash
cd /root/dogeAI

# 启动服务
./service-manage.sh start

# 停止服务
./service-manage.sh stop

# 重启服务
./service-manage.sh restart

# 查看状态
./service-manage.sh status

# 设置开机自启
./service-manage.sh enable

# 取消开机自启
./service-manage.sh disable

# 实时日志
./service-manage.sh logs

# 最近50条日志
./service-manage.sh log

# 测试服务
./service-manage.sh test
```

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

## 📊 服务配置详情

### 自动重启策略

```ini
Restart=always           # 总是自动重启
RestartSec=10           # 重启前等待10秒
StartLimitInterval=60   # 60秒内
StartLimitBurst=3       # 最多重启3次
```

**含义**: 服务崩溃后会自动重启，如果60秒内连续崩溃超过3次，则停止尝试。

### 资源限制

```ini
LimitNOFILE=65536       # 最大文件描述符
LimitNPROC=4096         # 最大进程数
```

### 日志管理

```ini
StandardOutput=append:/root/dogeAI/logs/proxy.log
StandardError=append:/root/dogeAI/logs/error.log
```

日志文件位置:
- 标准输出: `/root/dogeAI/logs/proxy.log`
- 错误输出: `/root/dogeAI/logs/error.log`
- 系统日志: `journalctl -u claude-proxy`

### 安全配置

```ini
PrivateTmp=true         # 使用独立的/tmp目录
ProtectSystem=strict    # 只读系统目录
ProtectHome=false       # 可访问家目录
ReadWritePaths=/root/dogeAI  # 可写目录
```

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

## 🔍 服务状态检查

### 查看详细状态

```bash
systemctl status claude-proxy
```

正常输出示例:
```
● claude-proxy.service - Claude Code Proxy Server
   Loaded: loaded (/etc/systemd/system/claude-proxy.service; enabled)
   Active: active (running) since Tue 2026-02-17 19:14:20 CST
 Main PID: 145425 (python3.11)
    Tasks: 5 (limit: 11716)
  Memory: 34.8M
```

### 检查端口监听

```bash
netstat -tlnp | grep 3001
# 或
ss -tlnp | grep 3001
```

### 检查进程

```bash
ps aux | grep main_http.py
```

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

## 🛠️ 故障排查

### 服务无法启动

1. 查看服务状态
   ```bash
   systemctl status claude-proxy
   ```

2. 查看系统日志
   ```bash
   journalctl -u claude-proxy -n 50
   ```

3. 查看错误日志
   ```bash
   cat /root/dogeAI/logs/error.log
   ```

4. 手动运行测试
   ```bash
   cd /root/dogeAI
   python3.11 main_http.py
   ```

### 端口被占用

```bash
# 查看3001端口占用
netstat -tlnp | grep 3001

# 停止占用进程
systemctl stop claude-proxy
```

### 服务频繁重启

查看重启次数:
```bash
systemctl status claude-proxy | grep restart
```

如果超过限制，服务会停止运行。需要：
1. 修复问题
2. 手动启动: `systemctl start claude-proxy`

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

## 📝 日志查看

### 实时日志

```bash
# systemd日志
journalctl -u claude-proxy -f

# 应用日志
tail -f /root/dogeAI/logs/proxy.log
```

### 历史日志

```bash
# 最近50条
journalctl -u claude-proxy -n 50

# 今天日志
journalctl -u claude-proxy --since today

# 指定时间范围
journalctl -u claude-proxy --since "1 hour ago"
```

### 日志分析

```bash
# 错误日志
journalctl -u claude-proxy | grep ERROR

# 启动日志
journalctl -u claude-proxy | grep Started

# 重启记录
journalctl -u claude-proxy | grep Restarting
```

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

## 🔄 更新和升级

### 修改服务配置

1. 编辑服务文件
   ```bash
   vi /etc/systemd/system/claude-proxy.service
   ```

2. 重新加载配置
   ```bash
   systemctl daemon-reload
   ```

3. 重启服务
   ```bash
   systemctl restart claude-proxy
   ```

### 更新代码

1. 停止服务
   ```bash
   systemctl stop claude-proxy
   ```

2. 更新代码
   ```bash
   cd /root/dogeAI
   # 进行代码修改...
   ```

3. 重启服务
   ```bash
   systemctl start claude-proxy
   ```

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

## 🌐 网络架构

```
外网请求
    ↓
nginx (443端口) - SSL终端
    ↓
反向代理转发
    ↓
claude-proxy (3001端口) - HTTP处理
    ↓
DashScope API
```

**优势**:
- nginx处理SSL/HTTPS，性能更好
- Python服务只处理业务逻辑，更稳定
- 支持nginx级别的负载均衡和缓存

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

## ✅ 验证清单

启动后请验证:

- [ ] 服务状态: `systemctl status claude-proxy`
- [ ] 端口监听: `netstat -tlnp | grep 3001`
- [ ] 健康检查: `curl http://127.0.0.1:3001/api/hello`
- [ ] 模型列表: `curl http://127.0.0.1:3001/v1/models`
- [ ] 外网访问: `curl http://59.110.40.73:3001/api/hello`
- [ ] HTTPS访问: `curl https://59.110.40.73/v1/claudecode/api/hello`
- [ ] 日志正常: `journalctl -u claude-proxy -n 20`

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

## 📞 快速参考

```bash
# 快速命令
cd /root/dogeAI

# 查看状态
./service-manage.sh status

# 重启服务
./service-manage.sh restart

# 查看日志
./service-manage.sh logs

# 测试服务
./service-manage.sh test

# 或使用systemctl
systemctl status claude-proxy
systemctl restart claude-proxy
journalctl -u claude-proxy -f
```

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

**配置时间**: 2026-02-17
**服务状态**: ✅ 运行中
**开机自启**: ✅ 已启用
**自动重启**: ✅ 已配置
