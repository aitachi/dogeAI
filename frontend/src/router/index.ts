import { createRouter, createWebHistory } from 'vue-router'
import type { RouteRecordRaw } from 'vue-router'
import { useAuthStore } from '@/stores/auth'
import { useAdminAuthStore } from '@/stores/adminAuth'

// 布局组件
import DefaultLayout from '@/components/layout/DefaultLayout.vue'
import AdminLayout from '@/components/layout/AdminLayout.vue'

// 页面组件
import Home from '@/views/Home.vue'
import Client from '@/views/Client.vue'

const routes: RouteRecordRaw[] = [
  {
    path: '/',
    component: DefaultLayout,
    children: [
      // 首页
      {
        path: '',
        name: 'Home',
        component: Home
      },
      // 客户端
      {
        path: 'client',
        name: 'Client',
        component: Client
      },
      // 兼容旧路径
      {
        path: 'client.html',
        redirect: '/client'
      },
      // 文档
      {
        path: 'docs/claude',
        name: 'ClaudeDocs',
        component: () => import('@/views/Docs/Claude.vue')
      },
      {
        path: 'docs/gpt',
        name: 'GPTDocs',
        component: () => import('@/views/Docs/GPT.vue')
      },
      // 兼容旧路径
      {
        path: 'claudecode.html',
        redirect: '/docs/claude'
      },
      {
        path: 'gptcode.html',
        redirect: '/docs/gpt'
      },
      // 管理后台登录（不使用布局）
      {
        path: 'admin/login',
        name: 'AdminLogin',
        component: () => import('@/views/Admin/Login.vue'),
        meta: { guestAdmin: true }
      },
      // 兼容旧管理路径
      {
        path: 'admin.html',
        redirect: '/admin/dashboard'
      }
    ]
  },
  // 管理后台使用独立布局
  {
    path: '/admin',
    component: AdminLayout,
    meta: { requiresAdmin: true },
    children: [
      {
        path: '',
        redirect: '/admin/dashboard'
      },
      {
        path: 'dashboard',
        name: 'AdminDashboard',
        component: () => import('@/views/Admin/Dashboard.vue')
      },
      {
        path: 'users',
        name: 'AdminUsers',
        component: () => import('@/views/Admin/Users.vue')
      },
      {
        path: 'recharge-cards',
        name: 'AdminRechargeCards',
        component: () => import('@/views/Admin/RechargeCards.vue')
      }
    ]
  },
  {
    path: '/:pathMatch(.*)*',
    redirect: '/'
  }
]

const router = createRouter({
  history: createWebHistory('/new/'),
  routes
})

// 路由守卫
router.beforeEach((to, from, next) => {
  const authStore = useAuthStore()
  const adminAuthStore = useAdminAuthStore()

  // 管理后台需要管理员权限
  if (to.meta.requiresAdmin) {
    if (!adminAuthStore.isAuthenticated) {
      // 未登录，跳转到管理员登录页
      next({ name: 'AdminLogin', query: { redirect: to.fullPath } })
      return
    }
  }

  // 管理员登录页 - 如果已登录则跳转到后台
  if (to.meta.guestAdmin) {
    if (adminAuthStore.isAuthenticated) {
      next({ name: 'AdminDashboard' })
      return
    }
  }

  // 客户端页面需要登录
  if (to.name === 'Client') {
    if (!authStore.isAuthenticated) {
      // 允许访问登录/注册表单
      next()
    } else {
      next()
    }
  } else {
    next()
  }
})

export default router
