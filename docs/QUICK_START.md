#!/bin/bash
# Token中转平台 - 快速开始指南

cat << 'EOF'
╔════════════════════════════════════════════════════════════════════╗
║                                                                  ║
║           Token中转平台 - 快速开始指南                            ║
║                                                                  ║
╚════════════════════════════════════════════════════════════════════╝

📋 快速开始（5分钟部署）

─────────────────────────────────────────────────────────────────────
第一步：部署系统
─────────────────────────────────────────────────────────────────────

运行一键部署脚本：

    bash /root/deploy_token_gateway.sh

这将自动完成：
  ✅ 安装依赖
  ✅ 创建目录
  ✅ 初始化数据库
  ✅ 启动服务

─────────────────────────────────────────────────────────────────────
第二步：创建管理员Token
─────────────────────────────────────────────────────────────────────

    python3 /root/init_token_gateway.py --create-admin admin

输出示例：
    ✅ 管理员Token创建成功
       用户名: admin
       Token: sk-tk-aBcDeFgHiJkLnOpQrStUvWxYz123456789
       每日限额: 10,000,000 tokens

⚠️  重要：请保存好此Token！

─────────────────────────────────────────────────────────────────────
第三步：添加后端API Keys
─────────────────────────────────────────────────────────────────────

添加您的实际Anthropic API Keys：

    python3 /root/init_token_gateway.py --add-key \
        "primary-1" \
        "sk-ant-api03-xxxxx" \
        100 \
        10

参数说明：
  primary-1           → Key的名称（自定义）
  sk-ant-api03-xxxxx  → 实际的API Key
  100                 → 权重（越大分配越多请求）
  10                  → 优先级（越大优先级越高）

建议配置多个Keys：
    # 主Key（高优先级）
    python3 /root/init_token_gateway.py --add-key main-1 "sk-ant-xxx1" 100 10

    # 备用Key（中优先级）
    python3 /root/init_token_gateway.py --add-key backup-1 "sk-ant-xxx2" 100 5

    # 应急Key（低优先级）
    python3 /root/init_token_gateway.py --add-key emergency "sk-ant-xxx3" 50 1

─────────────────────────────────────────────────────────────────────
第四步：验证系统状态
─────────────────────────────────────────────────────────────────────

    python3 /root/token_gateway_manager.py --status

正常输出：
    📊 后端Keys状态 (共 3 个)
    ─────────────────────────────────────────────────────────────
    名称        健康度  权重   优先级  状态      平均响应
    ─────────────────────────────────────────────────────────────
    main-1      100    100    10      ✅ 健康    N/A
    backup-1    100    100    5       ✅ 健康    N/A
    emergency   100    50     1       ✅ 健康    N/A

─────────────────────────────────────────────────────────────────────
第五步：测试API
─────────────────────────────────────────────────────────────────────

使用curl测试：

    curl -X POST http://localhost:8000/v1/messages \
      -H "x-api-key: sk-tk-你的管理员Token" \
      -H "anthropic-version: 2023-06-01" \
      -H "content-type: application/json" \
      -d '{
        "model": "claude-3-haiku-20240307",
        "max_tokens": 100,
        "messages": [{"role": "user", "content": "Hello!"}]
      }'

或者使用Python：

    import httpx

    response = httpx.post(
        "http://localhost:8000/v1/messages",
        headers={
            "x-api-key": "sk-tk-你的管理员Token",
            "anthropic-version": "2023-06-01"
        },
        json={
            "model": "claude-3-haiku-20240307",
            "max_tokens": 100,
            "messages": [{"role": "user", "content": "Hello!"}]
        }
    )

    print(response.json())

─────────────────────────────────────────────────────────────────────
第六步：配置客户端（会话粘性）
─────────────────────────────────────────────────────────────────────

为了确保对话连贯性，客户端应该使用相同的session_id：

    import httpx

    # 第一次请求
    session_id = "my_conversation_001"

    response1 = httpx.post(
        "http://localhost:8000/v1/messages",
        headers={
            "x-api-key": "sk-tk-你的Token",
            "x-session-id": session_id  # 关键：携带session_id
        },
        json={
            "model": "claude-3-sonnet-20240229",
            "max_tokens": 100,
            "messages": [{"role": "user", "content": "What is AI?"}]
        }
    )

    # 第二次请求（同一会话）
    response2 = httpx.post(
        "http://localhost:8000/v1/messages",
        headers={
            "x-api-key": "sk-tk-你的Token",
            "x-session-id": session_id  # 相同的session_id
        },
        json={
            "model": "claude-3-sonnet-20240229",
            "max_tokens": 100,
            "messages": [
                {"role": "user", "content": "What is AI?"},
                {"role": "assistant", "content": response1.json()["content"]},
                {"role": "user", "content": "Tell me more"}
            ]
        }
    )

    # 系统会自动使用相同的后端Key，确保上下文连贯

