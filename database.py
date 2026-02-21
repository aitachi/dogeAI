"""
数据库模块 - 存储模型调用记录
"""

import sqlite3
import json
import time
import threading
from datetime import datetime, timedelta
from typing import Optional, List, Dict, Any
from contextlib import contextmanager

from config import logger

# 数据库文件路径
DB_PATH = "/root/dogeAI/proxy_monitor.db"

# 线程锁
db_lock = threading.Lock()


@contextmanager
def get_db():
    """获取数据库连接上下文"""
    conn = sqlite3.connect(DB_PATH, timeout=30.0)
    conn.row_factory = sqlite3.Row
    try:
        yield conn
        conn.commit()
    except Exception:
        conn.rollback()
        raise
    finally:
        conn.close()


def init_db():
    """初始化数据库表"""
    with get_db() as conn:
        cursor = conn.cursor()

        # 模型调用记录表
        cursor.execute("""
            CREATE TABLE IF NOT EXISTS model_calls (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                request_id TEXT NOT NULL,
                model_name TEXT NOT NULL,
                original_model TEXT NOT NULL,
                stream BOOLEAN NOT NULL,
                input_tokens INTEGER DEFAULT 0,
                output_tokens INTEGER DEFAULT 0,
                status TEXT NOT NULL,
                error_message TEXT,
                start_time REAL NOT NULL,
                end_time REAL,
                duration_ms INTEGER,
                first_token_time REAL,
                ttft_ms INTEGER,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )
        """)

        # 添加首token时间字段（如果表已存在）
        try:
            cursor.execute("ALTER TABLE model_calls ADD COLUMN first_token_time REAL")
        except Exception:
            pass
        try:
            cursor.execute("ALTER TABLE model_calls ADD COLUMN ttft_ms INTEGER")
        except Exception:
            pass

        # 模型统计表
        cursor.execute("""
            CREATE TABLE IF NOT EXISTS model_stats (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                model_name TEXT NOT NULL UNIQUE,
                total_calls INTEGER DEFAULT 0,
                successful_calls INTEGER DEFAULT 0,
                failed_calls INTEGER DEFAULT 0,
                total_input_tokens INTEGER DEFAULT 0,
                total_output_tokens INTEGER DEFAULT 0,
                total_duration_ms INTEGER DEFAULT 0,
                last_call_time TIMESTAMP,
                updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )
        """)

        # 创建索引
        cursor.execute("""
            CREATE INDEX IF NOT EXISTS idx_model_calls_model
            ON model_calls(model_name)
        """)
        cursor.execute("""
            CREATE INDEX IF NOT EXISTS idx_model_calls_start_time
            ON model_calls(start_time)
        """)
        cursor.execute("""
            CREATE INDEX IF NOT EXISTS idx_model_calls_status
            ON model_calls(status)
        """)

        conn.commit()
        logger.info("数据库初始化完成")


def record_call_start(
    request_id: str,
    model_name: str,
    original_model: str,
    stream: bool
) -> int:
    """记录调用开始"""
    with db_lock:
        try:
            with get_db() as conn:
                cursor = conn.cursor()
                cursor.execute("""
                    INSERT INTO model_calls (
                        request_id, model_name, original_model, stream,
                        status, start_time
                    ) VALUES (?, ?, ?, ?, 'running', ?)
                """, (request_id, model_name, original_model, stream, time.time()))
                return cursor.lastrowid
        except Exception as e:
            logger.error(f"记录调用开始失败: {e}")
            return 0


