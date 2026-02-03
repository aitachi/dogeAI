#!/usr/bin/env python3
"""初始化数据库"""
import sqlite3
from pathlib import Path

DB_PATH = "/var/lib/anthropic-proxy/stats.db"

def init_db():
    conn = sqlite3.connect(DB_PATH)
    cursor = conn.cursor()

    # 创建API密钥表
    cursor.execute('''
        CREATE TABLE IF NOT EXISTS api_keys (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            token TEXT UNIQUE NOT NULL,
            user TEXT NOT NULL,
            daily_limit INTEGER DEFAULT 1000000,
            expires_at TEXT,
            created_at TEXT DEFAULT CURRENT_TIMESTAMP,
            is_active BOOLEAN DEFAULT 1
        )
    ''')

    # 创建调用统计表
    cursor.execute('''
        CREATE TABLE IF NOT EXISTS call_stats (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            api_key_id INTEGER,
            model TEXT,
            input_tokens INTEGER DEFAULT 0,
            output_tokens INTEGER DEFAULT 0,
            total_tokens INTEGER DEFAULT 0,
            status TEXT DEFAULT 'success',
            error_message TEXT,
            created_at TEXT DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (api_key_id) REFERENCES api_keys (id)
        )
    ''')

    # 创建每日统计表
    cursor.execute('''
        CREATE TABLE IF NOT EXISTS daily_stats (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            api_key_id INTEGER,
            date TEXT,
            request_count INTEGER DEFAULT 0,
            total_tokens INTEGER DEFAULT 0,
            UNIQUE(api_key_id, date),
            FOREIGN KEY (api_key_id) REFERENCES api_keys (id)
        )
    ''')

    # 现有的API密钥
    EXISTING_KEYS = [
        {
            "token": "sk-dMUtFZ9IWedPSgSd0WMkH7XhP0idZVtAVnp48MYgNLo",
            "user": "a01",
            "daily_limit": 10000000,
            "expires_at": "2027-01-29T17:05:29"
        }
    ]

    for key in EXISTING_KEYS:
        try:
            cursor.execute('''
                INSERT INTO api_keys (token, user, daily_limit, expires_at)
                VALUES (?, ?, ?, ?)
            ''', (
                key['token'],
                key['user'],
                key['daily_limit'],
                key['expires_at']
            ))
            print(f"✅ 导入密钥: {key['user']}")
        except sqlite3.IntegrityError:
            print(f"⚠️  密钥已存在: {key['user']}")

    conn.commit()
    conn.close()
    print("✅ 数据库初始化完成!")

if __name__ == "__main__":
    init_db()
