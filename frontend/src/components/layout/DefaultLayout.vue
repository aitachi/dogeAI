<template>
  <div class="default-layout">
    <Navbar />
    <main class="main-content">
      <router-view />
    </main>
    <footer class="footer">
      <div class="footer-container">
        <p>&copy; 2026 狼狗AI. All rights reserved.</p>
        <div class="footer-links">
          <a href="/">首页</a>
          <a href="/docs/claude">Claude 配置</a>
          <a href="/client">登录</a>
        </div>
      </div>
    </footer>
  </div>
</template>

<script setup lang="ts">
import Navbar from '@/components/layout/Navbar.vue'
</script>

<style scoped>
.default-layout {
  min-height: 100vh;
  display: flex;
  flex-direction: column;
}

.main-content {
  flex: 1;
}

.footer {
  background: #2d3748;
  color: white;
  padding: 20px 0;
}

.footer-container {
  max-width: 1200px;
  margin: 0 auto;
  padding: 0 16px;
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.footer p {
  font-size: 12px;
  opacity: 0.8;
  margin: 0;
}

.footer-links a {
  color: #667eea;
  text-decoration: none;
  margin-left: 16px;
  font-size: 12px;
}

.footer-links a:hover {
  color: white;
}
</style>
