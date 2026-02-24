<template>
  <div class="client-page">
    <!-- 认证视图 -->
    <div v-if="!isAuthenticated" class="auth-container">
      <div class="auth-card">
        <div class="auth-header">
          <h1>⚡ Aitachi</h1>
          <p>AI 模型计费平台</p>
        </div>

        <!-- Tabs -->
        <div class="auth-tabs">
          <button
            :class="['tab', { active: currentAuthTab === 'login' }]"
            @click="currentAuthTab = 'login'"
          >
            登录
          </button>
          <button
            :class="['tab', { active: currentAuthTab === 'register' }]"
            @click="currentAuthTab = 'register'"
          >
            注册
          </button>
        </div>

        <!-- Alert -->
        <div v-if="alert.show" :class="['alert', `alert-${alert.type}`]">
          <span>{{ alert.type === 'success' ? '✅' : alert.type === 'info' ? 'ℹ️' : '❌' }}</span>
          <span>{{ alert.message }}</span>
        </div>

        <!-- Login Form -->
        <form v-if="currentAuthTab === 'login'" @submit.prevent="handleLogin" class="auth-form">
          <div class="form-group">
            <label>用户名或邮箱</label>
            <input
              v-model="loginForm.account"
              type="text"
              class="input"
              placeholder="输入用户名或邮箱"
              required
            />
          </div>
          <div class="form-group">
            <label>密码</label>
            <input
              v-model="loginForm.password"
              type="password"
              class="input"
              placeholder="输入密码"
              required
            />
          </div>
          <button type="submit" class="btn btn-primary" :disabled="isLoading">
            {{ isLoading ? '登录中...' : '登录' }}
          </button>
          <p class="auth-hint">
            忘记密码？
            <a @click="currentAuthTab = 'register'">重新注册</a>
          </p>
        </form>

        <!-- Register Form -->
        <form v-else @submit.prevent="handleRegister" class="auth-form">
          <div class="form-group">
            <label>用户名</label>
            <input
              v-model="registerForm.username"
              type="text"
              class="input"
              placeholder="3-32个字符"
              required
            />
          </div>
          <div class="form-group">
            <label>邮箱</label>
            <input
              v-model="registerForm.email"
              type="email"
              class="input"
              placeholder="用于找回密码"
              required
            />
          </div>
          <div class="form-group">
            <label>密码</label>
            <input
              v-model="registerForm.password"
              type="password"
              class="input"
              placeholder="至少6个字符"
              required
            />
          </div>
          <div class="form-group">
            <label>确认密码</label>
            <input
              v-model="registerForm.confirmPassword"
              type="password"
              class="input"
              placeholder="再次输入密码"
              required
            />
          </div>
          <button type="submit" class="btn btn-primary" :disabled="isLoading">
            {{ isLoading ? '注册中...' : '注册' }}
          </button>
        </form>
      </div>
    </div>

    <!-- 主界面 -->
    <div v-else class="main-container">
      <!-- 导航栏 -->
      <nav class="navbar">
        <div class="navbar-brand">
          <span class="logo">⚡</span>
          <div>
            <h1>Aitachi</h1>
            <p>AI 模型计费平台</p>
          </div>
        </div>
        <div class="navbar-user">
          <div class="user-info">
            <div class="user-name">{{ username }}</div>
            <div class="user-tier">{{ tier }}</div>
          </div>
          <div class="balance-display">
            <span>{{ formatBalance(balance) }}</span> 积分
          </div>
          <button @click="logout" class="btn btn-outline">退出</button>
        </div>
      </nav>

      <!-- 统计卡片 -->
      <div class="stats-grid">
        <div class="stat-card">
          <div class="stat-icon">💰</div>
          <div class="stat-value">{{ formatBalance(balance) }}</div>
          <div class="stat-label">当前余额</div>
        </div>
        <div class="stat-card">
          <div class="stat-icon">📊</div>
          <div class="stat-value">{{ totalRequests }}</div>
          <div class="stat-label">总请求次数</div>
        </div>
        <div class="stat-card">
          <div class="stat-icon">🔥</div>
          <div class="stat-value">{{ formatNumber(totalTokens) }}</div>
          <div class="stat-label">总Token数</div>
        </div>
        <div class="stat-card">
          <div class="stat-icon">⚡</div>
          <div class="stat-value">{{ tierDisplay }}</div>
          <div class="stat-label">账户等级</div>
        </div>
      </div>

      <!-- 标签页 -->
      <div class="tabs">
        <button
          v-for="tab in tabs"
          :key="tab.key"
          :class="['tab', { active: currentTab === tab.key }]"
          @click="currentTab = tab.key"
        >
          {{ tab.label }}
        </button>
      </div>

      <!-- 概览页面 -->
      <div v-show="currentTab === 'overview'" class="tab-content">
        <div class="card">
          <div class="card-header">
            <h3 class="card-title">可用模型</h3>
          </div>
          <div class="model-grid">
            <div class="model-card">
              <div class="model-name">Opus 4.6</div>
              <div class="model-desc">最强模型 · 7分/次</div>
            </div>
            <div class="model-card">
              <div class="model-name">Sonnet 4.6</div>
              <div class="model-desc">平衡模型 · 4分/次</div>
            </div>
            <div class="model-card">
              <div class="model-name">Haiku</div>
              <div class="model-desc">快速模型 · 1分/次</div>
            </div>
          </div>
        </div>

        <div class="card">
          <div class="card-header">
            <h3 class="card-title">API Key</h3>
          </div>
          <div class="api-key-card">
            <div>
              <div class="api-key-label">您的 API Key</div>
              <div class="api-key-display">{{ maskedApiKey }}</div>
            </div>
            <button @click="copyApiKey" class="btn btn-outline">复制</button>
          </div>
          <p class="api-key-hint">在 API 请求中使用此 Key 进行身份认证：</p>
          <pre class="code-block"><code>export ANTHROPIC_API_KEY="{{ apiKey }}"
