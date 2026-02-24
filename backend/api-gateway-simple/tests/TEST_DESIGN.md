# DogeAI API Gateway 全面测试设计文档

## 文档信息

| 项目 | 内容 |
|------|------|
| 项目名称 | DogeAI API Gateway 测试套件 |
| 版本 | v1.0 |
| 编制日期 | 2026-02-24 |
| 测试范围 | api-gateway-simple (端口 8081) |

---

## 一、测试环境配置

### 1.1 环境变量

```bash
# 数据库
export DATABASE_URL="postgresql://postgres:postgres@localhost:5432/dogeai_test"

# Redis
export REDIS_URL="redis://127.0.0.1:6379"

# JWT
export JWT_SECRET="test-secret-key-for-testing-32bytes-min"

# 服务器
export SERVER_ADDR="0.0.0.0:8081"
```

### 1.2 测试数据准备

```sql
-- 测试用户
INSERT INTO users (user_id, username, password_hash, email, tier, balance, token_version, status) VALUES
('test_user_001', 'testuser1', '8d969eef6ecad3c29a3a629280e686cf0c3f5d5a86aff3ca12020c923adc6c92', 'test1@example.com', 'base', 1000000, 0, 'active'),
('test_user_002', 'testuser2', '8d969eef6ecad3c29a3a629280e686cf0c3f5d5a86aff3ca12020c923adc6c92', 'test2@example.com', 'pro', 5000000, 0, 'active'),
('test_user_003', 'testuser3', '8d969eef6ecad3c29a3a629280e686cf0c3f5d5a86aff3ca12020c923adc6c92', 'test3@example.com', 'max', 10000000, 0, 'active'),
('test_user_admin', 'admin', '8d969eef6ecad3c29a3a629280e686cf0c3f5d5a86aff3ca12020c923adc6c92', 'admin@example.com', 'admin', 999999999, 0, 'active');

-- 测试充值卡
INSERT INTO recharge_cards (card_code, amount, used) VALUES
('TEST-CARD-0001', 100000, false),
('TEST-CARD-0002', 500000, false),
('TEST-CARD-EXPIRED', 200000, true),
('TEST-CARD-USED', 300000, true);

-- 测试提供商配置
INSERT INTO model_providers (provider_id, provider_name, provider_type, base_url, api_key, health_status, enabled) VALUES
('glm-test', 'GLM 测试提供商', 'glm', 'https://open.bigmodel.cn/api/paas/v4/', 'test-key-glm', 'healthy', true),
('qwen-test', '千问测试提供商', 'qwen', 'https://dashscope.aliyuncs.com/api/v1/', 'test-key-qwen', 'healthy', true);

-- 测试模型池
INSERT INTO model_pools (pool_id, pool_name, description, enabled, fallback_chain) VALUES
('opus', 'Opus 池', '高性能模型池', true, ARRAY['glm-test', 'qwen-test']),
('sonnet', 'Sonnet 池', '中等性能模型池', true, ARRAY['glm-test']),
('haiku', 'Haiku 池', '基础模型池', true, ARRAY['qwen-test']);

-- 测试路由规则
INSERT INTO routing_rules (pool_id, task_sequence_min, task_sequence_max, provider_id, priority, actual_model) VALUES
('opus', 0, 100, 'glm-test', 1, 'glm-4'),
('sonnet', 0, 100, 'qwen-test', 1, 'qwen-plus');
```

---

## 二、功能测试用例设计

### 2.1 认证模块测试 (40个用例)

#### 2.1.1 用户注册 (10个用例)

| 用例ID | 测试场景 | 输入数据 | 预期结果 |
|--------|----------|----------|----------|
| AUTH-REG-001 | 正常注册 | 用户名: validuser001, 密码: Pass123!, 邮箱: valid@example.com | 注册成功，返回 token |
| AUTH-REG-002 | 用户名已存在 | 用户名: test_user_001 | 返回 "用户名已存在" 错误 |
| AUTH-REG-003 | 邮箱已存在 | 邮箱: test1@example.com | 返回 "邮箱已存在" 错误 |
| AUTH-REG-004 | 密码太短 | 密码: abc | 返回 "密码长度至少6位" 错误 |
| AUTH-REG-005 | 用户名为空 | 用户名: "" | 返回 "用户名不能为空" 错误 |
| AUTH-REG-006 | 密码为空 | 密码: "" | 返回 "密码不能为空" 错误 |
| AUTH-REG-007 | 特殊字符用户名 | 用户名: user@#$ | 返回 "用户名格式无效" 错误 |
| AUTH-REG-008 | 超长用户名 | 用户名: aaaaaaaaaaaaaaaaaaaaaa (50+) | 返回 "用户名长度超出限制" |
| AUTH-REG-009 | 无效邮箱格式 | 邮箱: invalid-email | 返回 "邮箱格式无效" |
| AUTH-REG-010 | SQL注入测试 | 用户名: admin'; DROP TABLE users;-- | 安全拒绝，返回格式错误 |

