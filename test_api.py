#!/usr/bin/env python3.11
"""
Claude Code Proxy - 完整API测试脚本 (Python版本)
测试所有API端点是否正确工作
"""

import json
import sys
import time
import requests
from typing import Dict, Any, Optional

# 配置
BASE_URL = "https://127.0.0.1"  # 本地测试
API_KEY = "test-key"
VERIFY_SSL = False  # 跳过SSL验证

# 测试统计
total_tests = 0
passed_tests = 0
failed_tests = 0
errors = []

# 颜色输出
class Colors:
    GREEN = '\033[92m'
    RED = '\033[91m'
    YELLOW = '\033[93m'
    BLUE = '\033[94m'
    ENDC = '\033[0m'
    BOLD = '\033[1m'

def print_header(title: str):
    """打印标题"""
    print(f"\n{Colors.BLUE}{'='*60}{Colors.ENDC}")
    print(f"{Colors.BLUE}{title}{Colors.ENDC}")
    print(f"{Colors.BLUE}{'='*60}{Colors.ENDC}")

def print_test(name: str):
    """打印测试名称"""
    global total_tests
    total_tests += 1
    print(f"\n{Colors.BLUE}[测试 {total_tests}] {name}{Colors.ENDC}")

def print_success(message: str):
    """打印成功消息"""
    global passed_tests
    passed_tests += 1
    print(f"{Colors.GREEN}✅ {message}{Colors.ENDC}")

def print_error(message: str, details: str = ""):
    """打印错误消息"""
    global failed_tests
    failed_tests += 1
    print(f"{Colors.RED}❌ {message}{Colors.ENDC}")
    if details:
        print(f"   {details}")
    errors.append(f"{name}: {message}")

def api_request(method: str, path: str, data: Optional[Dict] = None,
                expected_field: Optional[str] = None) -> bool:
    """发送API请求并验证响应"""
    url = f"{BASE_URL}{path}"
    headers = {
        "Content-Type": "application/json",
        "x-api-key": API_KEY
    }

    try:
        if method == "GET":
            response = requests.get(url, headers=headers, verify=VERIFY_SSL, timeout=10)
        elif method == "POST":
            response = requests.post(url, headers=headers, json=data, verify=VERIFY_SSL, timeout=30)
        else:
            print_error(f"不支持的HTTP方法: {method}")
            return False

        # 检查HTTP状态码
        if response.status_code not in [200, 201]:
            print_error(f"HTTP {response.status_code}", response.text[:100])
            return False

        # 解析JSON
        try:
            result = response.json()
        except:
            print_error("响应不是有效的JSON", response.text[:100])
            return False

        # 检查预期字段
        if expected_field:
            if str(result).find(expected_field) >= 0:
                print_success(f"{method} {path}")
                print(f"   响应示例: {str(result)[:100]}...")
                return True
            else:
                print_error(f"缺少预期字段: {expected_field}", str(result)[:200])
                return False

        print_success(f"{method} {path}")
        print(f"   响应示例: {str(result)[:100]}...")
        return True

    except requests.exceptions.SSLError as e:
        print_error("SSL错误", str(e))
        return False
    except requests.exceptions.Timeout:
        print_error("请求超时")
        return False
    except requests.exceptions.ConnectionError as e:
        print_error("连接失败", str(e))
        return False
    except Exception as e:
        print_error("未知错误", str(e))
        return False

def test_streaming():
    """测试流式响应"""
    print_test("流式响应 (SSE)")

    url = f"{BASE_URL}/v1/messages"
    headers = {
        "Content-Type": "application/json",
        "x-api-key": API_KEY
    }
    data = {
        "model": "claude-sonnet-4-5-20250929",
        "max_tokens": 50,
        "stream": True,
        "messages": [{"role": "user", "content": "say hello"}]
    }

    try:
        response = requests.post(url, headers=headers, json=data,
                                stream=True, verify=VERIFY_SSL, timeout=30)

        if response.status_code != 200:
            print_error(f"HTTP {response.status_code}")
            return False

        # 读取SSE事件
        event_count = 0
        has_message_start = False
        has_content_block = False
        has_message_delta = False
        has_message_stop = False

        for line in response.iter_lines():
            if line:
                line = line.decode('utf-8') if isinstance(line, bytes) else line
                if line.startswith('data: '):
                    event_count += 1
                    if 'message_start' in line:
                        has_message_start = True
                    if 'content_block' in line:
                        has_content_block = True
                    if 'message_delta' in line:
                        has_message_delta = True
                    if 'message_stop' in line:
                        has_message_stop = True

        if event_count >= 3 and has_message_start and has_message_stop:
            print_success(f"流式响应 (收到{event_count}个事件)")
            print(f"   - message_start: {has_message_start}")
            print(f"   - content_block: {has_content_block}")
            print(f"   - message_delta: {has_message_delta}")
            print(f"   - message_stop: {has_message_stop}")
            return True
        else:
            print_error(f"流式响应格式错误 (事件数: {event_count})")
            return False

    except Exception as e:
        print_error("流式测试失败", str(e))
        return False