export ANTHROPIC_BASE_URL="https://115.190.62.87/v1"</code></pre>
        </div>
      </div>

      <!-- 充值页面 -->
      <div v-show="currentTab === 'recharge'" class="tab-content">
        <div class="card">
          <div class="card-header">
            <h3 class="card-title">充值码兑换</h3>
          </div>
          <div v-if="redeemAlert.show" :class="['alert', `alert-${redeemAlert.type}`]">
            <span>{{ redeemAlert.type === 'success' ? '✅' : '❌' }}</span>
            <span>{{ redeemAlert.message }}</span>
          </div>
          <div class="form-group">
            <label>输入充值码</label>
            <input
              v-model="redeemForm.code"
              type="text"
              class="input"
              placeholder="例如: S5ABCD1234"
              style="text-transform: uppercase;"
            />
          </div>
          <button @click="redeemCode" class="btn btn-primary" :disabled="isRedeeming">
            {{ isRedeeming ? '兑换中...' : '兑换充值码' }}
          </button>
        </div>
      </div>

      <!-- 使用记录页面 -->
      <div v-show="currentTab === 'history'" class="tab-content">
        <div class="card">
          <div class="card-header">
            <h3 class="card-title">计费记录</h3>
            <button @click="loadHistory" class="btn btn-outline" :disabled="isLoadingHistory">
              {{ isLoadingHistory ? '刷新中...' : '刷新' }}
            </button>
          </div>
          <div class="table-container">
            <table>
              <thead>
                <tr>
                  <th>时间</th>
                  <th>模型</th>
                  <th>输入Token</th>
                  <th>输出Token</th>
                  <th>总Token</th>
                  <th>消耗积分</th>
                </tr>
              </thead>
              <tbody>
                <tr v-if="historyRecords.length === 0">
                  <td colspan="6" class="text-center">暂无记录</td>
                </tr>
                <tr v-for="record in historyRecords" :key="record.created_at">
                  <td>{{ formatTimestamp(record.created_at) }}</td>
                  <td><span class="badge badge-info">{{ record.model }}</span></td>
                  <td>{{ formatNumber(record.input_tokens || 0) }}</td>
                  <td>{{ formatNumber(record.output_tokens || 0) }}</td>
                  <td>{{ formatNumber(record.total_tokens || 0) }}</td>
                  <td class="cost">{{ record.cost || 0 }}</td>
                </tr>
              </tbody>
            </table>
          </div>
        </div>
      </div>

      <!-- 设置页面 -->
      <div v-show="currentTab === 'settings'" class="tab-content">
        <div class="card">
          <div class="card-header">
            <h3 class="card-title">账户信息</h3>
          </div>
          <div v-if="profileAlert.show" :class="['alert', `alert-${profileAlert.type}`]">
            <span>{{ profileAlert.type === 'success' ? '✅' : '❌' }}</span>
            <span>{{ profileAlert.message }}</span>
          </div>
          <div class="form-group">
            <label>用户ID</label>
            <input type="text" class="input" :value="user?.user_id || ''" readonly />
          </div>
          <div class="form-group">
            <label>用户名</label>
            <input type="text" class="input" v-model="settingsForm.username" />
          </div>
          <div class="form-group">
            <label>邮箱</label>
            <input type="email" class="input" :value="user?.email || ''" readonly />
          </div>
          <div class="form-group">
            <label>账户等级</label>
            <input type="text" class="input" :value="tier" readonly />
          </div>
          <button @click="updateProfile" class="btn btn-primary" :disabled="isUpdating">
            {{ isUpdating ? '保存中...' : '保存资料' }}
          </button>
        </div>

        <div class="card">
          <div class="card-header">
            <h3 class="card-title">修改密码</h3>
          </div>
          <div v-if="passwordAlert.show" :class="['alert', `alert-${passwordAlert.type}`]">
            <span>{{ passwordAlert.type === 'success' ? '✅' : '❌' }}</span>
            <span>{{ passwordAlert.message }}</span>
          </div>
          <div class="form-group">
            <label>当前密码</label>
            <input type="password" class="input" v-model="passwordForm.oldPassword" />
          </div>
          <div class="form-group">
            <label>新密码</label>
            <input type="password" class="input" v-model="passwordForm.newPassword" />
          </div>
          <div class="form-group">
            <label>确认新密码</label>
            <input type="password" class="input" v-model="passwordForm.confirmPassword" />
          </div>
          <button @click="changePassword" class="btn btn-primary" :disabled="isChangingPassword">
            {{ isChangingPassword ? '修改中...' : '修改密码' }}
          </button>
        </div>

        <div class="card">
          <div class="card-header">
            <h3 class="card-title">计费规则</h3>
          </div>
          <table>
            <thead>
              <tr>
                <th>模型</th>
                <th>基础积分</th>
              </tr>
            </thead>
            <tbody>
              <tr>
                <td><span class="badge badge-primary">Opus 4.6</span></td>
                <td>7 分/次</td>
              </tr>
              <tr>
                <td><span class="badge badge-info">Sonnet 4.6</span></td>
                <td>4 分/次</td>
              </tr>
              <tr>
                <td><span class="badge badge-success">Haiku</span></td>
                <td>1 分/次</td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>
    </div>

    <!-- Toast 通知 -->
    <div v-if="toast.show" :class="['toast', `toast-${toast.type}`]">
      {{ toast.message }}
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import { useRouter } from 'vue-router'
import { useAuthStore } from '@/stores/auth'
import { userApi } from '@/api/user'
import { rechargeApi } from '@/api/recharge'
import type { HistoryRecord } from '@/api/types'