─────────────────────────────────────────────────────────────────────
常用命令
─────────────────────────────────────────────────────────────────────

服务管理：
    systemctl status token-gateway     # 查看状态
    systemctl start token-gateway      # 启动服务
    systemctl stop token-gateway       # 停止服务
    systemctl restart token-gateway    # 重启服务
    journalctl -u token-gateway -f     # 查看日志

系统监控：
    python3 /root/token_gateway_manager.py --status    # 系统状态
    python3 /root/token_gateway_manager.py --top-keys  # Top Keys
    python3 /root/token_gateway_manager.py --errors    # 错误日志
    python3 /root/token_gateway_manager.py --watch     # 实时监控

Key管理：
    python3 /root/init_token_gateway.py --list-keys           # 列出Keys
    python3 /root/init_token_gateway.py --list-tokens         # 列出Tokens
    python3 /root/init_token_gateway.py --add-key ...         # 添加Key

API测试：
    python3 /root/test_gateway.py                            # 运行测试

─────────────────────────────────────────────────────────────────────
系统架构
─────────────────────────────────────────────────────────────────────

┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│  用户请求   │────▶│  API网关    │────▶│ Key池管理   │
│  (Token)    │     │  (FastAPI)  │     │  (负载均衡) │
└─────────────┘     └─────────────┘     └──────┬──────┘
                                               │
                          ┌────────────────────┼────────────────────┐
                          ▼                    ▼                    ▼
                    ┌─────────┐         ┌─────────┐         ┌─────────┐
                    │ API Key │         │ API Key │         │ API Key │
                    │   #1    │         │   #2    │         │   #3    │
                    └────┬────┘         └────┬────┘         └────┬────┘
                         │                   │                   │
                         └───────────────────┴───────────────────┘
                                               │
                                               ▼
                                    ┌──────────────────┐
                                    │ Anthropic API    │
                                    └──────────────────┘

核心特性：
  ✅ 会话粘性 - 确保对话连贯性
  ✅ 智能负载均衡 - 自动选择最优Key
  ✅ 健康检查 - 实时监控Key状态
  ✅ 故障切换 - 自动跳过不健康的Key
  ✅ 熔断机制 - 防止级联失败

─────────────────────────────────────────────────────────────────────
配置文件位置
─────────────────────────────────────────────────────────────────────

配置文件：     /root/token_gateway_config.yaml
数据库：       /var/lib/token-gateway/gateway.db
日志文件：     /var/log/token-gateway/gateway.log
服务文件：     /etc/systemd/system/token-gateway.service
主程序：       /root/token_gateway_v2.py
管理工具：     /root/token_gateway_manager.py
初始化工具：   /root/init_token_gateway.py

─────────────────────────────────────────────────────────────────────
下一步
─────────────────────────────────────────────────────────────────────

1. 阅读完整文档：cat /root/TOKEN_GATEWAY_README.md
2. 查看架构设计：cat /root/token_gateway_architecture.md
3. 配置监控告警
4. 添加更多后端Keys
5. 性能调优

─────────────────────────────────────────────────────────────────────
故障排查
─────────────────────────────────────────────────────────────────────

问题：服务无法启动
解决：
    journalctl -u token-gateway -n 50    # 查看错误日志
    systemctl status token-gateway        # 检查服务状态
    python3 /root/token_gateway_v2.py    # 手动运行测试

问题：Key不可用
解决：
    python3 /root/token_gateway_manager.py --diagnose  # 诊断问题
    curl http://localhost:8000/api/v1/keys              # 检查Keys

问题：响应慢
解决：
    python3 /root/token_gateway_manager.py --trends     # 查看趋势
    # 增加Key数量或调整权重

─────────────────────────────────────────────────────────────────────
技术支持
─────────────────────────────────────────────────────────────────────

详细文档：/root/TOKEN_GATEWAY_README.md
架构文档：/root/token_gateway_architecture.md

祝使用愉快！🎉

EOF
