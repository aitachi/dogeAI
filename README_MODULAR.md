# Claude Code Proxy - 模块化架构

## 文件结构

```
/root/dogeAI/
├── main.py              # 主入口 (99行)
├── config.py            # 配置管理 (47行)
├── utils.py             # 工具函数 (117行)
├── converter.py         # 请求/响应转换 (268行)
├── streamer.py          # 流式SSE处理 (218行)
├── ssl_helper.py        # SSL证书生成 (90行)
├── routes/              # 路由模块
│   ├── __init__.py      # 路由注册 (17行)
│   ├── base.py          # 基础路由 (100行)
│   ├── api.py           # 平台API (117行)
│   ├── oauth.py         # OAuth认证 (128行)
│   └── claude.py        # Claude Code端点+核心消息路由 (312行)
├── config.json          # 配置文件
├── proxy.py.backup      # 原始单文件版本备份
└── proxy.py             # 原始版本(已弃用)
```

## 模块说明

### 核心模块

- **config.py**: 加载配置文件，定义所有常量和全局配置
- **utils.py**: 通用工具函数（ID生成、模型映射、用户资料等）
- **converter.py**: Anthropic格式与OpenAI格式的相互转换
- **streamer.py**: 流式SSE响应格式转换

### 路由模块

- **routes/base.py**: 健康检查、模型列表、测试端点
- **routes/api.py**: 平台API端点（bootstrap、auth、account等）
- **routes/oauth.py**: OAuth认证流程端点
- **routes/claude.py**: Claude Code专用端点和核心消息处理

### 辅助模块

- **ssl_helper.py**: 自动生成和管理SSL证书

## 启动方式

### Python 3.11+ (推荐)
```bash
cd /root/dogeAI
python3.11 main.py
```

### Python 3.6
```bash
cd /root/dogeAI
python3 main.py
```

## 配置文件

config.json 包含以下配置：
- api_base: 目标API地址
- api_key: API密钥
- model_map: 模型名称映射
- http_port: HTTP端口
- https_port: HTTPS端口

## 与原版对比

### 优点
1. **代码组织清晰**: 每个文件职责单一，易于理解和维护
2. **模块化设计**: 可以独立修改和测试各个模块
3. **代码复用**: 工具函数集中管理，避免重复
4. **易于扩展**: 添加新功能只需修改对应模块
5. **团队协作**: 多人可以并行开发不同模块

### 文件大小对比

| 文件 | 行数 | 说明 |
|------|------|------|
| proxy.py (原版) | 1394行 | 单文件包含所有功能 |
| main.py | 99行 | 主入口和中间件 |
| routes/claude.py | 312行 | 最大的路由文件 |
| converter.py | 268行 | 格式转换逻辑 |
| streamer.py | 218行 | 流式处理 |
| 其他文件 | 100-150行 | 模块化拆分 |

## 测试结果

✅ 所有端点测试通过：
- /api/hello
- /v1/models
- /api/claude_code/settings
- /v1/oauth/hello
- /v1/messages (核心消息路由)

## 回滚方式

如需回滚到原版：
```bash
cp proxy.py.backup proxy.py
python3 proxy.py
```

## 部署建议

1. 使用 Python 3.11+ 运行以获得更好的性能
2. 使用 systemd 或 supervisord 管理进程
3. 配置 nginx 反向代理
4. 定期备份 config.json