const router = useRouter()
const authStore = useAuthStore()

// 状态
const currentAuthTab = ref('login')
const currentTab = ref('overview')
const isLoading = ref(false)
const isRedeeming = ref(false)
const isLoadingHistory = ref(false)
const isUpdating = ref(false)
const isChangingPassword = ref(false)

// 提示消息
const alert = ref({ show: false, message: '', type: 'error' })
const redeemAlert = ref({ show: false, message: '', type: 'error' })
const passwordAlert = ref({ show: false, message: '', type: 'error' })
const profileAlert = ref({ show: false, message: '', type: 'error' })
const toast = ref({ show: false, message: '', type: 'success' })

// 表单
const loginForm = ref({ account: '', password: '' })
const registerForm = ref({ username: '', email: '', password: '', confirmPassword: '' })
const redeemForm = ref({ code: '' })
const settingsForm = ref({ username: '' })
const passwordForm = ref({ oldPassword: '', newPassword: '', confirmPassword: '' })

// 历史记录
const historyRecords = ref<HistoryRecord[]>([])
const totalRequests = ref(0)
const totalTokens = ref(0)

// 标签页定义
const tabs = [
  { key: 'overview', label: '概览' },
  { key: 'recharge', label: '充值' },
  { key: 'history', label: '使用记录' },
  { key: 'settings', label: '设置' }
]

