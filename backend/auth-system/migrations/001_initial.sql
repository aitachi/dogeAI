-- 认证系统数据库初始化脚本

-- 用户表
CREATE TABLE IF NOT EXISTS users (
    id BIGSERIAL PRIMARY KEY,

    -- 基本信息
    username VARCHAR(100) NOT NULL UNIQUE,
    email VARCHAR(255) NOT NULL UNIQUE,
    password_hash VARCHAR(255) NOT NULL,

    -- 用户等级和权限
    tier VARCHAR(50) NOT NULL DEFAULT 'Base',
    scopes TEXT[] NOT NULL DEFAULT ARRAY['read'],

    -- 账户状态
    status VARCHAR(50) NOT NULL DEFAULT 'active',
    status_reason TEXT,

    -- Token管理
    token_version INTEGER NOT NULL DEFAULT 0,

    -- 积分余额
    balance BIGINT NOT NULL DEFAULT 0,
    daily_points BIGINT NOT NULL DEFAULT 0,

    -- 时间戳
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- 暂停时间
    suspended_at TIMESTAMPTZ,

    -- 索引
    CONSTRAINT valid_tier CHECK (
        tier IN ('Base', 'Pro', 'Max', 'AMax', 'Enterprise')
    ),
    CONSTRAINT valid_status CHECK (
        status IN ('active', 'suspended', 'banned', 'deleted')
    )
);

-- 创建索引
CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);
CREATE INDEX IF NOT EXISTS idx_users_username ON users(username);
CREATE INDEX IF NOT EXISTS idx_users_status ON users(status);
CREATE INDEX IF NOT EXISTS idx_users_tier ON users(tier);

-- 用户会话表 (可选, 用于会话管理)
CREATE TABLE IF NOT EXISTS user_sessions (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,

    -- Token信息
    jti VARCHAR(255) NOT NULL UNIQUE,
    token_version INTEGER NOT NULL,

    -- 会话信息
    ip_address INET,
    user_agent TEXT,

    -- 状态
    is_active BOOLEAN NOT NULL DEFAULT true,

    -- 时间戳
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL,
    revoked_at TIMESTAMPTZ,
);

CREATE INDEX IF NOT EXISTS idx_sessions_user_id ON user_sessions(user_id);
CREATE INDEX IF NOT EXISTS idx_sessions_jti ON user_sessions(jti);
CREATE INDEX IF NOT EXISTS idx_sessions_active ON user_sessions(is_active, expires_at);

-- API密钥表 (用于长生命周期Token)
CREATE TABLE IF NOT EXISTS api_keys (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,

    -- API密钥信息
    key_id VARCHAR(100) NOT NULL UNIQUE,
    key_hash VARCHAR(255) NOT NULL,

    -- 权限
    scopes TEXT[] NOT NULL,
    tier VARCHAR(50) NOT NULL,

    -- 状态
    is_active BOOLEAN NOT NULL DEFAULT true,

    -- 元数据
    name VARCHAR(255) NOT NULL,
    last_used_at TIMESTAMPTZ,

    -- 时间戳
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ,
);

CREATE INDEX IF NOT EXISTS idx_api_keys_user_id ON api_keys(user_id);
CREATE INDEX IF NOT EXISTS idx_api_keys_key_id ON api_keys(key_id);
CREATE INDEX IF NOT EXISTS idx_api_keys_active ON api_keys(is_active);

-- 权限变更日志表
CREATE TABLE IF NOT EXISTS auth_audit_log (
    id BIGSERIAL PRIMARY KEY,

    -- 用户信息
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,

    -- 操作类型
    action VARCHAR(100) NOT NULL,
    resource_type VARCHAR(100),
    resource_id BIGINT,

    -- 详情
    details JSONB,

    -- IP和User Agent
    ip_address INET,
    user_agent TEXT,

    -- 时间戳
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
);

CREATE INDEX IF NOT EXISTS idx_audit_log_user_id ON auth_audit_log(user_id);
CREATE INDEX IF NOT EXISTS idx_audit_log_action ON auth_audit_log(action);
CREATE INDEX IF NOT EXISTS idx_audit_log_created ON auth_audit_log(created_at);

-- 插入测试用户 (开发环境)
-- 密码: password123 (实际应该使用bcrypt等哈希)
INSERT INTO users (username, email, password_hash, tier, scopes, balance, daily_points) VALUES
    ('test_user', 'test@example.com', '$2b$12$placeholder_hash', 'Pro', ARRAY['read', 'write'], 5000, 1850),
    ('admin_user', 'admin@example.com', '$2b$12$placeholder_hash', 'Enterprise', ARRAY['read', 'write', 'admin'], 999999, 999999)
ON CONFLICT (username) DO NOTHING;

-- 创建更新时间戳触发器函数
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- 为users表创建触发器
CREATE TRIGGER update_users_updated_at
    BEFORE UPDATE ON users
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- 视图: 活跃用户统计
CREATE OR REPLACE VIEW active_users_stats AS
SELECT
    tier,
    COUNT(*) as user_count,
    SUM(balance) as total_balance,
    AVG(daily_points) as avg_daily_points
FROM users
WHERE status = 'active'
GROUP BY tier;

-- 注释
COMMENT ON TABLE users IS '用户表';
COMMENT ON TABLE user_sessions IS '用户会话表';
COMMENT ON TABLE api_keys IS 'API密钥表';
COMMENT ON TABLE auth_audit_log IS '认证审计日志表';

COMMENT ON COLUMN users.token_version IS 'Token版本号, 用于Token撤销';
COMMENT ON COLUMN users.tier IS '用户等级: Base/Pro/Max/AMax/Enterprise';
COMMENT ON COLUMN users.scopes IS '权限范围数组';
COMMENT ON COLUMN user_sessions.jti IS 'JWT ID, 用于Token追踪';
COMMENT ON COLUMN api_keys.key_hash IS 'API密钥的哈希值, 不存储明文';
