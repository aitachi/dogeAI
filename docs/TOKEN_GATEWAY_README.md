# Token中转平台 - 完整使用文档

## 📚 目录

1. [快速开始](#快速开始)
2. [核心功能](#核心功能)
3. [API接口](#api接口)
4. [管理工具](#管理工具)
5. [配置说明](#配置说明)
6. [故障排查](#故障排查)
7. [最佳实践](#最佳实践)

---

## 🚀 快速开始

### 1. 一键部署

```bash
chmod +x /root/deploy_token_gateway.sh
./deploy_token_gateway.sh
```

### 2. 创建管理员Token

```bash
python3 /root/init_token_gateway.py --create-admin admin
```

输出示例：
```
✅ 管理员Token创建成功
   用户名: admin
   Token: sk-tk-xxxxx...
   每日限额: 10,000,000 tokens
   有效期: 365天
```

### 3. 添加后端API Keys

```bash
python3 /root/init_token_gateway.py --add-key \
    "anthropic-main-1" \
    "sk-ant-api03-xxx" \
    100 \
    10
```

参数说明：
- `anthropic-main-1`: Key名称（唯一标识）
- `sk-ant-api03-xxx`: 实际的API Key
- `100`: 权重（用于负载均衡）
- `10`: 优先级（数字越大优先级越高）

### 4. 查看系统状态

```bash
python3 /root/token_gateway_manager.py --status
```

---

## 🎯 核心功能

### 1. Key池管理

支持多个后端API Keys，统一管理和调度：

```python
# 添加Key
POST /api/v1/keys
{
    "key_name": "anthropic-main-1",
    "api_key": "sk-ant-xxx",
    "provider": "anthropic",
    "weight": 100,
    "priority": 10
}

# 列出Keys
GET /api/v1/keys

# 更新Key
PUT /api/v1/keys/{key_id}
{
    "weight": 150,
    "priority": 15
}

# 删除Key
DELETE /api/v1/keys/{key_id}
```

### 2. 智能负载均衡

三种策略可选：

#### 策略1: 会话粘性 (推荐) ⭐
```
同一会话始终使用相同的Key
→ 确保对话连贯性
→ 提升用户体验
→ 减少Key切换
```

#### 策略2: 加权轮询
```
根据权重和优先级分配
→ 权重高的Key获得更多请求
→ 适合Key性能相近的场景
```

#### 策略3: 响应时间优先
```
实时监控响应时间
→ 优先使用响应最快的Key
→ 自动避障
```

### 3. 健康检查系统

**自动检测项：**
- ✅ 响应时间
- ✅ 成功率
- ✅ 连续失败次数
- ✅ 错误类型

**自动处理：**
- 🔄 降低健康分数
- ⚠️ 标记为不健康
- 🔒 触发熔断器
- ✅ 自动恢复

### 4. 故障切换

**触发条件：**
- 响应时间 > 10秒
- 连接超时 > 5秒
- 连续失败 > 3次
- 错误率 > 50%

**切换动作：**
```
1. 立即标记当前Key为不健康
2. 自动切换到其他健康Key
3. 记录失败日志
4. 继续处理用户请求
```

### 5. 会话管理

**会话粘性机制：**

```python
# 客户端请求时携带会话ID
headers = {
    "x-api-key": "sk-tk-xxxxx",  # 用户Token
    "x-session-id": "sess_12345"  # 会话ID（可选）
}

# 系统会：
# 1. 检查是否有该会话绑定的Key
# 2. 如果有且Key健康 → 使用绑定的Key
# 3. 如果没有或Key不健康 → 选择新Key并绑定
```

---

## 📡 API接口

### 1. 基础接口

#### 健康检查
```
GET /health
```

响应：
```json
{
    "status": "healthy",
    "total_keys": 5,
    "healthy_keys": 4,
    "unhealthy_keys": 1
}
```

#### 系统信息
```
GET /
```

### 2. Key管理接口

#### 列出所有Keys
```
GET /api/v1/keys
```

响应：
```json
{
    "keys": [
        {
            "id": 1,
            "key_name": "anthropic-main-1",
            "provider": "anthropic",
            "is_active": true,
            "weight": 100,
            "priority": 10,
            "health_score": 95,
            "status": "healthy",
            "avg_response_time": "850ms",
            "consecutive_failures": 0
        }
    ]
}
```

#### 添加Key
```
POST /api/v1/keys
Content-Type: application/json

{
    "key_name": "anthropic-backup-1",
    "api_key": "sk-ant-xxx",
    "provider": "anthropic",
    "weight": 100,
    "priority": 5
}
```

#### 更新Key
```
PUT /api/v1/keys/{key_id}
Content-Type: application/json

{
    "weight": 150,
    "priority": 15,
    "is_active": true
}
```

#### 删除Key
```
DELETE /api/v1/keys/{key_id}
```

### 3. 统计接口

#### 获取统计信息
```
GET /api/v1/stats
```

响应：
```json
{
    "total_requests_last_hour": 1234,
    "success_rate": "98.5%",
    "avg_response_time": "950ms",
    "keys_summary": {
        "total": 5,
        "healthy": 4,
        "unhealthy": 1
    }
}
```

### 4. 代理接口

#### 聊天接口
```
POST /v1/messages
Headers:
    x-api-key: sk-tk-xxxxx (用户Token)
    x-session-id: sess_12345 (可选，会话ID)
    anthropic-version: 2023-06-01

Body:
{
    "model": "claude-3-sonnet-20240229",
    "max_tokens": 1024,
    "messages": [
        {"role": "user", "content": "Hello!"}
    ]
}
```

---

## 🛠️ 管理工具

### 系统状态监控

```bash
# 查看完整状态
python3 /root/token_gateway_manager.py --status

# 输出示例：
# 📊 后端Keys状态 (共 5 个)
# ──────────────────────────────────────────────────────────────────
# 名称                 健康度   权重     优先级   状态         平均响应
# ──────────────────────────────────────────────────────────────────
# anthropic-main-1     95      100     10       ✅ 健康      850ms
# anthropic-backup-1   88      100     5        ✅ 健康      920ms
# ...
```

### Top Keys分析

```bash
# 查看Top 10 Keys
python3 /root/token_gateway_manager.py --top-keys 10
```

### 错误日志查看

```bash
# 查看最近20个错误
python3 /root/token_gateway_manager.py --errors 20
```

### 会话统计

```bash
# 查看会话统计
python3 /root/token_gateway_manager.py --sessions
```

### 健康趋势

```bash
# 查看24小时健康趋势
python3 /root/token_gateway_manager.py --trends 24

# 查看特定Key的趋势
python3 /root/token_gateway_manager.py --trends 24 --key-id 1
```

### 系统诊断

```bash
# 自动诊断系统问题
python3 /root/token_gateway_manager.py --diagnose
```

### 实时监控

```bash
# 实时监控模式（每5秒刷新）
python3 /root/token_gateway_manager.py --watch
```

---

## ⚙️ 配置说明

### 配置文件：`/root/token_gateway_config.yaml`

```yaml
# Key池配置
key_pool:
  health_check:
    enabled: true
    interval_seconds: 30      # 健康检查间隔
    timeout_seconds: 10       # 健康检查超时

  circuit_breaker:
    enabled: true
    failure_threshold: 10     # 连续失败触发熔断
    recovery_timeout: 300     # 熔断恢复时间(秒)

# 负载均衡配置
load_balancer:
  strategy: "session_sticky"  # 策略: session_sticky/weighted/response_time

  session_sticky:
    timeout_seconds: 3600     # 会话超时时间

# 限流配置
limits:
  max_concurrent_per_key: 10  # 每个Key最大并发
  max_concurrent_requests: 50  # 全局最大并发
```

### 调优建议

**场景1: 高并发 (> 1000 QPS)**
```yaml
limits:
  max_concurrent_per_key: 20
  max_concurrent_requests: 200

key_pool:
  health_check:
    interval_seconds: 15  # 更频繁的健康检查
```

**场景2: 低延迟要求**
```yaml
load_balancer:
  strategy: "response_time"  # 使用响应时间优先策略

limits:
  timeout:
    connection: 3
    read: 60
    total: 65
```

**场景3: 对话连贯性要求高**
```yaml
load_balancer:
  strategy: "session_sticky"

  session_sticky:
    timeout_seconds: 7200  # 延长会话时间到2小时
```

---

## 🔧 故障排查

### 问题1: 所有Key都不可用

**检查步骤：**
```bash
# 1. 查看系统状态
python3 /root/token_gateway_manager.py --status

# 2. 查看错误日志
python3 /root/token_gateway_manager.py --errors

# 3. 检查服务日志
journalctl -u token-gateway -n 50
```

**常见原因：**
- API Keys过期或失效
- 网络连接问题
- API提供商限流

**解决方案：**
```bash
# 检查每个Key的健康状态
curl http://localhost:8000/api/v1/keys

# 更新不健康的Key
curl -X PUT http://localhost:8000/api/v1/keys/1 \
    -H "Content-Type: application/json" \
    -d '{"is_active": false}'
```

### 问题2: 响应慢

**检查步骤：**
```bash
# 查看健康趋势
python3 /root/token_gateway_manager.py --trends

# 查看慢响应Keys
python3 /root/token_gateway_manager.py --diagnose
```

**优化方案：**
1. 增加Key数量（分散负载）
2. 提高健康Key的权重
3. 调整超时配置
4. 检查网络连接

### 问题3: 频繁切换Key

**原因：** 健康检查阈值设置过于敏感

**解决方案：**
```yaml
# 修改配置文件
key_pool:
  circuit_breaker:
    failure_threshold: 20  # 提高触发阈值
    recovery_timeout: 600  # 延长恢复时间
```

---

## 📖 最佳实践

### 1. Key管理最佳实践

**分层设计：**
```
第一层：高性能Key（优先级10）
  • 响应快，稳定性高
  • 处理80%的请求

第二层：普通Key（优先级5）
  • 性能一般
  • 处理15%的请求

第三层：应急Key（优先级1）
  • 仅在前两层不可用时使用
  • 处理5%的请求
```

**示例配置：**
```bash
# 添加高性能主Key
python3 /root/init_token_gateway.py --add-key \
    "anthropic-ultra-1" "sk-ant-xxx" 100 10

# 添加普通备用Key
python3 /root/init_token_gateway.py --add-key \
    "anthropic-std-1" "sk-ant-xxx" 100 5

# 添加应急Key
python3 /root/init_token_gateway.py --add-key \
    "anthropic-emergency" "sk-ant-xxx" 50 1
```

### 2. 监控告警

**建议监控指标：**
- 健康Key数量 < 2 → 立即告警
- 错误率 > 10% → 警告
- 响应时间 > 5秒 → 警告
- 所有Key不可用 → 紧急告警

**设置定时检查：**
```bash
# 添加到crontab
*/5 * * * * /root/token_gateway_manager.py --diagnose | mail -s "Gateway Alert" admin@example.com
```

### 3. 容量规划

**预估Key数量：**
```
单个Key容量：约50-100并发/秒

所需Key数 = 预期QPS / 50

例如：
  500 QPS → 10个Key
  1000 QPS → 20个Key
```

**建议冗余：**
```
生产环境: 实际需求数量 × 1.5
测试环境: 实际需求数量 × 1.2
```

### 4. 日常维护

**每日检查：**
```bash
python3 /root/token_gateway_manager.py --status
python3 /root/token_gateway_manager.py --diagnose
```

**每周任务：**
- 分析使用统计
- 评估Key性能
- 清理过期会话
- 备份数据库

**数据库备份：**
```bash
# 手动备份
cp /var/lib/token-gateway/gateway.db \
   /var/lib/token-gateway/backup/gateway_$(date +%Y%m%d).db

# 自动备份（添加到crontab）
0 2 * * * cp /var/lib/token-gateway/gateway.db /var/lib/token-gateway/backup/gateway_$(date +\%Y\%m\%d).db
```

---

## 📞 技术支持

遇到问题？

1. 查看日志：`journalctl -u token-gateway -f`
2. 运行诊断：`python3 /root/token_gateway_manager.py --diagnose`
3. 查看文档：`/root/token_gateway_architecture.md`

---

**版本**: v2.0.0
**更新时间**: 2026-01-31
