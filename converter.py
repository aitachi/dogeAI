"""
请求和响应格式转换
Anthropic格式 <-> OpenAI格式
"""

import json
import logging
from typing import Optional, Union, List, Dict, Any

from config import DEFAULT_PARAMS, logger
from utils import gen_id, map_model, clamp_max_tokens

logger = logging.getLogger("proxy")


# ============================================================
# 文本提取工具
# ============================================================
def _extract_text(content) -> str:
    """从content中提取文本"""
    if isinstance(content, str):
        return content
    if isinstance(content, list):
        parts = []
        for b in content:
            if isinstance(b, str):
                parts.append(b)
            elif isinstance(b, dict) and b.get("type") == "text":
                parts.append(b.get("text", ""))
        return "\n".join(parts)
    return str(content) if content else ""


# ============================================================
# 消息格式转换 Anthropic -> OpenAI
# ============================================================
def convert_messages(messages: list, system=None) -> List[Dict[str, Any]]:
    """转换消息格式"""
    result: List[Dict[str, Any]] = []

    if system:
        sys_text = _extract_text(system)
        if sys_text:
            result.append({"role": "system", "content": sys_text})

    for msg in messages:
        role = msg.get("role", "user")
        content = msg.get("content") or ""

        if role == "user":
            if isinstance(content, list):
                text_parts: List[str] = []
                tool_results: List[Dict[str, Any]] = []

                for block in content:
                    if not isinstance(block, dict):
                        text_parts.append(str(block))
                        continue
                    btype = block.get("type", "")

                    if btype == "text":
                        text_parts.append(block.get("text", ""))
                    elif btype == "tool_result":
                        tc_content = block.get("content", "")
                        if isinstance(tc_content, list):
                            tc_content = "\n".join(
                                b.get("text", "")
                                for b in tc_content
                                if isinstance(b, dict) and b.get("type") == "text"
                            )
                        elif not isinstance(tc_content, str):
                            tc_content = (
                                json.dumps(tc_content, ensure_ascii=False)
                                if tc_content
                                else ""
                            )
                        if block.get("is_error"):
                            tc_content = f"[ERROR] {tc_content}"

                        tool_results.append({
                            "role": "tool",
                            "tool_call_id": block.get("tool_use_id", ""),
                            "content": tc_content,
                        })

                for tr in tool_results:
                    result.append(tr)
                combined = "\n".join(text_parts).strip()
                if combined:
                    result.append({"role": "user", "content": combined})
                if not tool_results and not combined:
                    result.append({"role": "user", "content": ""})
            else:
                result.append({"role": "user", "content": str(content)})

        elif role == "assistant":
            if isinstance(content, list):
                text_parts = []
                tool_calls = []
                for block in content:
                    if not isinstance(block, dict):
                        text_parts.append(str(block))
                        continue
                    btype = block.get("type", "")
                    if btype == "text":
                        text_parts.append(block.get("text", ""))
                    elif btype == "tool_use":
                        tool_calls.append({
                            "id": block.get("id", gen_id("call")),
                            "type": "function",
                            "function": {
                                "name": block.get("name", ""),
                                "arguments": json.dumps(
                                    block.get("input", {}), ensure_ascii=False
                                ),
                            },
                        })
                out: Dict[str, Any] = {"role": "assistant"}
                out["content"] = "\n".join(text_parts) if text_parts else None
                if tool_calls:
                    out["tool_calls"] = tool_calls
                result.append(out)
            else:
                result.append({"role": "assistant", "content": str(content)})

    return result


def convert_tools(tools: Optional[List]) -> Optional[List]:
    """转换工具格式"""
    if not tools:
        return None
    out = []
    for t in tools:
        out.append({
            "type": "function",
            "function": {
                "name": t.get("name", ""),
                "description": t.get("description", ""),
                "parameters": t.get(
                    "input_schema", {"type": "object", "properties": {}}
                ),
            },
        })
    return out


def convert_tool_choice(tc) -> Optional[Union[str, Dict]]:
    """转换工具选择参数"""
    if not tc or not isinstance(tc, dict):
        return None
    t = tc.get("type", "auto")
    if t == "auto":
        return "auto"
    if t == "any":
        return "required"
    if t == "tool":
        return {"type": "function", "function": {"name": tc.get("name", "")}}
    return "auto"


def build_openai_request(body: dict) -> dict:
    """构建OpenAI格式的请求"""
    model = map_model(body.get("model", ""))
    messages = convert_messages(body.get("messages", []), body.get("system"))

    raw_max_tokens = body.get("max_tokens")
    clamped_max_tokens = clamp_max_tokens(raw_max_tokens, model)

    req: Dict[str, Any] = {
        "model": model,
        "messages": messages,
        "stream": body.get("stream", False),
        "max_tokens": clamped_max_tokens,
    }

    temp = body.get("temperature")
    if temp is not None:
        req["temperature"] = temp
    elif "temperature" in DEFAULT_PARAMS:
        req["temperature"] = DEFAULT_PARAMS["temperature"]

    if "top_p" in body:
        req["top_p"] = body["top_p"]

    tools = convert_tools(body.get("tools"))
    if tools:
        req["tools"] = tools
        tc = convert_tool_choice(body.get("tool_choice"))
        if tc:
            req["tool_choice"] = tc

    if body.get("stop_sequences"):
        req["stop"] = body["stop_sequences"]

    if req["stream"]:
        req["stream_options"] = {"include_usage": True}

    return req


# ============================================================
# 响应格式转换 OpenAI -> Anthropic
# ============================================================
def _map_finish(fr: Optional[str]) -> str:
    """映射结束原因"""
    if fr in ("tool_calls", "function_call"):
        return "tool_use"
    if fr == "length":
        return "max_tokens"
    return "end_turn"


def build_anthropic_response(openai_resp: dict, original_model: str) -> dict:
    """构建Anthropic格式的响应"""
    if "error" in openai_resp:
        return {
            "type": "error",
            "error": {
                "type": "api_error",
                "message": openai_resp["error"].get("message", "Unknown error"),
            },
        }
    choices = openai_resp.get("choices", [])
    if not choices:
        return {
            "type": "error",
            "error": {"type": "api_error", "message": "No choices in response"},
        }

    message = choices[0].get("message", {})
    content: List[Dict[str, Any]] = []

    if message.get("content"):
        content.append({"type": "text", "text": message["content"]})

    if message.get("tool_calls"):
        for tc in message["tool_calls"]:
            try:
                args = json.loads(tc["function"]["arguments"])
            except (json.JSONDecodeError, KeyError, TypeError):
                args = {}
            content.append({
                "type": "tool_use",
                "id": tc.get("id", gen_id("toolu")),
                "name": tc["function"]["name"],
                "input": args,
            })
    if not content:
        content.append({"type": "text", "text": ""})

    usage = openai_resp.get("usage", {})

    return {
        "id": gen_id("msg"),
        "type": "message",
        "role": "assistant",
        "content": content,
        "model": original_model,
        "stop_reason": _map_finish(choices[0].get("finish_reason")),
        "stop_sequence": None,
        "usage": {
            "input_tokens": usage.get("prompt_tokens", 0),
            "output_tokens": usage.get("completion_tokens", 0),
            "cache_creation_input_tokens": 0,
            "cache_read_input_tokens": 0,
        },
    }