#### 2.1.2 用户登录 (10个用例)

| 用例ID | 测试场景 | 输入数据 | 预期结果 |
|--------|----------|----------|----------|
| AUTH-LOGIN-001 | 正常登录 | 用户名: test_user_001, 密码: password123 | 返回 token 和用户信息 |
| AUTH-LOGIN-002 | 用户名不存在 | 用户名: nonexistentuser | 返回 "用户名或密码错误" |
| AUTH-LOGIN-003 | 密码错误 | 用户名: test_user_001, 密码: wrongpassword | 返回 "用户名或密码错误" |
| AUTH-LOGIN-004 | 空用户名 | 用户名: "" | 返回 "用户名不能为空" |
| AUTH-LOGIN-005 | 空密码 | 密码: "" | 返回 "密码不能为空" |
| AUTH-LOGIN-006 | 账户被暂停 | 用户名: suspended_user | 返回 "账户已被暂停" |
| AUTH-LOGIN-007 | 使用邮箱登录 | 邮箱: test1@example.com | 返回 token |
| AUTH-LOGIN-008 | 大小写敏感测试 | 用户名: TestUser_001 | 返回 "用户名或密码错误" |
| AUTH-LOGIN-009 | 帐号登录(字段) | 账号: test_user_001 | 返回 token (兼容字段) |
| AUTH-LOGIN-010 | 连续失败锁定 | 连续5次失败 | 第6次返回 "账户已锁定，请5分钟后重试" |

#### 2.1.3 Token验证 (10个用例)

| 用例ID | 测试场景 | 输入数据 | 预期结果 |
|--------|----------|----------|----------|
| AUTH-TOKEN-001 | 有效Token | 有效 Bearer token | 验证通过，返回用户信息 |
| AUTH-TOKEN-002 | 过期Token | 过期的 token | 返回 "Token已过期" |
| AUTH-TOKEN-003 | 无效签名 | 篡改的 token | 返回 "Token签名无效" |
| AUTH-TOKEN-004 | 缺少Authorization头 | 无 Authorization header | 返回 "缺少认证信息" |
| AUTH-TOKEN-005 | 错误的Bearer格式 | Bearer invalidformat | 返回 "认证格式错误" |
| AUTH-TOKEN-006 | x-api-key认证 | x-api-key: valid-api-key | 验证通过 |
| AUTH-TOKEN-007 | 被撤销的Token | 已撤销的 jti | 返回 "Token已被撤销" |
| AUTH-TOKEN-008 | Token版本不匹配 | 旧版本 token | 返回 "Token已失效" |
| AUTH-TOKEN-009 | 格式错误的JWT | malformed jwt | 返回 "Token格式错误" |
| AUTH-TOKEN-010 | 空Token | Bearer (空) | 返回 "Token不能为空" |

#### 2.1.4 用户登出 (10个用例)

| 用例ID | 测试场景 | 输入数据 | 预期结果 |
|--------|----------|----------|----------|
| AUTH-LOGOUT-001 | 正常登出 | 有效 token | Token被撤销，返回成功 |
| AUTH-LOGOUT-002 | 重复登出 | 同一token登出两次 | 第二次返回 "Token已失效" |
| AUTH-LOGOUT-003 | 无效Token登出 | 无效 token | 返回 "Token无效" |
| AUTH-LOGOUT-004 | 无Token登出 | 无 token | 返回 "缺少认证信息" |
| AUTH-LOGOUT-005 | 登出后使用Token | 登出后使用原token | 返回 "Token已失效" |
| AUTH-LOGOUT-006 | 并发登出 | 同一token并发登出 | 所有请求返回成功 |
| AUTH-LOGOUT-007 | 登出后重新登录 | 登出后登录获取新token | 获得新token并可用 |
| AUTH-LOGOUT-008 | 不同用户登出互不影响 | 用户A登出不影响用户B | 验证通过 |
| AUTH-LOGOUT-009 | 登出清理缓存 | 验证本地缓存清除 | 缓存中token被清除 |
| AUTH-LOGOUT-010 | 登出记录日志 | 验证日志记录 | 日志记录登出操作 |

