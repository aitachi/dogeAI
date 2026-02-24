-- 添加对账表和更新计费记录表
-- 执行方式: psql -U postgres -d api_gateway -f migrations/002_add_reconciliation.sql

-- 更新计费记录表，添加billing_status字段
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'billing_records'
        AND column_name = 'billing_status'
    ) THEN
        ALTER TABLE billing_records ADD COLUMN billing_status VARCHAR(20) DEFAULT 'pending';
    END IF;
END $$;

-- 创建余额对账表
CREATE TABLE IF NOT EXISTS balance_reconciliation (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL,
    db_balance BIGINT NOT NULL,
    redis_balance BIGINT NOT NULL,
    difference BIGINT NOT NULL,
    reconciled_at TIMESTAMP DEFAULT NOW(),
    FOREIGN KEY (user_id) REFERENCES users(id)
);

CREATE INDEX IF NOT EXISTS idx_reconciliation_user ON balance_reconciliation(user_id);
CREATE INDEX IF NOT EXISTS idx_reconciliation_date ON balance_reconciliation(reconciled_at);

-- 创建充值码使用历史表
CREATE TABLE IF NOT EXISTS recharge_history (
    id BIGSERIAL PRIMARY KEY,
    code VARCHAR(20) NOT NULL,
    user_id BIGINT NOT NULL,
    points INT NOT NULL,
    package_type VARCHAR(50),
    used_at TIMESTAMP DEFAULT NOW(),
    FOREIGN KEY (user_id) REFERENCES users(id)
);

CREATE INDEX IF NOT EXISTS idx_recharge_history_user ON recharge_history(user_id);
CREATE INDEX IF NOT EXISTS idx_recharge_history_date ON recharge_history(used_at);

-- 更新充值码表，添加更多字段
DO $$
BEGIN
    -- 添加plan_type字段
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'recharge_codes'
        AND column_name = 'plan_type'
    ) THEN
        ALTER TABLE recharge_codes ADD COLUMN plan_type VARCHAR(50);
    END IF;

    -- 添加user_id字段（定向充值码）
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'recharge_codes'
        AND column_name = 'user_id'
    ) THEN
        ALTER TABLE recharge_codes ADD COLUMN user_id BIGINT;
    END IF;

    -- 添加batch_id字段
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'recharge_codes'
        AND column_name = 'batch_id'
    ) THEN
        ALTER TABLE recharge_codes ADD COLUMN batch_id VARCHAR(64);
    END IF;

    -- 添加used_at字段
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'recharge_codes'
        AND column_name = 'used_at'
    ) THEN
        ALTER TABLE recharge_codes ADD COLUMN used_at TIMESTAMP;
    END IF;

    -- 添加valid_days字段
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'recharge_codes'
        AND column_name = 'valid_days'
    ) THEN
        ALTER TABLE recharge_codes ADD COLUMN valid_days INT DEFAULT 30;
    END IF;

    -- 添加max_uses字段
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'recharge_codes'
        AND column_name = 'max_uses'
    ) THEN
        ALTER TABLE recharge_codes ADD COLUMN max_uses INT DEFAULT 1;
    END IF;
END $$;

COMMENT ON TABLE balance_reconciliation IS '余额对账记录表';
COMMENT ON TABLE recharge_history IS '充值码使用历史表';
