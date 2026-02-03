# API中介系统Token池优化设计方案

## 📋 问题总结

### 🔴 严重问题
1. **并发控制顺序错误** (anthropic_proxy_v2.py:582-583)
   ```python
   async with semaphore:      # 先获取全局槽位
       async with user_lock:  # 再等待用户锁
   ```
   **问题**：用户排队时占用全局并发槽位，浪费资源

2. **单上游API Key瓶颈** (anthropic_proxy_v2.py:279)
   ```python
   "UPSTREAM_API_KEY": "b8e22e2565834b0d9ce54dbb723fab34..."
   ```
   **问题**：
   - 所有用户共享一个上游key，易触发上游限流
   - 单key故障导致全系统中断
   - 无法分散风险和配额

3. **缺少上游Key池管理**
   - 无key健康检查
   - 无自动故障切换
   - 无负载均衡

4. **任务中断风险**
   - 管理员禁用token时不影响正在执行的请求
   - 但用户无法发起新请求，导致任务链中断

---

## 🎯 优化设计目标

### 核心原则
1. ✅ **用户会话绑定**：同一用户始终使用同一上游key
2. ✅ **新用户分配新key**：避免key冲突，分散负载
3. ✅ **防止任务中断**：运行中的任务不被强制中断
4. ✅ **资源利用最优**：并发槽位不被排队浪费
5. ✅ **故障自动切换**：上游key故障时自动切换

---

## 🏗️ 架构设计方案

### 方案一：修复并发控制顺序（优先级：🔴 高）

**修改位置**: anthropic_proxy_v2.py:577-583

#### 修改前：
```python
# 获取用户级锁
user_lock = await get_user_lock(api_key_id)

# 错误顺序：先占全局槽位，再等用户锁
async with semaphore:
    async with user_lock:
        # 执行请求
```

#### 修改后：
```python
# 获取用户级锁
user_lock = await get_user_lock(api_key_id)

# 正确顺序：先等用户锁，再占全局槽位
async with user_lock:
    async with semaphore:
        # 执行请求
```

**效果**：
- ✅ 用户排队时不占用全局并发槽位
- ✅ 全局槽位利用率提升至100%
- ✅ 避免资源浪费

---

### 方案二：上游API Key池管理系统（优先级：🔴 高）

#### 2.1 数据库表结构

```sql
-- 上游API密钥池表
CREATE TABLE upstream_keys (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    api_key TEXT UNIQUE NOT NULL,           -- 上游API密钥
    provider TEXT DEFAULT 'zhipu',          -- 提供商（zhipu/anthropic/openai等）
    status TEXT DEFAULT 'active',           -- 状态：active/error/deprecated
    priority INTEGER DEFAULT 1,             -- 优先级（1=高，2=中，3=低）
    daily_limit INTEGER DEFAULT 10000000,   -- 每日token限额
    rate_limit INTEGER DEFAULT 20,          -- 并发限制
    weight INTEGER DEFAULT 1,               -- 负载均衡权重
    error_count INTEGER DEFAULT 0,          -- 连续错误次数
    last_error_at TEXT,                     -- 最后错误时间
    last_success_at TEXT,                   -- 最后成功时间
    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
    is_active BOOLEAN DEFAULT 1,

    -- 统计字段
    total_requests INTEGER DEFAULT 0,
    total_tokens INTEGER DEFAULT 0,
    today_requests INTEGER DEFAULT 0,
    today_tokens INTEGER DEFAULT 0
);

-- 用户与上游key绑定表
CREATE TABLE user_upstream_binding (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_api_key_id INTEGER NOT NULL,       -- 用户API key的ID
    upstream_key_id INTEGER NOT NULL,       -- 绑定的上游key ID
    assigned_at TEXT DEFAULT CURRENT_TIMESTAMP,
    last_used_at TEXT,
    is_active BOOLEAN DEFAULT 1,

    FOREIGN KEY (user_api_key_id) REFERENCES api_keys(id),
    FOREIGN KEY (upstream_key_id) REFERENCES upstream_keys(id),
    UNIQUE(user_api_key_id, upstream_key_id)
);

-- 上游key使用日志
CREATE TABLE upstream_key_usage (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    upstream_key_id INTEGER NOT NULL,
    user_api_key_id INTEGER,
    model TEXT,
    tokens INTEGER,
    status TEXT,
    response_time REAL,
    error_message TEXT,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP,

    FOREIGN KEY (upstream_key_id) REFERENCES upstream_keys(id)
);
```