---

### 2.2 聊天模块测试 (40个用例)

#### 2.2.1 基础聊天请求 (10个用例)

| 用例ID | 测试场景 | 输入数据 | 预期结果 |
|--------|----------|----------|----------|
| CHAT-001 | 正常聊天请求 | model: opus, messages: [{role: user, content: "hello"}] | 返回聊天响应 |
| CHAT-002 | 空消息列表 | messages: [] | 返回 "消息列表不能为空" |
| CHAT-003 | 单轮对话 | 单条用户消息 | 返回助手回复 |
| CHAT-004 | 多轮对话 | 多轮对话历史 | 返回上下文相关回复 |
| CHAT-005 | 系统消息 | 包含 system 消息 | 使用系统提示词 |
| CHAT-006 | 超长消息 | 10000+ 字符消息 | 正常处理或返回错误 |
| CHAT-007 | 特殊字符消息 | 包含 emoji、特殊符号 | 正常处理 |
| CHAT-008 | 模型不存在 | model: nonexistent_model | 返回 "模型不存在" |
| CHAT-009 | 余额不足 | balance < cost | 返回 "余额不足" |
| CHAT-010 | 流式请求 | stream: true | 返回流式响应 |

#### 2.2.2 模型选择 (10个用例)

| 用例ID | 测试场景 | 输入数据 | 预期结果 |
|--------|----------|----------|----------|
| CHAT-MODEL-001 | Opus模型 | model: opus | 使用 opus 池 |
| CHAT-MODEL-002 | Sonnet模型 | model: sonnet | 使用 sonnet 池 |
| CHAT-MODEL-003 | Haiku模型 | model: haiku | 使用 haiku 池 |
| CHAT-MODEL-004 | 模型别名(gpt-4) | model: gpt-4 | 映射到 opus 池 |
| CHAT-MODEL-005 | 模型别名(gpt-3.5) | model: gpt-3.5-turbo | 映射到 sonnet 池 |
| CHAT-MODEL-006 | 未知模型 | model: unknown | 返回 "模型不存在" |
| CHAT-MODEL-007 | 大小写不敏感 | model: OPUS | 正常映射到 opus |
| CHAT-MODEL-008 | 池不可用 | model: disabled_pool | 返回 "模型暂不可用" |
| CHAT-MODEL-009 | 多用户并发 | 不同用户同时请求 | 正常处理 |
| CHAT-MODEL-010 | 模型切换 | 同一会话切换模型 | 正常响应 |

#### 2.2.3 Token计算 (10个用例)

| 用例ID | 测试场景 | 输入数据 | 预期结果 |
|--------|----------|----------|----------|
| CHAT-TOKEN-001 | 正常计算 | input_tokens: 100, output: 50 | 返回正确费用 |
| CHAT-TOKEN-002 | 零输入token | input_tokens: 0 | 费用计算正确 |
| CHAT-TOKEN-003 | 零输出token | output_tokens: 0 | 费用计算正确 |
| CHAT-TOKEN-004 | 大量token | input: 100000, output: 50000 | 费用计算正确 |
| CHAT-TOKEN-005 | 模型费率 | 不同模型不同费率 | 使用正确费率 |
| CHAT-TOKEN-006 | 余额扣除 | 扣费前余额充足 | 余额正确减少 |
| CHAT-TOKEN-007 | 余额不足检测 | balance < cost | 返回错误不扣费 |
| CHAT-TOKEN-008 | 负余额保护 | balance < 0 | 返回错误 |
| CHAT-TOKEN-009 | 并发扣费 | 多请求同时扣费 | 余额正确 |
| CHAT-TOKEN-010 | 扣费事务 | 扣费失败回滚 | 事务正确处理 |

#### 2.2.4 流式响应 (10个用例)