def record_call_end(
    call_id: int,
    status: str,
    input_tokens: int = 0,
    output_tokens: int = 0,
    error_message: Optional[str] = None,
    first_token_time: Optional[float] = None
):
    """记录调用结束"""
    if call_id == 0:
        return

    with db_lock:
        try:
            end_time = time.time()

            with get_db() as conn:
                cursor = conn.cursor()

                # 获取开始时间
                cursor.execute(
                    "SELECT start_time FROM model_calls WHERE id = ?",
                    (call_id,)
                )
                row = cursor.fetchone()
                if not row:
                    return

                start_time = row["start_time"]
                duration_ms = int((end_time - start_time) * 1000)

                # 计算首token响应时间
                ttft_ms = None
                if first_token_time:
                    ttft_ms = int((first_token_time - start_time) * 1000)

                # 更新调用记录
                cursor.execute("""
                    UPDATE model_calls SET
                        status = ?, input_tokens = ?, output_tokens = ?,
                        error_message = ?, end_time = ?, duration_ms = ?,
                        first_token_time = ?, ttft_ms = ?
                    WHERE id = ?
                """, (
                    status, input_tokens, output_tokens,
                    error_message, end_time, duration_ms,
                    first_token_time, ttft_ms, call_id
                ))

                # 获取模型名称用于更新统计
                cursor.execute(
                    "SELECT model_name FROM model_calls WHERE id = ?",
                    (call_id,)
                )
                row = cursor.fetchone()
                if not row:
                    return

                model_name = row["model_name"]

                # 更新或插入模型统计
                cursor.execute("""
                    INSERT INTO model_stats (
                        model_name, total_calls, successful_calls, failed_calls,
                        total_input_tokens, total_output_tokens, total_duration_ms,
                        last_call_time
                    ) VALUES (?, 1, ?, ?, ?, ?, ?, ?)
                    ON CONFLICT(model_name) DO UPDATE SET
                        total_calls = total_calls + 1,
                        successful_calls = successful_calls + ?,
                        failed_calls = failed_calls + ?,
                        total_input_tokens = total_input_tokens + ?,
                        total_output_tokens = total_output_tokens + ?,
                        total_duration_ms = total_duration_ms + ?,
                        last_call_time = ?,
                        updated_at = CURRENT_TIMESTAMP
                """, (
                    model_name,
                    1 if status == "success" else 0,
                    0 if status == "success" else 1,
                    input_tokens, output_tokens, duration_ms,
                    datetime.now().isoformat(),
                    1 if status == "success" else 0,
                    0 if status == "success" else 1,
                    input_tokens, output_tokens, duration_ms,
                    datetime.now().isoformat()
                ))

        except Exception as e:
            logger.error(f"记录调用结束失败: {e}")