// 计算属性
const isAuthenticated = computed(() => authStore.isAuthenticated)
const user = computed(() => authStore.user)
const username = computed(() => authStore.username)
const balance = computed(() => authStore.balance)
const tier = computed(() => authStore.tier)
const apiKey = computed(() => authStore.apiKey || '')

const tierDisplay = computed(() => {
  const tierMap: Record<string, string> = {
    'pro': '专业版',
    'base': '基础版',
    'free': '免费版'
  }
  return tierMap[tier.value] || '基础版'
})

const maskedApiKey = computed(() => {
  // 直接显示完整的 API Key，不隐藏
  if (!apiKey.value) return '未设置'
  return apiKey.value
})

// 工具函数
function formatBalance(num: number): string {
  if (num >= 100000000) {
    return (num / 100000000).toFixed(1) + '亿'
  } else if (num >= 10000) {
    return (num / 10000).toFixed(1) + '万'
  }
  return num.toLocaleString()
}

function formatNumber(num: number): string {
  if (num >= 10000) {
    return (num / 10000).toFixed(1) + '万'
  }
  return num.toLocaleString()
}

// 格式化 Unix 时间戳（后端返回秒级时间戳）
function formatTimestamp(timestamp: number): string {
  // 判断是秒级还是毫秒级
  const date = new Date(timestamp * 1000)
  if (date.getFullYear() < 2000) {
    // 如果年份太小，说明是毫秒级
    return new Date(timestamp).toLocaleString('zh-CN')
  }
  return date.toLocaleString('zh-CN')
}

// 显示 toast 通知
function showToast(message: string, type = 'success') {
  toast.value = { show: true, message, type }
  setTimeout(() => {
    toast.value.show = false
  }, 3000)
}

// 显示提示消息（用于表单内）
function showAlert(alertRef: typeof alert, message: string, type = 'error') {
  alertRef.value = { show: true, message, type }
  setTimeout(() => {
    alertRef.value.show = false
  }, 5000)
}

// 登录
async function handleLogin() {
  if (!loginForm.value.account || !loginForm.value.password) {
    showAlert(alert, '请输入用户名和密码')
    return
  }

  isLoading.value = true
  const result = await authStore.login(loginForm.value.account, loginForm.value.password)

  if (result.success) {
    showAlert(alert, '登录成功！', 'success')
    settingsForm.value.username = user.value?.username || ''
    await loadHistory()
  } else {
    showAlert(alert, result.message || '登录失败')
  }
  isLoading.value = false
}

// 注册
async function handleRegister() {
  if (registerForm.value.password.length < 6) {
    showAlert(alert, '密码至少需要6个字符')
    return
  }

  if (registerForm.value.password !== registerForm.value.confirmPassword) {
    showAlert(alert, '两次输入的密码不一致')
    return
  }

  isLoading.value = true
  const result = await authStore.register(
    registerForm.value.username,
    registerForm.value.email,
    registerForm.value.password
  )

  if (result.success) {
    showAlert(alert, '注册成功！', 'success')
    currentAuthTab.value = 'login'
    loginForm.value.account = registerForm.value.username
  } else {
    showAlert(alert, result.message || '注册失败')
  }
  isLoading.value = false
}

