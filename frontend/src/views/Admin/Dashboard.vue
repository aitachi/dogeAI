<template>
  <div class="admin-dashboard">
    <div class="admin-header">
      <h1>管理控制台</h1>
      <div class="admin-actions">
        <button @click="refreshData" class="btn btn-outline">
          刷新数据
        </button>
      </div>
    </div>

    <!-- Stats Cards -->
    <div class="stats-grid">
      <div class="stat-card" v-for="stat in stats" :key="stat.label">
        <div class="stat-icon">{{ stat.icon }}</div>
        <div class="stat-value">{{ stat.value }}</div>
        <div class="stat-label">{{ stat.label }}</div>
        <div v-if="stat.trend" :class="['stat-trend', stat.trendClass]">
          {{ stat.trend }}
        </div>
      </div>
    </div>

    <!-- Quick Actions -->
    <div class="section">
      <h2>快捷操作</h2>
      <div class="actions-grid">
        <router-link to="/admin/users" class="action-card primary">
          <span class="action-icon">👤</span>
          <span class="action-label">用户管理</span>
        </router-link>
        <router-link to="/admin/recharge-cards" class="action-card success">
          <span class="action-icon">💳</span>
          <span class="action-label">充值卡管理</span>
        </router-link>
        <button @click="refreshData" class="action-card warning">
          <span class="action-icon">🔄</span>
          <span class="action-label">刷新数据</span>
        </button>
        <button @click="openReportModal" class="action-card info">
          <span class="action-icon">📊</span>
          <span class="action-label">统计报表</span>
        </button>
      </div>
    </div>

    <!-- Recent Activity -->
    <div class="section">
      <h2>最近活动</h2>
      <div class="activity-list">
        <div v-for="item in recentActivity" :key="item.id" class="activity-item">
          <div class="activity-icon">{{ item.icon }}</div>
          <div class="activity-content">
            <div class="activity-title">{{ item.title }}</div>
            <div class="activity-time">{{ item.time }}</div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import apiClient from '@/api/client'

const stats = ref([
  { icon: '👥', label: '总用户数', value: '-', trend: null, trendClass: '' },
  { icon: '✅', label: '活跃用户', value: '-', trend: null, trendClass: '' },
  { icon: '💰', label: '总余额', value: '-', trend: null, trendClass: '' },
  { icon: '📊', label: '今日请求', value: '-', trend: null, trendClass: '' },
  { icon: '💵', label: '今日消费', value: '-', trend: null, trendClass: '' },
  { icon: '🔑', label: 'API Keys', value: '-', trend: null, trendClass: '' }
])

const quickActions = [
  { icon: '👤', label: '用户管理', class: 'primary', handler: () => {} },
  { icon: '💳', label: '充值卡管理', class: 'success', handler: () => {} },
  { icon: '🔄', label: '刷新数据', class: 'warning', handler: () => {} },
  { icon: '📊', label: '统计报表', class: 'info', handler: () => {} }
]

function openReportModal() {
  alert('统计报表功能开发中...')
}

const recentActivity = ref([
  { id: 1, icon: '👤', title: '新用户注册: test_user', time: '2 分钟前' },
  { id: 2, icon: '💳', title: '充值卡使用: TEST1000', time: '5 分钟前' },
  { id: 3, icon: '🔄', title: 'Redis 缓存同步完成', time: '10 分钟前' }
])

async function refreshData() {
  try {
    const response = await apiClient.get('/admin/stats')
    if (response) {
      // Update stats with real data
      stats.value[0].value = response.total_users || '-'
      stats.value[1].value = response.active_users || '-'
      stats.value[5].value = response.api_keys || '-'
    }
  } catch (error) {
    console.error('Failed to fetch admin stats:', error)
  }
}

onMounted(() => {
  refreshData()
})
</script>

<style scoped>
.admin-dashboard {
  padding: 24px;
  max-width: 1400px;
  margin: 0 auto;
}

.admin-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 32px;
}

.admin-header h1 {
  font-size: 28px;
  font-weight: 700;
}

.stats-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
  gap: 20px;
  margin-bottom: 32px;
}

.stat-card {
  background: white;
  border-radius: 12px;
  padding: 24px;
  text-align: center;
  box-shadow: 0 1px 3px rgba(0,0,0,0.1);
  transition: transform 0.2s, box-shadow 0.2s;
}

.stat-card:hover {
  transform: translateY(-2px);
  box-shadow: 0 8px 16px rgba(0,0,0,0.1);
}

.stat-icon {
  font-size: 28px;
  margin-bottom: 12px;
}

.stat-value {
  font-size: 28px;
  font-weight: 700;
  color: #1f2937;
  margin-bottom: 4px;
}

.stat-label {
  font-size: 13px;
  color: #6b7280;
}

.stat-trend {
  font-size: 12px;
  margin-top: 8px;
}

.stat-trend.up {
  color: #10b981;
}

.stat-trend.down {
  color: #ef4444;
}

.section {
  background: white;
  border-radius: 12px;
  padding: 24px;
  margin-bottom: 24px;
  box-shadow: 0 1px 3px rgba(0,0,0,0.1);
}

.section h2 {
  font-size: 18px;
  font-weight: 600;
  margin-bottom: 20px;
}

.actions-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
  gap: 16px;
}

.action-card {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 12px;
  padding: 24px;
  border-radius: 12px;
  border: 2px solid transparent;
  cursor: pointer;
  transition: all 0.2s;
  background: #f9fafb;
  text-decoration: none;
  color: inherit;
}

.action-card:hover {
  background: #f3f4f6;
  transform: translateY(-2px);
}

.action-card.primary {
  background: #eff6ff;
  border-color: #3b82f6;
}

.action-card.success {
  background: #f0fdf4;
  border-color: #10b981;
}

.action-card.warning {
  background: #fffbeb;
  border-color: #f59e0b;
}

.action-card.info {
  background: #faf5ff;
  border-color: #a855f7;
}

.action-icon {
  font-size: 32px;
}

.action-label {
  font-size: 14px;
  font-weight: 500;
}

.activity-list {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.activity-item {
  display: flex;
  gap: 16px;
  padding: 16px;
  background: #f9fafb;
  border-radius: 8px;
}

.activity-icon {
  font-size: 24px;
}

.activity-content {
  flex: 1;
}

.activity-title {
  font-weight: 500;
  margin-bottom: 4px;
}

.activity-time {
  font-size: 12px;
  color: #6b7280;
}

.btn {
  padding: 8px 16px;
  border-radius: 6px;
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
  border: 1px solid #e5e7eb;
  background: white;
  transition: all 0.2s;
}

.btn-outline:hover {
  background: #f3f4f6;
}
</style>
