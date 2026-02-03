# Anthropic API 代理服务 - 配置文档

## 📋 服务信息

**服务器地址**: `59.110.40.73`
**服务端口**: `8081` (推荐), `8080` (直连)
**版本**: v2.0.0
**状态**: ✅ 生产级配置

---

## 🚀 核心功能

### 1. 并发控制
- ✅ **20个并发请求**同时处理
- ✅ 智能队列管理
- ✅ 自动负载均衡

### 2. 模型映射
支持最新模型名称：
- `claude-sonnet-4.5` → 智谱AI后端
- `claude-opus-4.5` → 智谱AI后端
- `claude-3.5-sonnet`
- `claude-3.5-haiku`
- `claude-3-opus`

### 3. 完整统计
- ✅ 每个API密钥的调用次数
- ✅ Token使用统计
- ✅ 请求历史记录
- ✅ 每日限额管理

### 4. 服务保障
- ✅ systemd自动启动
- ✅ 崩溃自动重启
- ✅ 日志轮转(100MB x 10)
- ✅ 数据持久化(SQLite)

---

## 📊 客户端配置

### Python
```python
from anthropic import Anthropic

client = Anthropic(
    api_key="sk-dMUtFZ9IWedPSgSd0WMkH7XhP0idZVtAVnp48MYgNLo",
    base_url="http://59.110.40.73"
)

message = client.messages.create(
    model="claude-sonnet-4.5",
    max_tokens=1024,
    messages=[{"role": "user", "content": "你好"}]
)

print(message.content)
```

### cURL
```bash
curl -X POST http://59.110.40.73/v1/messages \
  -H "x-api-key: sk-dMUtFZ9IWedPSgSd0WMkH7XhP0idZVtAVnp48MYgNLo" \
  -H "anthropic-version: 2023-06-01" \
  -H "content-type: application/json" \
  -d '{
    "model": "claude-sonnet-4.5",
    "max_tokens": 1024,
    "messages": [{"role": "user", "content": "你好"}]
  }'
```

### 环境变量
```bash
export ANTHROPIC_API_KEY="sk-dMUtFZ9IWedPSgSd0WMkH7XhP0idZVtAVnp48MYgNLo"
export ANTHROPIC_BASE_URL="http://59.110.40.73"
```

---

## 🔧 管理工具

### 使用管理脚本
```bash
/root/api-proxy-admin.sh
```

功能：
1. 查看服务状态
2. 管理API密钥
3. 查看调用统计
4. 实时日志查看
5. 重启服务

### API接口

**健康检查**
```bash
curl http://59.110.40.73/health
```

**模型列表**
```bash
curl http://59.110.40.73/v1/models
```

**创建API密钥**
```bash
curl -X POST "http://59.110.40.73/admin/keys/create?user=new_user&limit=1000000&days=365" \
  -H "x-admin-key: admin-change-this-key"
```

**查看统计**
```bash
# 全局统计
curl "http://59.110.40.73/admin/stats?days=7" \
  -H "x-admin-key: admin-change-this-key"

# 特定密钥统计
curl "http://59.110.40.73/admin/stats?api_key=sk-xxx&days=7" \
  -H "x-admin-key: admin-change-this-key"
```

---

## 📁 重要文件位置

```
# 服务代码
/root/anthropic_proxy_v2.py

# 数据库
/var/lib/anthropic-proxy/stats.db

# 日志文件
/var/log/anthropic-proxy/api-proxy.log

# systemd服务
/etc/systemd/system/anthropic-proxy.service

# Nginx配置
/etc/nginx/conf.d/anthropic-proxy-v2.conf

# 管理工具
/root/api-proxy-admin.sh
```

---

## 🔒 安全建议

1. **修改默认管理员密钥**
   ```bash
   # 编辑代码，修改以下行：
   if admin_key != "admin-change-this-key":
   ```

2. **限制管理接口访问**
   ```nginx
   # 在Nginx配置中取消注释：
   # allow 172.16.0.0/12;
   # allow 127.0.0.1;
   # deny all;
   ```

3. **定期备份数据库**
   ```bash
   cp /var/lib/anthropic-proxy/stats.db /backup/stats.db.$(date +%Y%m%d)
   ```

---

## ⚡ 性能优化

### 当前配置
- **并发数**: 20
- **超时时间**: 120秒
- **日志大小**: 100MB x 10文件

### 调整并发数
编辑 `/root/anthropic_proxy_v2.py`:
```python
MAX_CONCURRENT_REQUESTS = 20  # 修改这个值
```

重启服务：
```bash
systemctl restart anthropic-proxy
```

---

## 🛠️ 常用命令

```bash
# 查看服务状态
systemctl status anthropic-proxy

# 启动服务
systemctl start anthropic-proxy

# 停止服务
systemctl stop anthropic-proxy

# 重启服务
systemctl restart anthropic-proxy

# 查看日志
journalctl -u anthropic-proxy -f

# 查看API日志
tail -f /var/log/anthropic-proxy/api-proxy.log

# 查看Nginx日志
tail -f /var/log/nginx/anthropic-proxy-access.log
```

---

## 📞 支持

- 服务状态: `http://59.110.40.73/health`
- API文档: `http://59.110.40.73/docs` (FastAPI自动生成)

---

**生成时间**: 2026-01-29
**版本**: v2.0.0
