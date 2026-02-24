#!/bin/bash
set -e

DB_NAME="api_gateway"

echo "========================================"
echo "数据库迁移脚本"
echo "========================================"

# 检查数据库是否存在
echo "检查数据库..."
DB_EXISTS=$(sudo -u postgres psql -tAc "SELECT 1 FROM pg_database WHERE datname='$DB_NAME'")

if [ "$DB_EXISTS" != "1" ]; then
    echo "创建数据库: $DB_NAME"
    sudo -u postgres createdb $DB_NAME
else
    echo "数据库已存在: $DB_NAME"
fi

# 执行迁移
echo "执行数据库迁移..."

sudo -u postgres psql -d $DB_NAME << 'SQL'
-- 用户表
CREATE TABLE IF NOT EXISTS users (
    user_id VARCHAR(64) PRIMARY KEY,
    username VARCHAR(64) UNIQUE NOT NULL,
    api_token VARCHAR(128) UNIQUE,
    tier VARCHAR(32) NOT NULL DEFAULT 'base',
    balance BIGINT NOT NULL DEFAULT 0,
    token_version INTEGER NOT NULL DEFAULT 1,
    active BOOLEAN NOT NULL DEFAULT true,
    created_at BIGINT NOT NULL,
    updated_at BIGINT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_users_username ON users(username);
CREATE INDEX IF NOT EXISTS idx_users_api_token ON users(api_token);
CREATE INDEX IF NOT EXISTS idx_users_tier ON users(tier);

-- 请求日志表
CREATE TABLE IF NOT EXISTS request_logs (
    log_id BIGSERIAL PRIMARY KEY,
    user_id VARCHAR(64) NOT NULL,
    model VARCHAR(64) NOT NULL,
    prompt_tokens INTEGER NOT NULL,
    completion_tokens INTEGER NOT NULL,
    total_tokens INTEGER NOT NULL,
    cost BIGINT NOT NULL,
    status VARCHAR(32) NOT NULL,
    created_at BIGINT NOT NULL,
    error_message TEXT
);

CREATE INDEX IF NOT EXISTS idx_request_logs_user_id ON request_logs(user_id);
CREATE INDEX IF NOT EXISTS idx_request_logs_created_at ON request_logs(created_at);

-- 创建测试用户
INSERT INTO users (user_id, username, api_token, tier, balance, token_version, active, created_at, updated_at)
VALUES ('user_test_001', 'test_user', 'sk_test_1234567890abcdef', 'pro', 100000000, 1, true, EXTRACT(EPOCH FROM NOW())::BIGINT, EXTRACT(EPOCH FROM NOW())::BIGINT)
ON CONFLICT (username) DO UPDATE SET
    balance = EXCLUDED.balance,
    tier = EXCLUDED.tier,
    updated_at = EXCLUDED.updated_at;

SQL

echo ""
echo "迁移完成!"
echo ""
echo "用户列表:"
sudo -u postgres psql -d $DB_NAME -c "SELECT username, tier, balance FROM users;"