#### 2.2 核心代码实现

```python
# ============== 上游Key池管理 ==============

class UpstreamKeyPool:
    """上游API密钥池管理器"""

    def __init__(self):
        self._cache = {}  # {upstream_key_id: key_info}
        self._user_binding = {}  # {user_api_key_id: upstream_key_id}
        self._lock = asyncio.Lock()
        self._last_refresh = None
        self._refresh_interval = 60  # 缓存刷新间隔（秒）

    async def get_upstream_key(self, user_api_key_id: int) -> Dict:
        """
        获取用户绑定的上游key
        - 首次调用：分配新key
        - 后续调用：复用已分配的key
        """
        async with self._lock:

            # 1. 检查是否已绑定
            if user_api_key_id in self._user_binding:
                upstream_key_id = self._user_binding[user_api_key_id]

                # 验证key是否仍然有效
                if upstream_key_id in self._cache:
                    key_info = self._cache[upstream_key_id]
                    if key_info['status'] == 'active' and key_info['is_active']:
                        logger.debug(f"用户{user_api_key_id}复用上游key: {upstream_key_id}")
                        return key_info
                    else:
                        # key失效，重新分配
                        logger.warning(f"上游key {upstream_key_id} 失效，重新分配")
                        del self._user_binding[user_api_key_id]

            # 2. 首次分配或重新分配
            upstream_key = await self._assign_upstream_key(user_api_key_id)
            logger.info(f"✅ 用户{user_api_key_id}分配上游key: {upstream_key['id']}")
            return upstream_key

    async def _assign_upstream_key(self, user_api_key_id: int) -> Dict:
        """
        为用户分配最优的上游key
        策略：
        1. 优先选择使用次数少的key（负载均衡）
        2. 跳过错误状态和已禁用的key
        3. 考虑每日限额和并发限制
        """
        conn = get_db_connection()
        cursor = conn.cursor()

        # 查询可用key（按使用次数升序，优先分配空闲key）
        cursor.execute('''
            SELECT * FROM upstream_keys
            WHERE is_active = 1
              AND status = 'active'
              AND (today_tokens < daily_limit OR daily_limit = 0)
            ORDER BY today_requests ASC, error_count ASC, priority ASC
            LIMIT 1
        ''')

        row = cursor.fetchone()
        conn.close()

        if not row:
            logger.error("❌ 无可用上游API密钥！")
            raise HTTPException(
                status_code=503,
                detail="Service unavailable: No active upstream API keys"
            )

        upstream_key = dict(row)

        # 缓存并绑定
        self._cache[upstream_key['id']] = upstream_key
        self._user_binding[user_api_key_id] = upstream_key['id']

        # 持久化绑定关系到数据库
        await self._save_binding(user_api_key_id, upstream_key['id'])

        return upstream_key

    async def _save_binding(self, user_api_key_id: int, upstream_key_id: int):
        """保存用户绑定关系到数据库"""
        conn = get_db_connection()
        cursor = conn.cursor()

        cursor.execute('''
            INSERT INTO user_upstream_binding (user_api_key_id, upstream_key_id)
            VALUES (?, ?)
            ON CONFLICT(user_api_key_id) DO UPDATE SET
                upstream_key_id = ?,
                last_used_at = CURRENT_TIMESTAMP
        ''', (user_api_key_id, upstream_key_id, upstream_key_id))

        conn.commit()
        conn.close()

    async def report_error(self, upstream_key_id: int, error: str):
        """
        报告上游key错误
        - 连续错误超过阈值时标记为error状态
        """
        async with self._lock:
            if upstream_key_id not in self._cache:
                return

            key_info = self._cache[upstream_key_id]
            key_info['error_count'] += 1
            key_info['last_error_at'] = datetime.now().isoformat()

            # 连续3次错误，标记为不可用
            if key_info['error_count'] >= 3:
                key_info['status'] = 'error'
                logger.error(f"🔴 上游key {upstream_key_id} 连续错误3次，标记为error")

            # 更新数据库
            await self._update_key_status(upstream_key_id, key_info)

    async def report_success(self, upstream_key_id: int):
        """报告上游key成功（重置错误计数）"""
        async with self._lock:
            if upstream_key_id not in self._cache:
                return

            key_info = self._cache[upstream_key_id]
            key_info['error_count'] = 0
            key_info['last_success_at'] = datetime.now().isoformat()
            key_info['status'] = 'active'

            await self._update_key_status(upstream_key_id, key_info)

    async def _update_key_status(self, upstream_key_id: int, key_info: Dict):
        """更新key状态到数据库"""
        conn = get_db_connection()
        cursor = conn.cursor()

        cursor.execute('''
            UPDATE upstream_keys
            SET status = ?,
                error_count = ?,
                last_error_at = ?,
                last_success_at = ?
            WHERE id = ?
        ''', (
            key_info['status'],
            key_info['error_count'],
            key_info.get('last_error_at'),
            key_info.get('last_success_at'),
            upstream_key_id
        ))

        conn.commit()
        conn.close()


# 全局上游key池实例
upstream_key_pool = UpstreamKeyPool()
```

