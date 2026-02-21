"""
流式SSE响应转换
OpenAI SSE格式 -> Anthropic SSE格式
"""

import json
import logging
import traceback
from typing import Optional

import httpx

from utils import gen_id
from converter import _map_finish

logger = logging.getLogger("proxy")


async def stream_sse(response: httpx.Response, original_model: str):
    """转换流式SSE响应"""

    msg_id = gen_id("msg")

    # 发送message_start事件
    ms = {
        "type": "message_start",
        "message": {
            "id": msg_id,
            "type": "message",
            "role": "assistant",
            "content": [],
            "model": original_model,
            "stop_reason": None,
            "stop_sequence": None,
            "usage": {
                "input_tokens": 0,
                "output_tokens": 0,
                "cache_creation_input_tokens": 0,
                "cache_read_input_tokens": 0,
            },
        },
    }
    yield f"event: message_start\ndata: {json.dumps(ms)}\n\n"
    yield 'event: ping\ndata: {"type": "ping"}\n\n'

    # 状态追踪
    text_open = False
    text_idx = -1
    next_idx = 0
    tool_blocks: Dict[int, int] = {}
    closed: set = set()
    finish_reason_received: Optional[str] = None
    input_tokens = 0
    output_tokens = 0
    chunk_count = 0

    # 处理SSE流
    async for raw_line in response.aiter_lines():
        line = raw_line.strip()
        if not line:
            continue
        if not line.startswith("data:"):
            continue

        # 提取payload
        if line.startswith("data: "):
            payload = line[6:]
        else:
            payload = line[5:]

        payload = payload.strip()
        if payload == "[DONE]":
            break

        try:
            chunk = json.loads(payload)
        except json.JSONDecodeError:
            continue

        chunk_count += 1
        if chunk_count <= 3:
            logger.debug(f"SSE<<< chunk#{chunk_count}: {json.dumps(chunk, ensure_ascii=False)[:300]}")

        # 处理错误
        if "error" in chunk and not chunk.get("choices"):
            err_msg = chunk["error"].get("message", "Unknown upstream error")
            logger.error(f"SSE<<< 上游流内错误: {err_msg}")
            if not text_open:
                text_idx = next_idx
                next_idx += 1
                bs = {
                    "type": "content_block_start",
                    "index": text_idx,
                    "content_block": {"type": "text", "text": ""},
                }
                yield f"event: content_block_start\ndata: {json.dumps(bs)}\n\n"
                text_open = True
            td = {
                "type": "content_block_delta",
                "index": text_idx,
                "delta": {"type": "text_delta", "text": f"[Proxy Error] {err_msg}"},
            }
            yield f"event: content_block_delta\ndata: {json.dumps(td)}\n\n"
            continue

        choices = chunk.get("choices", [])

        if not choices:
            u = chunk.get("usage")
            if u:
                input_tokens = u.get("prompt_tokens", input_tokens)
                output_tokens = u.get("completion_tokens", output_tokens)
            continue

        delta = choices[0].get("delta", {})
        finish_reason = choices[0].get("finish_reason")

        # 处理文本内容
        text_piece = delta.get("content")
        if not text_piece:
            text_piece = delta.get("reasoning_content")

        if text_piece:
            if not text_open:
                text_idx = next_idx
                next_idx += 1
                bs = {
                    "type": "content_block_start",
                    "index": text_idx,
                    "content_block": {"type": "text", "text": ""},
                }
                yield f"event: content_block_start\ndata: {json.dumps(bs)}\n\n"
                text_open = True

            td = {
                "type": "content_block_delta",
                "index": text_idx,
                "delta": {"type": "text_delta", "text": text_piece},
            }
            yield f"event: content_block_delta\ndata: {json.dumps(td)}\n\n"

        # 处理工具调用
        if delta.get("tool_calls"):
            for tc in delta["tool_calls"]:
                tc_i = tc.get("index", 0)

                if tc_i not in tool_blocks:
                    if text_open and text_idx not in closed:
                        yield f'event: content_block_stop\ndata: {{"type":"content_block_stop","index":{text_idx}}}\n\n'
                        closed.add(text_idx)
                        text_open = False

                    bi = next_idx
                    next_idx += 1
                    tool_id = tc.get("id", gen_id("toolu"))
                    tool_name = tc.get("function", {}).get("name", "")
                    tbs = {
                        "type": "content_block_start",
                        "index": bi,
                        "content_block": {
                            "type": "tool_use",
                            "id": tool_id,
                            "name": tool_name,
                            "input": {},
                        },
                    }
                    yield f"event: content_block_start\ndata: {json.dumps(tbs)}\n\n"
                    tool_blocks[tc_i] = bi

                arg_chunk = tc.get("function", {}).get("arguments", "")
                if arg_chunk:
                    bi = tool_blocks[tc_i]
                    ijd = {
                        "type": "content_block_delta",
                        "index": bi,
                        "delta": {
                            "type": "input_json_delta",
                            "partial_json": arg_chunk,
                        },
                    }
                    yield f"event: content_block_delta\ndata: {json.dumps(ijd)}\n\n"

        # 处理结束原因
        if finish_reason:
            finish_reason_received = finish_reason
            u = chunk.get("usage")
            if u:
                input_tokens = u.get("prompt_tokens", input_tokens)
                output_tokens = u.get("completion_tokens", output_tokens)

    # 关闭所有content block
    if next_idx == 0:
        bs = {
            "type": "content_block_start",
            "index": 0,
            "content_block": {"type": "text", "text": ""},
        }
        yield f"event: content_block_start\ndata: {json.dumps(bs)}\n\n"
        yield 'event: content_block_stop\ndata: {"type":"content_block_stop","index":0}\n\n'
    else:
        if text_open and text_idx not in closed:
            yield f'event: content_block_stop\ndata: {{"type":"content_block_stop","index":{text_idx}}}\n\n'
            closed.add(text_idx)
        for _, bi in sorted(tool_blocks.items()):
            if bi not in closed:
                yield f'event: content_block_stop\ndata: {{"type":"content_block_stop","index":{bi}}}\n\n'
                closed.add(bi)

    # 发送message_delta和message_stop
    stop_reason = _map_finish(finish_reason_received) if finish_reason_received else "end_turn"

    md = {
        "type": "message_delta",
        "delta": {"stop_reason": stop_reason, "stop_sequence": None},
        "usage": {"output_tokens": output_tokens},
    }
    yield f"event: message_delta\ndata: {json.dumps(md)}\n\n"
    yield 'event: message_stop\ndata: {"type": "message_stop"}\n\n'