def get_metrics() -> Dict[str, Any]:
    """获取监控指标"""
    try:
        # 从配置中获取所有可用模型
        from config import MAX_OUTPUT_TOKENS_LIMIT

        # 获取所有配置的模型名称
        all_models = list(MAX_OUTPUT_TOKENS_LIMIT.keys())

        with get_db() as conn:
            cursor = conn.cursor()

            # 获取总体统计
            cursor.execute("""
                SELECT
                    COUNT(*) as total_calls,
                    COUNT(DISTINCT model_name) as online_models,
                    SUM(input_tokens) as total_input_tokens,
                    SUM(output_tokens) as total_output_tokens
                FROM model_calls
                WHERE start_time > ?
            """, (time.time() - 3600,))  # 最近1小时

            row = cursor.fetchone()
            total_calls = row["total_calls"] or 0
            total_input_tokens = row["total_input_tokens"] or 0
            total_output_tokens = row["total_output_tokens"] or 0

            # 计算速率 (tokens/sec)
            tokens_in_rate = round(total_input_tokens / 3600, 1) if total_input_tokens > 0 else 0
            tokens_out_rate = round(total_output_tokens / 3600, 1) if total_output_tokens > 0 else 0

            # 为所有配置的模型生成列表
            models = []
            for model_name in all_models:
                # 获取该模型的所有调用记录（用于计算中位数、最大、最小值）
                cursor.execute("""
                    SELECT
                        duration_ms, ttft_ms, status,
                        datetime('now', '-5 minutes') < last_call_time as is_online
                    FROM model_calls
                    WHERE model_name = ? AND status = 'success'
                """, (model_name,))

                calls = cursor.fetchall()
                durations = [c["duration_ms"] for c in calls if c["duration_ms"]]
                ttfts = [c["ttft_ms"] for c in calls if c["ttft_ms"]]

                # 计算统计数据
                calls_count = len(calls)
                avg_response = int(sum(durations) / len(durations)) if durations else 0
                min_response = min(durations) if durations else 0
                max_response = max(durations) if durations else 0

                # 计算中位数
                if durations:
                    sorted_durations = sorted(durations)
                    n = len(sorted_durations)
                    median_response = sorted_durations[n // 2] if n % 2 == 1 else (sorted_durations[n // 2 - 1] + sorted_durations[n // 2]) // 2
                else:
                    median_response = 0

                # TTFT统计
                avg_ttft = int(sum(ttfts) / len(ttfts)) if ttfts else 0
                min_ttft = min(ttfts) if ttfts else 0
                max_ttft = max(ttfts) if ttfts else 0

                # 是否在线（最近5分钟有调用）
                is_online = any(c["is_online"] for c in calls) if calls else False

                # 计算当前并发任务数
                cursor.execute("""
                    SELECT COUNT(*) as concurrent
                    FROM model_calls
                    WHERE model_name = ? AND status = 'running'
                """, (model_name,))
                concurrent_row = cursor.fetchone()
                concurrent = concurrent_row["concurrent"] if concurrent_row else 0

                # 获取tokens统计
                cursor.execute("""
                    SELECT
                        SUM(input_tokens) as tokens_in,
                        SUM(output_tokens) as tokens_out
                    FROM model_calls
                    WHERE model_name = ? AND status = 'success'
                """, (model_name,))
                token_row = cursor.fetchone()

                models.append({
                    "name": model_name,
                    "calls": calls_count,
                    "concurrent": concurrent,
                    "tokens_in": token_row["tokens_in"] or 0,
                    "tokens_out": token_row["tokens_out"] or 0,
                    "avg_response": avg_response,
                    "median_response": median_response,
                    "min_response": min_response,
                    "max_response": max_response,
                    "avg_ttft": avg_ttft,
                    "min_ttft": min_ttft,
                    "max_ttft": max_ttft,
                    "status": "online" if is_online else "idle"
                })

            # 按调用次数排序
            models.sort(key=lambda x: x["calls"], reverse=True)

            return {
                "totalCalls": total_calls,
                "onlineModels": len([m for m in models if m["status"] == "online"]),
                "tokensInRate": tokens_in_rate,
                "tokensOutRate": tokens_out_rate,
                "models": models,
                "lastUpdated": datetime.now().isoformat()
            }

    except Exception as e:
        logger.error(f"获取监控指标失败: {e}")
        # 返回空数据而不是模拟数据
        return {
            "totalCalls": 0,
            "onlineModels": 0,
            "tokensInRate": 0,
            "tokensOutRate": 0,
            "models": [],
            "lastUpdated": datetime.now().isoformat(),
            "error": str(e)
        }


def get_recent_calls(limit: int = 100) -> List[Dict[str, Any]]:
    """获取最近的调用记录"""
    try:
        with get_db() as conn:
            cursor = conn.cursor()
            cursor.execute("""
                SELECT
                    request_id, model_name, original_model, stream,
                    input_tokens, output_tokens, status, error_message,
                    start_time, end_time, duration_ms, created_at
                FROM model_calls
                ORDER BY start_time DESC
                LIMIT ?
            """, (limit,))

            calls = []
            for row in cursor.fetchall():
                calls.append({
                    "request_id": row["request_id"],
                    "model_name": row["model_name"],
                    "original_model": row["original_model"],
                    "stream": bool(row["stream"]),
                    "input_tokens": row["input_tokens"],
                    "output_tokens": row["output_tokens"],
                    "status": row["status"],
                    "error_message": row["error_message"],
                    "start_time": datetime.fromtimestamp(row["start_time"]).isoformat(),
                    "end_time": datetime.fromtimestamp(row["end_time"]).isoformat() if row["end_time"] else None,
                    "duration_ms": row["duration_ms"],
                    "created_at": row["created_at"]
                })

            return calls

    except Exception as e:
        logger.error(f"获取最近调用记录失败: {e}")
        return []


def cleanup_old_records(days: int = 7):
    """清理旧记录"""
    try:
        with get_db() as conn:
            cursor = conn.cursor()
            cutoff_time = time.time() - (days * 86400)

            # 删除旧的调用记录
            cursor.execute("""
                DELETE FROM model_calls
                WHERE start_time < ?
            """, (cutoff_time,))

            deleted = cursor.rowcount

            # 更新模型统计（重置计数器但保留模型记录）
            cursor.execute("""
                UPDATE model_stats SET
                    total_calls = 0,
                    successful_calls = 0,
                    failed_calls = 0,
                    total_input_tokens = 0,
                    total_output_tokens = 0,
                    total_duration_ms = 0
            """)

            conn.commit()
            logger.info(f"清理了 {deleted} 条旧记录")

    except Exception as e:
        logger.error(f"清理旧记录失败: {e}")


# 初始化数据库
init_db()