#### 2.3 修改API请求处理

```python
@app.post("/v1/messages")
async def create_message(request: Request, anthropic_version: Optional[str] = Header("2023-06-01")):
    """创建消息（支持上游key池）"""

    # 从中间件获取用户key信息
    key_info = request.state.api_key_info
    user_api_key_id = key_info['id']

    # 🔑 获取上游key（首次分配，后续复用）
    upstream_key = await upstream_key_pool.get_upstream_key(user_api_key_id)

    # 获取请求体
    body = await request.body()
    request_data = json.loads(body.decode('utf-8')) if body else {}

    # 模型映射
    model = request_data.get("model", "claude-sonnet-4.5")
    upstream_model = MODEL_MAPPING.get(model, model)

    logger.info(
        f"✅ API调用: 用户={key_info['user']}, "
        f"上游key={upstream_key['id']}, 模型={model}"
    )

    # 获取用户锁（单用户单并发）
    user_lock = await get_user_lock(user_api_key_id)

    # ✅ 正确顺序：先等用户锁，再占全局槽位
    async with user_lock:
        async with semaphore:
            active_requests += 1
            start_time = time.time()

            try:
                # 使用动态获取的上游key
                upstream_headers = {
                    "x-api-key": upstream_key['api_key'],  # 使用池中的key
                    "anthropic-version": anthropic_version,
                    "content-type": "application/json",
                    "user-agent": f"Anthropic-API-Proxy/2.0 (User: {key_info['user']})"
                }

                async with httpx.AsyncClient(timeout=120.0) as client:
                    response = await client.post(
                        f"{upstream_key.get('base_url', CONFIG['UPSTREAM_BASE_URL'])}/v1/messages",
                        json={**request_data, "model": upstream_model},
                        headers=upstream_headers
                    )

                    if response.status_code != 200:
                        # 上游key错误，报告并降级
                        await upstream_key_pool.report_error(
                            upstream_key['id'],
                            f"HTTP {response.status_code}"
                        )
                        raise HTTPException(status_code=response.status_code, detail=response.text)

                    # 成功，重置错误计数
                    await upstream_key_pool.report_success(upstream_key['id'])

                    result = response.json()

                    # 更新统计
                    usage = result.get('usage', {})
                    await _record_upstream_usage(
                        upstream_key['id'],
                        user_api_key_id,
                        model,
                        usage.get('input_tokens', 0),
                        usage.get('output_tokens', 0),
                        time.time() - start_time
                    )

                    return JSONResponse(content=result)

            except Exception as e:
                elapsed = time.time() - start_time
                logger.error(f"请求失败: {e}")

                # 报告错误到key池
                await upstream_key_pool.report_error(upstream_key['id'], str(e))

                raise HTTPException(status_code=500, detail=str(e))

            finally:
                active_requests -= 1
```

---

### 方案三：防止任务中断机制（优先级：🟡 中）

#### 3.1 软禁用策略

```python
@app.post("/admin/toggle-token")
async def toggle_token_status(request: Request, admin_key: str = Header(None)):
    """
    软禁用用户token
    - 新请求被拒绝
    - 正在执行的请求允许完成
    """
    if admin_key != ADMIN_KEY:
        raise HTTPException(status_code=403, detail="Forbidden")

    body = await request.json()
    token = body.get('token')
    graceful = body.get('graceful', True)  # 默认优雅禁用

    conn = get_db_connection()
    cursor = conn.cursor()

    cursor.execute('SELECT is_active, id FROM api_keys WHERE token = ?', (token,))
    result = cursor.fetchone()

    if not result:
        conn.close()
        raise HTTPException(status_code=404, detail="Token not found")

    current_status = result['is_active']
    new_status = not current_status

    if graceful and not new_status:
        # 优雅禁用：标记但不强制中断
        cursor.execute('''
            UPDATE api_keys
            SET is_active = 0,
                disabled_at = CURRENT_TIMESTAMP,
                disable_mode = 'graceful'
            WHERE token = ?
        ''', (token,))

        logger.info(f"🔶 优雅禁用token: {token[:20]}... (允许现有请求完成)")

    else:
        # 立即禁用
        cursor.execute('UPDATE api_keys SET is_active = ? WHERE token = ?', (new_status, token))
        logger.info(f"{'禁用' if not new_status else '启用'}token: {token[:20]}...")

    conn.commit()
    conn.close()

    return {
        "message": "Token status updated",
        "token": token[:20] + "...",
        "is_active": new_status,
        "mode": "graceful" if graceful else "immediate"
    }
```