// 充值码兑换
async function redeemCode() {
  const code = redeemForm.value.code.trim().toUpperCase()

  if (!code) {
    showAlert(redeemAlert, '请输入充值码')
    return
  }

  isRedeeming.value = true
  try {
    const response = await rechargeApi.redeem({ code })
    if (response.success) {
      const amountMsg = response.amount ? `获得 ${response.amount} 积分` : ''
      showAlert(redeemAlert, `充值成功！${amountMsg}`, 'success')
      redeemForm.value.code = ''
      // 刷新余额
      await authStore.refreshBalance()
    } else {
      showAlert(redeemAlert, response.message || '充值失败')
    }
  } catch (error: any) {
    showAlert(redeemAlert, error.message || '请求失败')
  }
  isRedeeming.value = false
}

// 加载历史记录
async function loadHistory() {
  isLoadingHistory.value = true
  try {
    const response = await userApi.getHistory()
    historyRecords.value = response.records || []

    // 更新统计数据
    totalRequests.value = historyRecords.value.length
    totalTokens.value = historyRecords.value.reduce((sum, r) => sum + (r.total_tokens || 0), 0)
  } catch (error) {
    console.error('加载历史失败:', error)
  }
  isLoadingHistory.value = false
}

// 更新资料
async function updateProfile() {
  const username = settingsForm.value.username.trim()

  if (username.length < 3 || username.length > 32) {
    showAlert(profileAlert, '用户名长度必须为3-32个字符')
    return
  }

  // 验证用户名格式
  if (!/^[a-zA-Z0-9_-]+$/.test(username)) {
    showAlert(profileAlert, '用户名只能包含字母、数字、下划线和连字符')
    return
  }

  isUpdating.value = true
  try {
    const response = await userApi.updateProfile(username)
    if (response.success && user.value) {
      user.value.username = username
      showAlert(profileAlert, '资料更新成功', 'success')
    } else {
      showAlert(profileAlert, response.message || '更新失败')
    }
  } catch (error: any) {
    showAlert(profileAlert, error.message || '请求失败')
  }
  isUpdating.value = false
}

// 修改密码
async function changePassword() {
  const { oldPassword, newPassword, confirmPassword } = passwordForm.value

  if (!oldPassword || !newPassword) {
    showAlert(passwordAlert, '请输入当前密码和新密码')
    return
  }

  if (newPassword.length < 6) {
    showAlert(passwordAlert, '新密码长度至少为6个字符')
    return
  }

  if (newPassword !== confirmPassword) {
    showAlert(passwordAlert, '两次输入的新密码不一致')
    return
  }

  isChangingPassword.value = true
  try {
    const response = await userApi.changePassword({ old_password: oldPassword, new_password: newPassword })
    if (response.success) {
      showAlert(passwordAlert, '密码修改成功', 'success')
      passwordForm.value = { oldPassword: '', newPassword: '', confirmPassword: '' }
    } else {
      showAlert(passwordAlert, response.message || '修改失败')
    }
  } catch (error: any) {
    showAlert(passwordAlert, error.message || '请求失败')
  }
  isChangingPassword.value = false
}

// 复制 API Key
function copyApiKey() {
  if (apiKey.value) {
    navigator.clipboard.writeText(apiKey.value).then(() => {
      showToast('API Key 已复制！')
    }).catch(() => {
      showToast('复制失败，请手动复制', 'error')
    })
  }
}

// 登出
function logout() {
  authStore.logout()
  router.push('/')
}

// 监听用户变化，自动加载历史
watch(() => authStore.user, async (newUser) => {
  if (newUser && isAuthenticated.value) {
    settingsForm.value.username = newUser.username || ''
    await loadHistory()
  }
})

// 初始化
onMounted(() => {
  const urlParams = new URLSearchParams(window.location.search)
  if (urlParams.get('tab') === 'register') {
    currentAuthTab.value = 'register'
  }

  if (isAuthenticated.value && user.value) {
    settingsForm.value.username = user.value.username || ''
    loadHistory()
  }
})
</script>

<style scoped>
.client-page {
  min-height: 100vh;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
}

/* 认证视图 */
.auth-container {
  display: flex;
  align-items: center;
  justify-content: center;
  min-height: 100vh;
  padding: 20px;
}

