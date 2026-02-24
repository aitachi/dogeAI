import apiClient from './client'
import type {
  LoginRequest,
  LoginResponse,
  RegisterRequest,
  BalanceResponse,
  HistoryResponse,
  ChangePasswordRequest,
  ChangePasswordResponse,
  ProfileRequest,
  ProfileResponse
} from './types'

export const userApi = {
  // 登录
  login: (data: LoginRequest) =>
    apiClient.post<LoginResponse>('/user/login', data),

  // 注册
  register: (data: RegisterRequest) =>
    apiClient.post<LoginResponse>('/user/register', data),

  // 获取余额
  getBalance: () =>
    apiClient.get<BalanceResponse>('/user/balance'),

  // 获取用户资料
  getProfile: (username?: string) =>
    apiClient.post<ProfileResponse>('/user/profile', username ? { username } : {}),

  // 更新用户资料
  updateProfile: (username: string) =>
    apiClient.post<ProfileResponse>('/user/profile', { username }),

  // 获取使用历史
  getHistory: () =>
    apiClient.get<HistoryResponse>('/user/history'),

  // 修改密码
  changePassword: (data: ChangePasswordRequest) =>
    apiClient.post<ChangePasswordResponse>('/user/change-password', data)
}

export default userApi