| 用例ID | 测试场景 | 输入数据 | 预期结果 |
|--------|----------|----------|----------|
| CHAT-STREAM-001 | 正常流式 | stream: true | 返回 SSE 流 |
| CHAT-STREAM-002 | 流式中断 | 客户端断开 | 服务端正确处理 |
| CHAT-STREAM-003 | 流式格式 | 验证 SSE 格式 | 符合 SSE 规范 |
| CHAT-STREAM-004 | 流式分块 | data: chunk | 分块正确传输 |
| CHAT-STREAM-005 | 流式结束 | data: [DONE] | 正确标记结束 |
| CHAT-STREAM-006 | 流式重连 | 断线重连 | 恢复流式传输 |
| CHAT-STREAM-007 | 流式限流 | 高频流式请求 | 限流保护 |
| CHAT-STREAM-008 | 非流式请求 | stream: false | 返回完整响应 |
| CHAT-STREAM-009 | 流式token计算 | 流式过程中计算 | token累计正确 |
| CHAT-STREAM-010 | 流式错误处理 | 流程中错误 | 返回错误chunk |

---

### 2.3 用户管理模块测试 (30个用例)

#### 2.3.1 余额查询 (10个用例)

| 用例ID | 测试场景 | 预期结果 |
|--------|----------|----------|
| USER-BAL-001 | 正常余额查询 | 返回当前余额 |
| USER-BAL-002 | 余额为0 | 返回 0 |
| USER-BAL-003 | 大额余额 | 返回大额余额 |
| USER-BAL-004 | 负余额 | 返回负数(异常情况) |
| USER-BAL-005 | 未认证查询 | 返回 401 |
| USER-BAL-006 | 缓存命中 | 第二次查询从缓存 |
| USER-BAL-007 | 余额精度 | 小数精度正确 |
| USER-BAL-008 | 并发查询 | 多次查询结果一致 |
| USER-BAL-009 | 不同用户余额 | 各用户返回各自余额 |
| USER-BAL-010 | 余额更新后查询 | 更新后返回新余额 |

#### 2.3.2 用户资料 (10个用例)

| 用例ID | 测试场景 | 预期结果 |
|--------|----------|----------|
| USER-PROFILE-001 | 查看完整资料 | 返回所有用户字段 |
| USER-PROFILE-002 | 更新邮箱 | 邮箱更新成功 |
| USER-PROFILE-003 | 更新已存在邮箱 | 返回 "邮箱已被使用" |
| USER-PROFILE-004 | 更新用户名 | 用户名更新成功 |
| USER-PROFILE-005 | 无效邮箱格式 | 返回 "邮箱格式无效" |
| USER-PROFILE-006 | 敏感信息保护 | 密码等字段不返回 |
| USER-PROFILE-007 | 资料字段完整性 | 所有必需字段存在 |
| USER-PROFILE-008 | 用户等级显示 | tier 字段正确 |
| USER-PROFILE-009 | 创建时间显示 | created_at 正确 |
| USER-PROFILE-010 | 未认证查看资料 | 返回 401 |

#### 2.3.3 密码管理 (10个用例)

| 用例ID | 测试场景 | 输入数据 | 预期结果 |
|--------|----------|----------|----------|
| USER-PWD-001 | 正常修改密码 | 旧密码正确，新密码符合要求 | 修改成功 |
| USER-PWD-002 | 旧密码错误 | 旧密码错误 | 返回 "旧密码不正确" |
| USER-PWD-003 | 新密码太短 | 新密码 < 6位 | 返回 "密码至少6位" |
| USER-PWD-004 | 新旧密码相同 | 新旧密码相同 | 返回 "新密码不能与旧密码相同" |
| USER-PWD-005 | 无新密码 | 未提供新密码 | 返回 "新密码不能为空" |
| USER-PWD-006 | 弱密码检测 | 新密码: 123456 | 返回 "密码强度不足" |
| USER-PWD-007 | 特殊字符密码 | 密码包含特殊字符 | 修改成功 |
| USER-PWD-008 | 密码加密存储 | 验证密码哈希存储 | 密码哈希正确 |
| USER-PWD-009 | 修改后重新登录 | 新密码可登录 | 登录成功 |
| USER-PWD-010 | 并发修改密码 | 同时修改 | 最后一次修改生效 |

---

### 2.4 充值模块测试 (20个用例)

#### 2.4.1 充值卡兑换 (10个用例)

