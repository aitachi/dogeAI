# 🔐 Aitachi.cloud 管理系统 - 配置完成文档

## ✅ 配置完成摘要

**登录系统**: 已创建并测试通过
**管理后台**: 已部署并可访问
**数据统计**: 实时显示所有Token使用情况

---

## 🔑 管理员登录信息

```
用户名: admin
密码: admin123
登录地址: http://aitachi.cloud/login.html
管理后台: http://aitachi.cloud/admin/dashboard.html
```

---

## 📊 管理系统功能

### 1. 登录页面
- ✅ 现代化登录界面
- ✅ 渐变紫色主题
- ✅ 错误提示功能
- ✅ 登录状态验证
- ✅ 自动跳转到管理后台

### 2. 管理仪表板

#### 数据统计卡片
- **总API密钥** - 已激活的密钥总数
- **今日调用次数** - 今日API请求总数
- **今日Token使用** - 今日消耗的Token数
- **活跃用户** - 今日有使用的用户数

#### API密钥列表
| 功能 | 说明 |
|------|------|
| 查看所有密钥 | 显示所有已生成的API密钥 |
| 搜索用户名 | 实时搜索和过滤 |
| 状态筛选 | 按激活/禁用状态筛选 |
| 使用率显示 | 可视化进度条显示 |
| 查看详情 | 点击查看完整密钥信息 |
| 启用/禁用 | 一键切换密钥状态 |

#### 密钥详细信息
- 用户名
- 完整API密钥
- 每日限额
- 已使用量
- 使用率百分比
- 创建时间
- 有效期至
- 激活状态

#### 最近调用记录
- 调用时间
- 用户名
- 使用的模型
- 输入Token数量
- 输出Token数量
- 总Token数量
- 调用状态（成功/失败）

---

## 🎯 当前API密钥统计

| # | 用户名 | 每日限额 | 今日已用 | 使用率 | 状态 |
|---|--------|----------|----------|--------|------|
| 1 | aitachi_cloud | 10,000,000 | 145 | 0.001% | ✅ 激活 |
| 2 | claude_code_public | 5,000,000 | 59 | 0.001% | ✅ 激活 |
| 3 | a01 | 10,000,000 | 398 | 0.004% | ✅ 激活 |
| 4 | test_user | 500,000 | 0 | 0% | ✅ 激活 |

**总计**: 4个API密钥
**今日总调用**: 12次
**今日总Token**: 602
**活跃用户**: 3个

---

## 🌐 访问地址

### 对外服务
- **网站首页**: http://aitachi.cloud
- **API服务**: http://aitachi.cloud/v1/
- **健康检查**: http://aitachi.cloud/health

### 管理系统
- **登录页面**: http://aitachi.cloud/login.html
- **管理后台**: http://aitachi.cloud/admin/dashboard.html

---

## 💡 使用步骤

### 第1步：访问登录页面
```
http://aitachi.cloud/login.html
```

### 第2步：输入登录凭据
```
用户名: admin
密码: admin123
```

### 第3步：点击登录按钮

### 第4步：自动跳转到管理后台
```
http://aitachi.cloud/admin/dashboard.html
```

### 第5步：查看和管理数据

**可执行的操作：**
- 📊 查看实时统计数据
- 🔑 查看所有API密钥
- 🔍 搜索特定用户
- 📈 查看使用统计
- ⏯️ 启用/禁用密钥
- 🔄 刷新数据
- 📝 查看调用记录

---

## 📋 管理后台API接口

### 1. 获取所有密钥
```bash
curl http://aitachi.cloud/admin/keys \
  -H "x-admin-key: admin-change-this-key"
```

### 2. 获取统计数据
```bash
curl "http://aitachi.cloud/admin/stats?days=7" \
  -H "x-admin-key: admin-change-this-key"
```

### 3. 获取最近调用
```bash
curl "http://aitachi.cloud/admin/recent-calls?limit=50" \
  -H "x-admin-key: admin-change-this-key"
```

### 4. 切换密钥状态
```bash
curl -X POST http://aitachi.cloud/admin/toggle-token \
  -H "x-admin-key: admin-change-this-key" \
  -H "content-type: application/json" \
  -d '{"token": "sk-xxxxx"}'
```

### 5. 获取密钥详情
```bash
curl "http://aitachi.cloud/admin/token-stats?token=sk-xxxxx&days=7" \
  -H "x-admin-key: admin-change-this-key"
```

---

## 🔧 修改管理员密码

### 方法1：修改前端密码

编辑文件：`/var/www/aitachi.cloud/login.html`

找到以下代码并修改：
```javascript
const ADMIN_CREDENTIALS = {
    username: 'admin',      // 修改用户名
    password: 'admin123'    // 修改密码
};
```

### 方法2：修改后端API密钥

编辑文件：`/root/anthropic_proxy_v2.py`

找到以下代码并修改：
```python
CONFIG["ADMIN_KEY"] = "admin-change-this-key"  # 修改为新密钥
```

然后重启服务：
```bash
systemctl restart anthropic-proxy
```

---

## 🔒 安全建议

### ⚠️ 重要提示

1. **立即修改默认密码**
   - 前端登录密码：admin/admin123
   - 后端API密钥：admin-change-this-key

2. **定期更换密码**
   - 建议每月更换一次
   - 使用强密码

3. **限制管理访问**
   - 使用VPN访问
   - 限制IP白名单
   - 启用HTTPS

4. **监控日志**
   - 定期检查访问日志
   - 监控异常登录
   - 记录管理操作

5. **备份数据**
   - 定期备份数据库
   - 备份配置文件
   - 备份日志文件

---

## 📁 文件位置

```
/var/www/aitachi.cloud/
├── index.html              # 网站首页
├── login.html              # 登录页面
└── admin/
    └── dashboard.html      # 管理仪表板

/etc/nginx/conf.d/
└── aitachi.conf            # Nginx配置

/root/
├── anthropic_proxy_v2.py   # API代理服务
└── test-admin-system.sh    # 测试脚本
```

---

## 🧪 测试命令

### 快速测试
```bash
bash /root/test-admin-system.sh
```

### 手动测试
```bash
# 测试登录页面
curl -I http://aitachi.cloud/login.html

# 测试API密钥列表
curl http://aitachi.cloud/admin/keys \
  -H "x-admin-key: admin-change-this-key" | python3 -m json.tool

# 测试统计数据
curl "http://aitachi.cloud/admin/stats?days=1" \
  -H "x-admin-key: admin-change-this-key" | python3 -m json.tool
```

---

## 🎉 配置完成！

**DNS生效后，用户可以通过以下方式访问：**

1. **网站首页**: http://aitachi.cloud
   - 查看服务介绍
   - 了解创始人信息
   - 查看API文档

2. **管理登录**: http://aitachi.cloud/login.html
   - 使用admin/admin123登录
   - 查看Token使用情况
   - 管理API密钥

3. **API服务**: http://aitachi.cloud/v1/
   - 正常的API调用
   - 需要有效的API Key

---

**生成时间**: 2026-01-29
**版本**: v2.0.0
**域名**: aitachi.cloud
**团队**: 张总、时义兵、Aitachi
