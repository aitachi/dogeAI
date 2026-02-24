import { defineStore } from 'pinia'
import { ref, computed } from 'vue'

const ADMIN_LOCAL_STORAGE_KEY = 'aitachi_admin_token'

// 管理员凭证（硬编码）
const ADMIN_CREDENTIALS = {
  username: 'admin',
  password: 'ab123456'
}

export const useAdminAuthStore = defineStore('adminAuth', () => {
  const isAdmin = ref<boolean>(false)
  const adminToken = ref<string | null>(null)
  const isLoading = ref(false)

  // 初始化：从 localStorage 加载
  function init() {
    const storedToken = localStorage.getItem(ADMIN_LOCAL_STORAGE_KEY)
    if (storedToken) {
      adminToken.value = storedToken
      isAdmin.value = true
    }
  }

  // 管理员登录
  async function login(username: string, password: string) {
    isLoading.value = true

    // 模拟网络延迟
    await new Promise(resolve => setTimeout(resolve, 500))

    // 验证管理员凭证
    if (username === ADMIN_CREDENTIALS.username && password === ADMIN_CREDENTIALS.password) {
      const token = btoa(`${username}:${Date.now()}`)
      adminToken.value = token
      isAdmin.value = true
      localStorage.setItem(ADMIN_LOCAL_STORAGE_KEY, token)
      isLoading.value = false
      return { success: true }
    }

    isLoading.value = false
    return { success: false, message: '账号或密码错误' }
  }

  // 管理员登出
  function logout() {
    isAdmin.value = false
    adminToken.value = null
    localStorage.removeItem(ADMIN_LOCAL_STORAGE_KEY)
  }

  // 检查是否已登录
  const isAuthenticated = computed(() => isAdmin.value && !!adminToken.value)

  return {
    isAdmin,
    adminToken,
    isLoading,
    isAuthenticated,
    init,
    login,
    logout
  }
})