| 用例ID | 测试场景 | 输入数据 | 预期结果 |
|--------|----------|----------|----------|
| RECHARGE-001 | 正常充值 | 有效充值码 | 充值成功，余额增加 |
| RECHARGE-002 | 充值卡不存在 | 充值码: INVALID | 返回 "充值卡不存在" |
| RECHARGE-003 | 充值卡已使用 | 充值码: TEST-CARD-USED | 返回 "充值卡已被使用" |
| RECHARGE-004 | 充值卡已过期 | 充值码: TEST-CARD-EXPIRED | 返回 "充值卡已过期" |
| RECHARGE-005 | 空充值码 | 充值码: "" | 返回 "充值码不能为空" |
| RECHARGE-006 | 重复使用充值卡 | 同一充值码再次使用 | 返回 "已被使用" |
| RECHARGE-007 | 充值金额验证 | 验证金额正确添加 | 余额增加正确金额 |
| RECHARGE-008 | 充值后余额查询 | 充值后查询 | 返回更新后余额 |
| RECHARGE-009 | 并发充值冲突 | 多人同一充值码 | 只有一人成功 |
| RECHARGE-010 | 充值日志记录 | 验证充值记录日志 | 日志正确记录 |

#### 2.4.2 充值卡管理 (10个用例)

| 用例ID | 测试场景 | 预期结果 |
|--------|----------|----------|
| RECHARGE-CARD-001 | 创建充值卡 | 创建成功 |
| RECHARGE-CARD-002 | 创建重复码 | 返回冲突 |
| RECHARGE-CARD-003 | 删除充值卡 | 删除成功 |
| RECHARGE-CARD-004 | 查询充值卡列表 | 返回列表 |
| RECHARGE-CARD-005 | 充值卡分页 | 分页正确 |
| RECHARGE-CARD-006 | 禁用充值卡 | 禁用成功 |
| RECHARGE-CARD-007 | 启用充值卡 | 启用成功 |
| RECHARGE-CARD-008 | 批量创建充值卡 | 批量创建成功 |
| RECHARGE-CARD-009 | 充值卡过期时间 | 过期时间正确 |
| RECHARGE-CARD-010 | 充值卡使用统计 | 统计数据正确 |

---

### 2.5 管理后台测试 (30个用例)

#### 2.5.1 统计数据 (10个用例)

| 用例ID | 测试场景 | 预期结果 |
|--------|----------|----------|
| ADMIN-STATS-001 | 总用户数 | 返回正确总数 |
| ADMIN-STATS-002 | 活跃用户数 | 返回活跃用户数 |
| ADMIN-STATS-003 | 今日充值额 | 返回今日充值总额 |
| ADMIN-STATS-004 | 今日消费额 | 返回今日消费总额 |
| ADMIN-STATS-005 | API调用量 | 返回调用次数 |
| ADMIN-STATS-006 | 模型使用分布 | 返回各模型使用占比 |
| ADMIN-STATS-007 | 收入趋势 | 返回趋势数据 |
| ADMIN-STATS-008 | 用户增长趋势 | 返回增长数据 |
| ADMIN-STATS-009 | 实时统计 | 返回近实时数据 |
| ADMIN-STATS-010 | 时间范围统计 | 不同时间范围数据 |

#### 2.5.2 用户管理 (10个用例)

| 用例ID | 测试场景 | 预期结果 |
|--------|----------|----------|
| ADMIN-USER-001 | 用户列表 | 返回用户列表 |
| ADMIN-USER-002 | 用户搜索 | 按用户名/邮箱搜索 |
| ADMIN-USER-003 | 用户详情 | 返回完整用户信息 |
| ADMIN-USER-004 | 封禁用户 | 用户状态变为suspended |
| ADMIN-USER-005 | 解封用户 | 用户状态变为active |
| ADMIN-USER-006 | 调整用户等级 | tier更新成功 |
| ADMIN-USER-007 | 调整余额 | 余额调整成功 |
| ADMIN-USER-008 | 重置密码 | 密码重置成功 |
| ADMIN-USER-009 | 用户登录历史 | 返回登录历史 |
| ADMIN-USER-010 | 批量操作用户 | 批量操作成功 |

#### 2.5.3 API密钥管理 (10个用例)

