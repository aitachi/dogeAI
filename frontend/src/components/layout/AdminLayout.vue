<template>
  <div class="admin-layout">
    <nav class="admin-navbar">
      <div class="navbar-container">
        <div class="navbar-brand">
          <router-link to="/admin/dashboard" class="brand-link">
            <span class="brand-icon">⚡</span>
            <div class="brand-text">
              <h1>狼狗AI - 管理后台</h1>
            </div>
          </router-link>
        </div>
        <div class="navbar-menu">
          <router-link to="/admin/dashboard" class="nav-link">控制台</router-link>
          <router-link to="/admin/users" class="nav-link">用户管理</router-link>
          <router-link to="/admin/recharge-cards" class="nav-link">充值卡</router-link>
          <div class="admin-info">
            <span class="admin-name">管理员</span>
            <button @click="handleLogout" class="btn btn-outline btn-sm">退出</button>
          </div>
        </div>
      </div>
    </nav>
    <main class="admin-main">
      <router-view />
    </main>
  </div>
</template>

<script setup lang="ts">
import { useRouter } from 'vue-router'
import { useAdminAuthStore } from '@/stores/adminAuth'

const router = useRouter()
const adminAuthStore = useAdminAuthStore()

function handleLogout() {
  if (confirm('确定要退出管理后台吗？')) {
    adminAuthStore.logout()
    router.push('/admin/login')
  }
}
</script>

<style scoped>
.admin-layout {
  min-height: 100vh;
  background: #f5f7fa;
}

.admin-navbar {
  background: #1e293b;
  color: white;
  box-shadow: 0 2px 4px rgba(0,0,0,0.1);
}

.navbar-container {
  max-width: 1400px;
  margin: 0 auto;
  padding: 0 24px;
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

.navbar-menu {
  display: flex;
  align-items: center;
  gap: 8px;
}

.nav-link {
  color: rgba(255,255,255,0.8);
  text-decoration: none;
  font-size: 14px;
  padding: 8px 16px;
  border-radius: 6px;
  transition: all 0.2s;
}

.nav-link:hover,
.nav-link.router-link-active {
  background: rgba(255,255,255,0.1);
  color: white;
}

.admin-info {
  display: flex;
  align-items: center;
  gap: 16px;
  padding-left: 16px;
  margin-left: 16px;
  border-left: 1px solid rgba(255,255,255,0.2);
}

.admin-name {
  font-size: 14px;
  font-weight: 500;
  color: #10b981;
}

.btn-sm {
  padding: 6px 14px;
  font-size: 12px;
}

.btn {
  padding: 8px 16px;
  border-radius: 6px;
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
  border: 1px solid rgba(255,255,255,0.3);
  background: transparent;
  color: white;
  transition: all 0.2s;
}

.btn-outline:hover {
  background: rgba(255,255,255,0.1);
  border-color: white;
}

.admin-main {
  padding: 0;
}
</style>