.auth-card {
  background: white;
  border-radius: 24px;
  padding: 40px;
  width: 100%;
  max-width: 400px;
  box-shadow: 0 8px 40px rgba(0, 0, 0, 0.1);
}

.auth-header {
  text-align: center;
  margin-bottom: 32px;
}

.auth-header h1 {
  font-size: 28px;
  margin-bottom: 4px;
  background: linear-gradient(90deg, #667eea, #764ba2);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
}

.auth-header p {
  color: #888;
  font-size: 13px;
}

.auth-tabs {
  display: flex;
  margin-bottom: 24px;
  border-bottom: 1px solid #e0e0e0;
}

.auth-tabs .tab {
  flex: 1;
  padding: 12px;
  border: none;
  background: transparent;
  cursor: pointer;
  font-weight: 500;
  color: #888;
  border-bottom: 2px solid transparent;
}

.auth-tabs .tab.active {
  color: #667eea;
  border-bottom-color: #667eea;
}

.auth-form {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.form-group {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.form-group label {
  font-size: 14px;
  font-weight: 500;
  color: #555;
}

.auth-hint {
  text-align: center;
  font-size: 13px;
  color: #888;
}

.auth-hint a {
  color: #667eea;
  cursor: pointer;
}

/* 主界面 */
.main-container {
  max-width: 1200px;
  margin: 0 auto;
  padding: 20px;
}

/* 导航栏 */
.navbar {
  background: rgba(255, 255, 255, 0.95);
  backdrop-filter: blur(10px);
  border-radius: 16px;
  padding: 16px 24px;
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 24px;
  box-shadow: 0 4px 20px rgba(0, 0, 0, 0.1);
}

.navbar-brand {
  display: flex;
  align-items: center;
  gap: 12px;
}

.logo {
  font-size: 28px;
}

.navbar-brand h1 {
  font-size: 20px;
  font-weight: 700;
  margin: 0;
  background: linear-gradient(90deg, #667eea, #764ba2);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
}

.navbar-brand p {
  font-size: 12px;
  color: #888;
  margin: 0;
}

.navbar-user {
  display: flex;
  align-items: center;
  gap: 16px;
}

.user-info {
  text-align: right;
}

.user-name {
  font-weight: 600;
}

.user-tier {
  font-size: 12px;
  color: #888;
}

.balance-display {
  background: linear-gradient(135deg, #667eea, #764ba2);
  color: white;
  padding: 8px 16px;
  border-radius: 20px;
  font-weight: 600;
}

/* 统计卡片 */
.stats-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
  gap: 16px;
  margin-bottom: 24px;
}

.stat-card {
  background: white;
  border-radius: 12px;
  padding: 20px;
  text-align: center;
  box-shadow: 0 2px 10px rgba(0, 0, 0, 0.05);
}

.stat-icon {
  font-size: 24px;
  margin-bottom: 12px;
}

.stat-value {
  font-size: 28px;
  font-weight: 700;
  color: #333;
}

.stat-label {
  font-size: 14px;
  color: #888;
  margin-top: 4px;
}

/* 标签页 */
.tabs {
  display: flex;
  gap: 8px;
  margin-bottom: 20px;
  background: rgba(255, 255, 255, 0.5);
  padding: 6px;
  border-radius: 12px;
}

.tabs .tab {
  flex: 1;
  padding: 12px;
  border: none;
  background: transparent;
  border-radius: 8px;
  cursor: pointer;
  font-weight: 500;
  color: #888;
  transition: all 0.3s;
}

.tabs .tab.active {
  background: white;
  color: #667eea;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
}

/* 卡片 */
.card {
  background: rgba(255, 255, 255, 0.95);
  backdrop-filter: blur(10px);
  border-radius: 16px;
  padding: 24px;
  margin-bottom: 20px;
  box-shadow: 0 4px 20px rgba(0, 0, 0, 0.1);
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
}

.card-title {
  font-size: 18px;
  font-weight: 600;
  color: #333;
  margin: 0;
}

/* 模型卡片 */
.model-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
  gap: 12px;
}

.model-card {
  background: #f8f9fa;
  border: 2px solid transparent;
  border-radius: 12px;
  padding: 16px;
  text-align: center;
}

.model-name {
  font-weight: 600;
}

.model-desc {
  font-size: 12px;
  color: #888;
}

/* API Key 卡片 */
.api-key-card {
  background: #f8f9fa;
  border-radius: 12px;
  padding: 16px;
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.api-key-label {
  font-weight: 600;
  margin-bottom: 4px;
}

.api-key-display {
  font-family: 'Monaco', monospace;
  background: white;
  padding: 8px 16px;
  border-radius: 6px;
  font-size: 14px;
}

.api-key-hint {
  margin-top: 16px;
  color: #888;
  font-size: 14px;
}

.code-block {
  background: #1e1e1e;
  color: #d4d4d4;
  padding: 12px;
  border-radius: 8px;
  font-size: 12px;
  overflow-x: auto;
}

.code-block code {
  font-family: monospace;
}

/* 表格 */
.table-container {
  overflow-x: auto;
}

table {
  width: 100%;
  border-collapse: collapse;
}

th, td {
  padding: 12px;
  text-align: left;
  border-bottom: 1px solid #f0f0f0;
}

th {
  font-weight: 600;
  color: #888;
  font-size: 12px;
  text-transform: uppercase;
}

.text-center {
  text-align: center;
}

.cost {
  color: #667eea;
  font-weight: 600;
}

/* 徽章 */
.badge {
  display: inline-block;
  padding: 4px 12px;
  border-radius: 20px;
  font-size: 12px;
  font-weight: 600;
}

.badge-primary { background: #667eea; color: white; }
.badge-success { background: #00ff88; color: white; }
.badge-info { background: #00d2ff; color: white; }

/* 通用样式 */
.btn {
  padding: 12px 24px;
  border: none;
  border-radius: 8px;
  font-size: 14px;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.3s;
}

.btn-primary {
  background: linear-gradient(135deg, #667eea, #764ba2);
  color: white;
}

.btn-primary:hover:not(:disabled) {
  transform: translateY(-2px);
  box-shadow: 0 4px 12px rgba(102, 126, 234, 0.4);
}

.btn-primary:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.btn-outline {
  background: transparent;
  border: 2px solid #667eea;
  color: #667eea;
}

.btn-outline:hover:not(:disabled) {
  background: #667eea;
  color: white;
}

.input {
  width: 100%;
  padding: 12px 16px;
  border: 1px solid #e0e0e0;
  border-radius: 8px;
  font-size: 14px;
  transition: border-color 0.3s;
}

.input:focus {
  outline: none;
  border-color: #667eea;
}

.input:read-only {
  background: #f9fafb;
  color: #6b7280;
}

.alert {
  padding: 16px;
  border-radius: 12px;
  margin-bottom: 16px;
  display: flex;
  align-items: center;
  gap: 12px;
}

.alert-error {
  background: rgba(255, 68, 68, 0.1);
  color: #cc0000;
}

.alert-success {
  background: rgba(0, 255, 136, 0.1);
  color: #00a86b;
}

/* Toast 通知 */
.toast {
  position: fixed;
  top: 20px;
  left: 50%;
  transform: translateX(-50%);
  padding: 12px 24px;
  border-radius: 8px;
  color: white;
  font-weight: 500;
  z-index: 9999;
  animation: slideDown 0.3s ease-out;
}

.toast-success {
  background: #10b981;
}

.toast-error {
  background: #ef4444;
}

@keyframes slideDown {
  from {
    opacity: 0;
    transform: translate(-50%, -20px);
  }
  to {
    opacity: 1;
    transform: translate(-50%, 0);
  }
}

/* 响应式 */
@media (max-width: 768px) {
  .navbar {
    flex-direction: column;
    gap: 16px;
  }

  .stats-grid {
    grid-template-columns: repeat(2, 1fr);
  }

  .model-grid {
    grid-template-columns: 1fr;
  }

  .card-header {
    flex-direction: column;
    align-items: flex-start;
    gap: 12px;
  }
}
</style>