| 用例ID | 测试场景 | 预期结果 |
|--------|----------|----------|
| ADMIN-KEY-001 | 创建API密钥 | 创建成功 |
| ADMIN-KEY-002 | 列出API密钥 | 返回密钥列表 |
| ADMIN-KEY-003 | 禁用API密钥 | 禁用成功 |
| ADMIN-KEY-004 | 启用API密钥 | 启用成功 |
| ADMIN-KEY-005 | 删除API密钥 | 删除成功 |
| ADMIN-KEY-006 | API密钥限流 | 设置限流规则 |
| ADMIN-KEY-007 | API密钥过期时间 | 设置过期时间 |
| ADMIN-KEY-008 | API密钥使用统计 | 返回使用统计 |
| ADMIN-KEY-009 | API密钥权限 | 设置权限 |
| ADMIN-KEY-010 | 重置API密钥 | 重置成功 |

---

### 2.6 限流模块测试 (20个用例)

#### 2.6.1 用户级限流 (10个用例)

| 用例ID | 测试场景 | 输入数据 | 预期结果 |
|--------|----------|----------|----------|
| RATE-USER-001 | Base用户限流 | tier: base, QPS: 10 | 超限返回 429 |
| RATE-USER-002 | Pro用户限流 | tier: pro, QPS: 100 | 超限返回 429 |
| RATE-USER-003 | Max用户限流 | tier: max, QPS: 200 | 超限返回 429 |
| RATE-USER-004 | 限流重置 | 等待时间窗口 | 限流重置 |
| RATE-USER-005 | 突发流量 | 瞬间高并发 | 触发限流 |
| RATE-USER-006 | 持续流量 | 持续请求 | 限流生效 |
| RATE-USER-007 | 等级升级 | tier升级后QPS变化 | 新QPS生效 |
| RATE-USER-008 | 限流记录 | 记录限流事件 | 日志记录 |
| RATE-USER-009 | 限流提示 | 返回限流信息 | 包含重置时间 |
| RATE-USER-010 | 白名单用户 | 白名单用户不限流 | 无限流 |

#### 2.6.2 IP限流 (10个用例)

| 用例ID | 测试场景 | 预期结果 |
|--------|----------|----------|
| RATE-IP-001 | 正常IP请求 | 正常处理 |
| RATE-IP-002 | IP超限 | 单IP超限返回429 |
| RATE-IP-003 | IP黑名单 | 黑名单IP拒绝 |
| RATE-IP-004 | IP白名单 | 白名单IP无限制 |
| RATE-IP-005 | 限流滑动窗口 | 滑动窗口正确计算 |
| RATE-IP-006 | 分布式IP限流 | Redis共享限流 |
| RATE-IP-007 | 限流恢复 | 时间窗口后恢复 |
| RATE-IP-008 | 代理IP检测 | 检测并拒绝代理 |
| RATE-IP-009 | 地理位置限流 | 按地区限流 |
| RATE-IP-010 | 限流粒度 | 细粒度限流 |

---

### 2.7 提供商管理测试 (30个用例)

#### 2.7.1 健康检查 (10个用例)

| 用例ID | 测试场景 | 预期结果 |
|--------|----------|----------|
| PROV-HEALTH-001 | 正常健康检查 | 返回健康状态 |
| PROV-HEALTH-002 | 提供商超时 | 超时返回不健康 |
| PROV-HEALTH-003 | 提供商401 | 401返回不健康 |
| PROV-HEALTH-004 | 提供商500 | 500返回不健康 |
| PROV-HEALTH-005 | 提供商恢复 | 恢复后变为健康 |
| PROV-HEALTH-006 | 健康检查频率 | 每60秒检查一次 |
| PROV-HEALTH-007 | 响应时间记录 | 记录响应时间 |
| PROV-HEALTH-008 | 连续失败计数 | 记录失败次数 |
| PROV-HEALTH-009 | 故障转移触发 | 5次失败触发转移 |
| PROV-HEALTH-010 | 健康状态持久化 | 状态保存到数据库 |

#### 2.7.2 故障转移 (10个用例)

| 用例ID | 测试场景 | 预期结果 |
|--------|----------|----------|
| FAILOVER-001 | 主提供商故障 | 自动切换备用 |
| FAILOVER-002 | 多级故障转移 | 按优先级转移 |
| FAILOVER-003 | 全部提供商故障 | 返回错误 |
| FAILOVER-004 | 提供商恢复 | 恢复后重新使用 |
| FAILOVER-005 | 故障转移日志 | 记录转移事件 |
| FAILOVER-006 | 转移延迟测试 | 转移<100ms |
| FAILOVER-007 | 转移一致性 | 数据一致性 |
| FAILOVER-008 | 转移回主 | 主恢复后切回 |
| FAILOVER-009 | 任务序号保留 | 转移保持上下文 |
| FAILOVER-010 | 通知告警 | 故障转移告警 |

