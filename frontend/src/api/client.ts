import axios from 'axios'
import type { AxiosInstance, AxiosError, InternalAxiosRequestConfig } from 'axios'

const LOCAL_STORAGE_KEY = 'aitachi_api_key'  // 与后端保持一致

// 错误代码映射
const ERROR_MESSAGES: Record<string, string> = {
  'HTTP_401': '未授权，请重新登录',
  'HTTP_403': '没有权限执行此操作',
  'HTTP_404': '请求的资源不存在',
  'HTTP_500': '服务器错误，请稍后重试',
  'INVALID_TOKEN': '令牌无效或已过期',
  'FORBIDDEN': '权限不足',
  'USER_SUSPENDED': '账户已被暂停',
  'USER_NOT_FOUND': '用户不存在',
}

interface ApiError {
  code?: string
  message: string
  status?: number
}

class ApiClient {
  private client: AxiosInstance

  constructor() {
    this.client = axios.create({
      baseURL: '/api',
      timeout: 30000,
      headers: {
        'Content-Type': 'application/json'
      }
    })

    // 请求拦截器
    this.client.interceptors.request.use((config: InternalAxiosRequestConfig) => {
      const apiKey = localStorage.getItem(LOCAL_STORAGE_KEY)
      if (apiKey) {
        config.headers['x-api-key'] = apiKey
      }
      return config
    })

    // 响应拦截器 - 改进的错误处理
    this.client.interceptors.response.use(
      (response) => response.data,
      (error: AxiosError) => {
        const data = error.response?.data as ApiError | undefined
        const code = data?.code || `HTTP_${error.response?.status || 'UNKNOWN'}`
        const message = ERROR_MESSAGES[code] || data?.message || error.message || '请求失败'

        const apiError: ApiError = {
          code,
          message,
          status: error.response?.status
        }

        // 记录错误便于调试
        if (process.env.NODE_ENV === 'development') {
          console.error('[API Error]', apiError, error)
        }

        return Promise.reject({ ...data, ...apiError })
      }
    )
  }

  get(url: string, config?: any) {
    return this.client.get(url, config)
  }

  post(url: string, data?: any, config?: any) {
    return this.client.post(url, data, config)
  }

  put(url: string, data?: any, config?: any) {
    return this.client.put(url, data, config)
  }

  patch(url: string, data?: any, config?: any) {
    return this.client.patch(url, data, config)
  }

  delete(url: string, config?: any) {
    return this.client.delete(url, config)
  }
}

export const apiClient = new ApiClient()
export default apiClient
