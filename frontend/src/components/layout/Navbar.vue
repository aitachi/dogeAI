<template>
  <nav class="navbar">
    <div class="navbar-container">
      <div class="navbar-brand">
        <router-link to="/" class="brand-link">
          <span class="brand-icon">⚡</span>
          <div class="brand-text">
            <h1>狼狗AI</h1>
            <p>AI 模型 API 中转服务</p>
          </div>
        </router-link>
      </div>
      <div class="navbar-menu">
        <router-link to="/" class="nav-link">首页</router-link>
        <div class="nav-dropdown" @mouseenter="showDocsMenu = true" @mouseleave="showDocsMenu = false">
          <span class="nav-link">📖 文档 ▾</span>
          <div v-show="showDocsMenu" class="dropdown-menu">
            <router-link to="/docs/claude" class="dropdown-item">Claude 配置</router-link>
            <router-link to="/docs/claude?section=install" class="dropdown-item">安装教程</router-link>
            <router-link to="/docs/claude?section=ide" class="dropdown-item">IDE 配置</router-link>
            <router-link to="/docs/gpt" class="dropdown-item">GPT 配置</router-link>
          </div>
        </div>
        <router-link v-if="!isAuthenticated" to="/client" class="nav-link nav-link-primary">登录</router-link>
        <template v-if="isAuthenticated">
          <div class="user-info">
            <span class="user-name">{{ username }}</span>
            <span class="user-balance">{{ formatBalance(balance) }} 积分</span>
          </div>
          <button @click="logout" class="btn btn-outline btn-sm">退出</button>
        </template>
      </div>
    </div>
  </nav>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { useRouter } from 'vue-router'
import { useAuthStore } from '@/stores/auth'

const router = useRouter()
const authStore = useAuthStore()
const showDocsMenu = ref(false)

const isAuthenticated = computed(() => authStore.isAuthenticated)
const username = computed(() => authStore.username)
const balance = computed(() => authStore.balance)

function formatBalance(num: number): string {
  if (num >= 100000000) {
    return (num / 100000000).toFixed(1) + '亿'
  } else if (num >= 10000) {
    return (num / 10000).toFixed(1) + '万'
  }
  return num.toString()
}

function logout() {
  authStore.logout()
  router.push('/')
}
</script>

<style scoped>
.navbar {
  background: #2d3748;
  color: white;
  position: sticky;
  top: 0;
  z-index: 100;
  box-shadow: 0 2px 4px rgba(0,0,0,0.1);
}

.navbar-container {
  max-width: 1200px;
  margin: 0 auto;
  padding: 0 16px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: 60px;
}

.navbar-brand {
  display: flex;
  align-items: center;
}

.brand-link {
  display: flex;
  align-items: center;
  gap: 12px;
  text-decoration: none;
  color: white;
}

.brand-icon {
  font-size: 24px;
}

.brand-text h1 {
  font-size: 16px;
  font-weight: 600;
  margin: 0;
}

.brand-text p {
  font-size: 11px;
  opacity: 0.8;
  margin: 0;
}

.navbar-menu {
  display: flex;
  align-items: center;
  gap: 20px;
}

.nav-link {
  color: rgba(255,255,255,0.8);
  text-decoration: none;
  font-size: 14px;
  transition: color 0.2s;
}

.nav-link:hover {
  color: white;
}

.nav-link-primary {
  background: linear-gradient(135deg, #00d2ff 0%, #3a7bd5 100%);
  padding: 6px 16px;
  border-radius: 20px;
  color: white;
}

.user-info {
  display: flex;
  align-items: center;
  gap: 12px;
}

.user-name {
  font-size: 14px;
  font-weight: 500;
}

.user-balance {
  font-size: 12px;
  color: #10b981;
}

.btn-sm {
  padding: 4px 12px;
  font-size: 12px;
}

.nav-dropdown {
  position: relative;
  display: flex;
  align-items: center;
}

.nav-dropdown .nav-link {
  cursor: pointer;
}

.dropdown-menu {
  position: absolute;
  top: 100%;
  left: 0;
  margin-top: 8px;
  background: white;
  border-radius: 8px;
  box-shadow: 0 4px 12px rgba(0,0,0,0.15);
  min-width: 160px;
  overflow: hidden;
  z-index: 100;
}

.dropdown-item {
  display: block;
  padding: 10px 16px;
  color: #374151;
  text-decoration: none;
  font-size: 14px;
  transition: background 0.2s;
}

.dropdown-item:hover {
  background: #f3f4f6;
}

.dropdown-item:first-child {
  border-radius: 8px 8px 0 0;
}

.dropdown-item:last-child {
  border-radius: 0 0 8px 8px;
}
</style>