#### 2.7.3 模型池管理 (10个用例)

| 用例ID | 测试场景 | 预期结果 |
|--------|----------|----------|
| POOL-001 | 创建模型池 | 池创建成功 |
| POOL-002 | 删除模型池 | 池删除成功 |
| POOL-003 | 启用/禁用池 | 状态更新成功 |
| POOL-004 | 池配置更新 | 配置更新生效 |
| POOL-005 | 池列表 | 返回所有池 |
| POOL-006 | 池统计 | 返回池统计信息 |
| POOL-007 | 池容量配置 | 容量限制生效 |
| POOL-008 | 池优先级 | 优先级排序 |
| POOL-009 | 池关联提供商 | 关联正确 |
| POOL-010 | 池性能监控 | 性能指标正确 |

---

## 三、性能测试用例设计

### 3.1 并发性能测试 (20个用例)

| 用例ID | 测试场景 | 并发数 | 持续时间 | 目标 |
|--------|----------|--------|----------|------|
| PERF-001 | 登录接口 | 100 | 10s | P99 < 100ms |
| PERF-002 | 聊天接口 | 500 | 30s | P99 < 2000ms |
| PERF-003 | 余额查询 | 1000 | 30s | P99 < 50ms |
| PERF-004 | 用户资料 | 200 | 10s | P99 < 100ms |
| PERF-005 | 健康检查 | 5000 | 60s | P99 < 10ms |
| PERF-006 | 流式聊天 | 100 | 30s | 首字节 < 100ms |
| PERF-007 | 并发登录 | 200 | 10s | 无错误 |
| PERF-008 | 并发充值 | 50 | 10s | 无余额错误 |
| PERF-009 | Token验证 | 5000 | 30s | P99 < 5ms (缓存) |
| PERF-010 | 中间件开销 | 1000 | 30s | <1ms overhead |

### 3.2 压力测试 (10个用例)

| 用例ID | 测试场景 | 并发数 | 持续时间 | 验证点 |
|--------|----------|--------|----------|--------|
| STRESS-001 | 极限并发 | 10000 | 5min | 无崩溃 |
| STRESS-002 | 长时间运行 | 100 | 24h | 无内存泄漏 |
| STRESS-003 | 大消息处理 | 100 | 10min | 无超时 |
| STRESS-004 | 连接数压力 | 50000 | 5min | 正常处理 |
| STRESS-005 | Redis压力 | 10000 qps | 10min | Redis稳定 |
| STRESS-006 | 数据库压力 | 1000 qps | 10min | DB稳定 |
| STRESS-007 | 缓存穿透 | 10000 qps | 5min | 缓存保护 |
| STRESS-008 | 缓存击穿 | 100 | 5min | 互斥锁生效 |
| STRESS-009 | 慢查询压力 | 1000 | 10min | 熔断生效 |
| STRESS-010 | 快速重启 | 重启10次 | 每次正常启动 |

### 3.3 内存和资源测试 (10个用例)

| 用例ID | 测试场景 | 目标 |
|--------|----------|------|
| MEM-001 | 内存泄漏检测 | 长时间运行无内存增长 |
| MEM-002 | 连接池管理 | 连接正确释放 |
| MEM-003 | 缓存大小控制 | 缓存有界 |
| MEM-004 | 请求体大小限制 | 超限请求被拒绝 |
| MEM-005 | 响应大小限制 | 大响应正确处理 |
| MEM-006 | 文件描述符 | 无FD泄漏 |
| MEM-007 | 线程池使用 | 线程正确回收 |
| MEM-008 | WebSocket连接 | 连接正确清理 |
| MEM-009 | 临时文件清理 | 临时文件被清理 |
| MEM-010 | GC压力测试 | GC正常工作 |

---

## 四、安全测试用例设计

### 4.1 认证安全 (20个用例)