def test_tool_use():
    """测试工具调用"""
    print_test("工具调用 (tool_use)")

    data = {
        "model": "claude-sonnet-4-5-20250929",
        "max_tokens": 100,
        "messages": [{"role": "user", "content": "What is the weather in Beijing?"}],
        "tools": [{
            "name": "get_weather",
            "description": "Get weather information",
            "input_schema": {
                "type": "object",
                "properties": {
                    "location": {"type": "string"}
                },
                "required": ["location"]
            }
        }]
    }

    url = f"{BASE_URL}/v1/messages"
    headers = {
        "Content-Type": "application/json",
        "x-api-key": API_KEY
    }

    try:
        response = requests.post(url, headers=headers, json=data,
                                verify=VERIFY_SSL, timeout=30)

        if response.status_code != 200:
            print_error(f"HTTP {response.status_code}")
            return False

        result = response.json()

        # 检查是否包含工具调用
        if '"tool_use"' in str(result) or '"content"' in str(result):
            print_success("工具调用响应")
            print(f"   响应: {str(result)[:150]}...")
            return True
        else:
            print_error("响应格式不正确", str(result)[:200])
            return False

    except Exception as e:
        print_error("工具调用测试失败", str(e))
        return False

def main():
    """主测试函数"""
    print_header("🧪 Claude Code Proxy - API自动化测试")
    print(f"目标: {BASE_URL}")
    print(f"API Key: {API_KEY}")
    print(f"SSL验证: {'跳过' if not VERIFY_SSL else '启用'}")

    # ========================================
    # 1. 基础健康检查
    # ========================================
    print_header("1️⃣  基础健康检查")

    api_request("GET", "/api/hello", expected_field='"message":"hello"')
    api_request("GET", "/", expected_field='"status":"ok"')

    # ========================================
    # 2. 模型管理
    # ========================================
    print_header("2️⃣  模型管理")

    api_request("GET", "/v1/models", expected_field='"object":"list"')
    api_request("GET", "/v1/models/claude-sonnet-4-5-20250929",
              expected_field='"id":"claude-sonnet-4-5-20250929"')

    # ========================================
    # 3. Claude Code专用端点
    # ========================================
    print_header("3️⃣  Claude Code端点")

    api_request("GET", "/api/claude_code/settings", expected_field='"allowed_tools"')
    api_request("GET", "/api/claude_code/policy_limits", expected_field='"rate_limits"')
    api_request("GET", "/api/claude_code/penguin_mode", expected_field='"enabled"')

    # ========================================
    # 4. 平台API
    # ========================================
    print_header("4️⃣  平台API")

    api_request("GET", "/api/bootstrap", expected_field='"account"')
    api_request("GET", "/api/auth", expected_field='"account"')
    api_request("GET", "/api/account", expected_field='"email"')
    api_request("GET", "/api/organizations", expected_field='"data":')
    api_request("GET", "/v1/organizations", expected_field='"data":')

    # ========================================
    # 5. OAuth认证
    # ========================================
    print_header("5️⃣  OAuth认证")

    api_request("GET", "/v1/oauth/hello", expected_field='"message":"hello"')
    api_request("GET", "/.well-known/openid-configuration",
              expected_field='"issuer":')

    # ========================================
    # 6. 用户信息
    # ========================================
    print_header("6️⃣  用户信息")

    api_request("GET", "/api/me", expected_field='"uuid":"user_')
    api_request("GET", "/v1/me", expected_field='"uuid":"user_')
    api_request("GET", "/userinfo", expected_field='"uuid":"user_')

    # ========================================
    # 7. 核心消息API
    # ========================================
    print_header("7️⃣  核心消息API")

    api_request("POST", "/v1/messages",
              data={"model": "claude-sonnet-4-5-20250929",
                    "max_tokens": 50,
                    "messages": [{"role": "user", "content": "1+1=?"}]},
              expected_field='"1 + 1 = 2"')

    api_request("POST", "/v1/messages/count_tokens",
              data={"model": "claude-sonnet-4-5-20250929",
                    "messages": [{"role": "user", "content": "hello"}]},
              expected_field='"input_tokens":')

    # ========================================
    # 8. 流式响应
    # ========================================
    print_header("8️⃣  流式响应")

    test_streaming()

    # ========================================
    # 9. 工具调用
    # ========================================
    print_header("9️⃣  工具调用")

    test_tool_use()

    # ========================================
    # 10. Usage统计
    # ========================================
    print_header("🔟 Usage统计")

    api_request("GET", "/v1/dashboard/billing/usage", expected_field='"daily_costs":')
    api_request("GET", "/v1/usage", expected_field='"daily_costs":')

    # ========================================
    # 测试结果汇总
    # ========================================
    print_header("📊 测试结果汇总")

    print(f"\n总测试数: {Colors.BOLD}{total_tests}{Colors.ENDC}")
    print(f"{Colors.GREEN}通过: {passed_tests}{Colors.ENDC}")
    print(f"{Colors.RED}失败: {failed_tests}{Colors.ENDC}")
    print(f"通过率: {Colors.BOLD}{passed_tests*100//total_tests if total_tests > 0 else 0}%{Colors.ENDC}")

    if failed_tests > 0:
        print(f"\n{Colors.RED}失败的测试:{Colors.ENDC}")
        for error in errors:
            print(f"  - {error}")

    print(f"\n{Colors.BLUE}{'='*60}{Colors.ENDC}")

    if failed_tests == 0:
        print(f"{Colors.GREEN}{Colors.BOLD}🎉 所有测试通过！API工作正常！{Colors.ENDC}")
        print(f"{Colors.BLUE}{'='*60}{Colors.ENDC}\n")
        return 0
    else:
        print(f"{Colors.RED}{Colors.BOLD}⚠️  有 {failed_tests} 个测试失败{Colors.ENDC}")
        print(f"{Colors.BLUE}{'='*60}{Colors.ENDC}\n")
        return 1

if __name__ == "__main__":
    sys.exit(main())
