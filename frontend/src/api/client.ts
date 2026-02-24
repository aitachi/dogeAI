import axios from 'axios'
import type { AxiosInstance, AxiosError } from 'axios'

const LOCAL_STORAGE_KEY = 'aitachi_api_key'  // 与后端保持一致

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
    this.client.interceptors.request.use((config) => {
      const apiKey = localStorage.getItem(LOCAL_STORAGE_KEY)
      if (apiKey) {
        config.headers['x-api-key'] = apiKey
      }
      return config
    })

    // 响应拦截器
    this.client.interceptors.response.use(
      (response) => response.data,
      (error: AxiosError) => {
        const message = error.response?.data?.message || error.message || '请求失败'
        return Promise.reject({ message, ...error.response?.data })
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

  delete(url: string, config?: any) {
    return this.client.delete(url, config)
  }
}

export const apiClient = new ApiClient()
export default apiClient