| 用例ID | 测试场景 | 预期结果 |
|--------|----------|----------|
| SEC-AUTH-001 | SQL注入登录 | 被拒绝 |
| SEC-AUTH-002 | XSS在用户名 | 被过滤 |
| SEC-AUTH-003 | 暴力破解密码 | 账户锁定 |
| SEC-AUTH-004 | Token伪造 | 被拒绝 |
| SEC-AUTH-005 | Token重放 | 被拒绝(黑名单) |
| SEC-AUTH-006 | 会话固定攻击 | Token包含会话ID |
| SEC-AUTH-007 | CSRF攻击 | CSRF保护 |
| SEC-AUTH-008 | 密码明文传输 | 密码哈希传输 |
| SEC-AUTH-009 | 弱密码检测 | 提示强密码 |
| SEC-AUTH-010 | 多设备登录 | 支持多设备 |

### 4.2 API安全 (20个用例)

| 用例ID | 测试场景 | 预期结果 |
|--------|----------|----------|
| SEC-API-001 | 路径遍历攻击 | 被拒绝 |
| SEC-API-002 | 命令注入 | 被过滤 |
| SEC-API-003 | XXE攻击 | 被拒绝 |
| SEC-API-004 | SSRF攻击 | 外部URL限制 |
| SEC-API-005 | 文件包含攻击 | 被拒绝 |
| SEC-API-006 | CORS配置 | 正确CORS头 |
| SEC-API-007 | 速率限制 | 限流保护 |
| SEC-API-008 | 请求大小限制 | 超限被拒绝 |
| SEC-API-009 | 参数污染 | 参数验证 |
| SEC-API-010 | HTTP方法限制 | 只允许允许方法 |

---

## 五、测试执行计划

### 5.1 测试工具

| 工具 | 用途 |
|------|------|
| cargo test | Rust 单元测试 |
| reqwest | HTTP 客户端测试 |
| redis-cli | Redis 直接测试 |
| psql | PostgreSQL 直接测试 |
| wrk/ab | 压力测试 |
| locust | 性能测试 |
| JMeter | 性能测试 |

### 5.2 测试顺序

1. **第一阶段**: 单元测试 (30分钟)
2. **第二阶段**: 功能测试 API (2小时)
3. **第三阶段**: 性能测试 (1小时)
4. **第四阶段**: 安全测试 (1小时)
5. **第五阶段**: 压力测试 (30分钟)
6. **第六阶段**: 集成测试 (1小时)

### 5.3 测试环境准备

```bash
# 1. 启动测试数据库
docker-compose up -d postgres redis

# 2. 运行数据库迁移
cd backend/api-gateway-simple
sqlx database create --database-url $DATABASE_URL
sqlx migrate run --database-url $DATABASE_URL

# 3. 启动API服务
cargo run

# 4. 运行测试
cargo test
```

---

## 六、测试报告模板

### 6.1 功能测试报告

| 模块 | 用例总数 | 通过 | 失败 | 通过率 |
|------|----------|------|------|--------|
| 认证模块 | 40 | - | - | -% |
| 聊天模块 | 40 | - | - | -% |
| 用户管理 | 30 | - | - | -% |
| 充值模块 | 20 | - | - | -% |
| 管理后台 | 30 | - | - | -% |
| 限流模块 | 20 | - | - | -% |
| 提供商管理 | 30 | - | - | -% |
| **总计** | **210** | - | - | **-%** |

### 6.2 性能测试报告

| 指标 | 目标 | 实际 | 状态 |
|------|------|------|------|
| 登录API P99 | <100ms | -ms | ⏳ |
| 聊天API P99 | <2000ms | -ms | ⏳ |
| 余额查询 P99 | <50ms | -ms | ⏳ |
| 健康检查 P99 | <10ms | -ms | ⏳ |
| 最大并发 | 10000 | - | ⏳ |
| 内存使用 | <1GB | -MB | ⏳ |

---

## 七、缺陷跟踪

### 7.1 缺陷严重级别

- **P0 - 严重**: 系统崩溃、数据丢失、安全漏洞
- **P1 - 高**: 主要功能不可用
- **P2 - 中**: 次要功能受影响
- **P3 - 低**: UI问题、小错误

### 7.2 缺陷报告模板

| 缺陷ID | 模块 | 描述 | 严重级别 | 状态 |
|--------|------|------|----------|------|
| BUG-001 | - | - | - | ⏳ |

---

*编制日期: 2026-02-24*
*版本: v1.0*
