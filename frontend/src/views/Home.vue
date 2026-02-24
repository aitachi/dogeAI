<template>
  <div class="home-page">
    <!-- Hero Section -->
    <section class="hero">
      <div class="hero-container">
        <h1 class="hero-title">⚡ 狼狗AI</h1>
        <p class="hero-subtitle">专业 AI API 中转服务 | 支持 Opus 4.6、Sonnet 4.6、Haiku</p>
        <div class="hero-actions">
          <router-link to="/client" class="btn btn-primary btn-large">
            🔐 客户端登录
          </router-link>
          <router-link to="/docs/claude" class="btn btn-outline btn-large">
            📖 安装教程
          </router-link>
        </div>
        <div class="hero-links">
          <router-link to="/docs/claude?section=install" class="hero-link">🐧 Linux 教程</router-link>
          <router-link to="/docs/claude?section=install" class="hero-link">🪟 Windows 教程</router-link>
          <router-link to="/docs/claude?section=install" class="hero-link">🍎 macOS 教程</router-link>
          <router-link to="/docs/claude?section=ide" class="hero-link">💻 IDE 配置</router-link>
        </div>
      </div>
    </section>

    <!-- Services Section -->
    <section class="section">
      <div class="container">
        <h2 class="section-title">核心服务</h2>
        <div class="grid">
          <div class="card" v-for="service in services" :key="service.title">
            <div class="card-icon">{{ service.icon }}</div>
            <h3>{{ service.title }}</h3>
            <p>{{ service.desc }}</p>
          </div>
        </div>
      </div>
    </section>

    <!-- Stats Section -->
    <section class="section section-gray">
      <div class="container">
        <h2 class="section-title">平台数据</h2>
        <div class="stats-grid">
          <div class="stat-item" v-for="stat in stats" :key="stat.label">
            <div class="stat-value">{{ stat.value }}</div>
            <div class="stat-label">{{ stat.label }}</div>
          </div>
        </div>
      </div>
    </section>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import axios from 'axios'

const services = [
  { icon: '🚀', title: 'API 代理服务', desc: '提供 Claude、GPT 等 AI 模型 API 中转，无需复杂配置' },
  { icon: '⚡', title: '高性能并发', desc: '支持 20+ 并发请求，智能队列管理，稳定响应' },
  { icon: '🔒', title: '安全可靠', desc: 'API Key 验证，完整日志统计，每日限额管理' },
  { icon: '📊', title: '实时统计', desc: '详细调用统计和 Token 使用记录' },
  { icon: '🌐', title: '多模型支持', desc: '支持 Opus 4.6、Sonnet 4.6、Haiku 模型' },
  { icon: '💼', title: '企业级方案', desc: '定制化解决方案，专属技术支持' }
]

const stats = ref([
  { label: '注册用户', value: '-' },
  { label: '活跃用户', value: '-' },
  { label: '累计请求', value: '-' }
])

onMounted(async () => {
  try {
    const response = await axios.get('/api/stats')
    stats.value[0].value = response.data.total_users
    stats.value[1].value = response.data.active_users
    stats.value[2].value = response.data.total_requests
  } catch (error) {
    console.error('Failed to fetch stats:', error)
  }
})
</script>

<style scoped>
.home-page {
  min-height: 100vh;
}

.hero {
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  color: white;
  padding: 80px 0;
  text-align: center;
}

.hero-container {
  max-width: 800px;
  margin: 0 auto;
  padding: 0 20px;
}

.hero-title {
  font-size: 42px;
  font-weight: 700;
  margin-bottom: 12px;
}

.hero-subtitle {
  font-size: 16px;
  opacity: 0.9;
  margin-bottom: 30px;
}

.hero-actions {
  display: flex;
  gap: 16px;
  justify-content: center;
}

.btn-large {
  padding: 12px 30px;
  font-size: 16px;
}

.hero-links {
  display: flex;
  gap: 16px;
  justify-content: center;
  margin-top: 24px;
  flex-wrap: wrap;
}

.hero-link {
  color: rgba(255, 255, 255, 0.85);
  text-decoration: none;
  font-size: 14px;
  padding: 8px 16px;
  background: rgba(255, 255, 255, 0.1);
  border-radius: 20px;
  transition: all 0.2s;
}

.hero-link:hover {
  background: rgba(255, 255, 255, 0.2);
  color: white;
}

.section {
  padding: 60px 0;
}

.section-gray {
  background: #f3f4f6;
}

.section-title {
  text-align: center;
  font-size: 28px;
  font-weight: 600;
  margin-bottom: 40px;
}

.grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
  gap: 20px;
}

.card {
  background: white;
  padding: 24px;
  border-radius: 8px;
  box-shadow: 0 1px 3px rgba(0,0,0,0.1);
  transition: transform 0.2s, box-shadow 0.2s;
}

.card:hover {
  transform: translateY(-2px);
  box-shadow: 0 4px 12px rgba(0,0,0,0.15);
}

.card-icon {
  font-size: 32px;
  margin-bottom: 12px;
}

.card h3 {
  font-size: 16px;
  font-weight: 600;
  margin-bottom: 8px;
}

.card p {
  font-size: 13px;
  color: #6b7280;
}

.stats-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
  gap: 24px;
}

.stat-item {
  background: white;
  padding: 24px;
  border-radius: 8px;
  text-align: center;
}

.stat-value {
  font-size: 32px;
  font-weight: 700;
  color: #667eea;
  margin-bottom: 8px;
}

.stat-label {
  font-size: 13px;
  color: #6b7280;
}
</style>