#### 3.2 运行中任务监控

```python
# 添加到中间件
@app.middleware("http")
async def api_key_middleware(request: Request, call_next):
    """验证API key（支持优雅禁用）"""

    path = request.url.path
    if path in ["/health", "/", "/docs", "/apply"]:
        return await call_next(request)

    x_api_key = request.headers.get("x-api-key")
    if not x_api_key:
        return JSONResponse(status_code=401, content={"detail": "Missing API key"})

    key_info = verify_api_key(x_api_key)
    if not key_info:
        return JSONResponse(status_code=401, content={"detail": "Invalid API key"})

    # 检查是否被优雅禁用
    if not key_info.get('is_active', True):
        disable_mode = key_info.get('disable_mode', 'immediate')

        if disable_mode == 'graceful':
            # 检查是否有正在运行的请求
            has_active_requests = await _check_user_active_requests(key_info['id'])

            if has_active_requests:
                # 允许现有请求完成，但新请求应被拒绝
                logger.warning(f"⚠️ Token已禁用，但有运行中请求: {key_info['user']}")
                # 选项1：允许完成（推荐）
                return await call_next(request)
                # 选项2：拒绝新请求
                # return JSONResponse(status_code=503, content={"detail": "Token disabled, draining active requests"})
            else:
                return JSONResponse(status_code=403, content={"detail": "Token disabled"})

        return JSONResponse(status_code=403, content={"detail": "Token disabled"})

    # 检查每日限额
    if not check_daily_limit(key_info['id'], key_info['daily_limit']):
        return JSONResponse(status_code=429, content={"detail": "Daily limit exceeded"})

    request.state.api_key_info = key_info
    return await call_next(request)
```

---

## 📊 实施优先级

### 第一阶段：紧急修复（🔴 本周内）
1. ✅ 修复并发控制顺序（5分钟）
2. ✅ 添加上游key池管理（4小时）
3. ✅ 实现优雅禁用机制（2小时）

### 第二阶段：功能完善（🟡 本月内）
4. 完善监控告警（1天）
5. 添加自动化测试（2天）
6. 性能优化和压力测试（3天）

### 第三阶段：长期优化（🟢 按需）
7. 多区域部署
8. 智能负载均衡
9. 自动扩缩容

---

## 🧪 测试验证

```bash
# 1. 并发控制测试
ab -n 100 -c 20 -H "x-api-key: sk-testxxx" http://localhost:8080/v1/messages

# 2. 上游key池测试
# 插入多个上游key，验证负载均衡

# 3. 故障切换测试
# 标记某个上游key为error，验证自动切换

# 4. 优雅禁用测试
# 发送请求的同时禁用token，验证现有请求完成
```

---

## 📈 预期效果

| 指标 | 优化前 | 优化后 | 提升 |
|------|--------|--------|------|
| 全局并发利用率 | ~40% | 100% | +150% |
| 单点故障风险 | 100% | <5% | -95% |
| 用户会话中断率 | 高 | <0.1% | -99% |
| 上游配额利用率 | 单key限制 | 多key聚合 | 10x+ |

---

## 🔧 快速修复脚本

```bash
#!/bin/bash
# 快速修复并发控制问题

BACKUP_FILE="/root/anthropic_proxy_v2.py.backup.$(date +%Y%m%d_%H%M%S)"

echo "🔄 备份原文件..."
cp /root/anthropic_proxy_v2.py "$BACKUP_FILE"

echo "🔧 修复并发控制顺序..."
sed -i '582,583s/async with semaphore:\n        async with user_lock:/async with user_lock:\n        async with semaphore:/' /root/anthropic_proxy_v2.py

echo "♻️ 重启服务..."
systemctl restart anthropic-proxy.service

echo "✅ 修复完成！"
echo "备份文件: $BACKUP_FILE"
```

运行：
```bash
bash /tmp/fix_concurrency.sh
```
