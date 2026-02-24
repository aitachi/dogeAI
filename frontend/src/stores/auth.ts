import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { User } from '@/api/types'
import { userApi } from '@/api/user'

const LOCAL_STORAGE_KEY = 'aitachi_api_key'  // 与后端保持一致

export const useAuthStore = defineStore('auth', () => {
  const user = ref<User | null>(null)
  const apiKey = ref<string | null>(null)
  const isLoading = ref(false)

  const isAuthenticated = computed(() => !!user.value && !!apiKey.value)
  const username = computed(() => user.value?.username || '')
  const balance = computed(() => user.value?.balance || 0)
  const tier = computed(() => user.value?.tier || 'base')

  // 初始化：从 localStorage 加载
  function init() {
    const storedKey = localStorage.getItem(LOCAL_STORAGE_KEY)
    if (storedKey) {
      apiKey.value = storedKey
      // 自动登录获取用户信息
      fetchUserInfo()
    }
  }

  // 设置认证信息
  function setAuth(u: User, key: string) {
    user.value = u
    apiKey.value = key
    localStorage.setItem(LOCAL_STORAGE_KEY, key)
  }

  // 清除认证信息
  function clearAuth() {
    user.value = null
    apiKey.value = null
    localStorage.removeItem(LOCAL_STORAGE_KEY)
  }

  // 登录
  async function login(account: string, password: string) {
    isLoading.value = true
    try {
      const response = await userApi.login({ account, password })
      if (response.success) {
        setAuth(
          {
            user_id: response.data.user_id,
            username: response.data.username,
            email: response.data.email,
            tier: response.data.tier,
            balance: response.data.balance
          },
          response.data.api_key
        )
        return { success: true }
      }
      return { success: false, message: response.message || '登录失败' }
    } catch (error: any) {
      return { success: false, message: error.message || '登录失败' }
    } finally {
      isLoading.value = false
    }
  }

  // 注册
  async function register(username: string, email: string, password: string) {
    isLoading.value = true
    try {
      const response = await userApi.register({ username, email, password })
      if (response.success) {
        setAuth(
          {
            user_id: response.data.user_id,
            username: response.data.username,
            email: response.data.email,
            tier: response.data.tier,
            balance: response.data.balance
          },
          response.data.api_key
        )
        return { success: true }
      }
      return { success: false, message: response.message || '注册失败' }
    } catch (error: any) {
      return { success: false, message: error.message || '注册失败' }
    } finally {
      isLoading.value = false
    }
  }

  // 获取用户信息
  async function fetchUserInfo() {
    if (!apiKey.value) return

    try {
      const response = await userApi.getBalance()
      // 更新余额
      if (user.value) {
        user.value.balance = response.balance
      }
    } catch (error) {
      // Token 可能失效，清除认证
      clearAuth()
    }
  }

  // 刷新余额
  async function refreshBalance() {
    if (!apiKey.value) return

    try {
      const response = await userApi.getBalance()
      if (user.value) {
        user.value.balance = response.balance
      }
      return response.balance
    } catch (error) {
      throw error
    }
  }

  // 登出
  function logout() {
    clearAuth()
  }

  return {
    user,
    apiKey,
    isLoading,
    isAuthenticated,
    username,
    balance,
    tier,
    init,
    setAuth,
    clearAuth,
    login,
    register,
    fetchUserInfo,
    refreshBalance,
    logout
  }
})
