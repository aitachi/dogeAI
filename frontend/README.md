# DogeAI Frontend (前端应用)

基于 Vue 3 + TypeScript 的现代化前端应用，提供用户仪表板和管理后台。

## 技术栈

- **Vue 3.4** - 渐进式 JavaScript 框架
- **TypeScript 5.3** - 类型安全
- **Vite 5.0** - 快速构建工具
- **Pinia 2.1** - 状态管理
- **Vue Router 4.2** - 路由管理
- **Axios 1.6** - HTTP 客户端

## 项目结构

```
frontend/
├── src/
│   ├── api/                 # API 封装
│   │   ├── client.ts        # Axios 客户端配置
│   │   ├── recharge.ts      # 充值 API
│   │   ├── types.ts         # TypeScript 类型定义
│   │   └── user.ts          # 用户 API
│   ├── components/          # 组件
│   │   ├── auth/            # 认证相关组件
│   │   ├── common/          # 通用组件
│   │   ├── layout/          # 布局组件
│   │   └── user/            # 用户相关组件
│   ├── composables/         # 组合式函数
│   ├── router/              # 路由配置
│   │   └── index.ts
│   ├── stores/              # Pinia 状态管理
│   │   ├── adminAuth.ts     # 管理员认证状态
│   │   └── auth.ts          # 用户认证状态
│   ├── styles/              # 全局样式
│   │   └── main.css
│   ├── utils/               # 工具函数
│   ├── views/               # 页面视图
│   │   ├── Admin/           # 管理后台
│   │   ├── Docs/            # 文档页面
│   │   ├── Client.vue       # 客户端页面
│   │   └── Home.vue         # 首页
│   ├── App.vue              # 根组件
│   └── main.ts              # 应用入口
├── index.html               # HTML 模板
├── vite.config.ts           # Vite 配置
├── tsconfig.json            # TypeScript 配置
├── tsconfig.node.json       # Node TypeScript 配置
└── package.json             # 依赖配置
```

## 快速开始

### 1. 安装依赖

```bash
npm install
```

### 2. 开发模式

```bash
npm run dev
```

访问 `http://localhost:5173`

### 3. 构建生产版本

```bash
npm run build
```

构建产物输出到 `dist/` 目录。

### 4. 预览生产构建

```bash
npm run preview
```

## 可用脚本

| 命令 | 说明 |
|------|------|
| `npm run dev` | 启动开发服务器 |
| `npm run build` | 构建生产版本 |
| `npm run build:check` | 类型检查 + 构建 |
| `npm run preview` | 预览生产构建 |
| `npm run lint` | 运行 ESLint |

## API 接口

前端连接以下后端 API：

- **认证**: `/api/user/login`, `/api/user/register`
- **用户**: `/api/user/balance`, `/api/user/profile`, `/api/user/history`
- **充值**: `/api/recharge/redeem`
- **统计**: `/api/stats`, `/api/health`
- **管理员**: `/api/admin/*`

## 页面说明

### 用户页面

- **首页** (`/`) - 项目介绍和快速开始
- **客户端** (`/client`) - 用户仪表板
- **文档** (`/docs/claude`, `/docs/gpt`) - API 文档

### 管理后台

- **登录** (`/admin/login`) - 管理员登录
- **仪表板** (`/admin/dashboard`) - 系统概览
- **用户管理** (`/admin/users`) - 用户列表和管理
- **充值卡管理** (`/admin/recharge-cards`) - 充值码管理

## 开发进度

- [x] 项目初始化
- [x] 通用组件 (Modal, Layout)
- [x] 路由配置
- [x] 状态管理 (Pinia)
- [x] API 客户端封装
- [x] 用户仪表板
- [x] 管理后台基础页面

## 配置

### API 地址

在 `src/api/client.ts` 中配置 API 基础地址：

```typescript
const apiClient = axios.create({
  baseURL: import.meta.env.VITE_API_URL || 'http://localhost:3000',
  // ...
});
```

### 环境变量

创建 `.env.local` 文件：

```
VITE_API_URL=http://localhost:3000
```

## 浏览器支持

- Chrome >= 87
- Firefox >= 78
- Safari >= 14
- Edge >= 88
