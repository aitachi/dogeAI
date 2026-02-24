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
    apiClient.post<LoginResponse>('/api/user/login', data),

  // 注册
  register: (data: RegisterRequest) =>
    apiClient.post<LoginResponse>('/api/user/register', data),

  // 获取用户资料
  getProfile: (data: ProfileRequest) =>
    apiClient.post<ProfileResponse>('/api/user/profile', data),

  // 获取余额
  getBalance: () =>
    apiClient.get<BalanceResponse>('/api/user/balance'),

  // 获取使用历史
  getHistory: () =>
    apiClient.get<HistoryResponse>('/api/user/history'),

  // 修改密码
  changePassword: (data: ChangePasswordRequest) =>
    apiClient.post<ChangePasswordResponse>('/api/user/change-password', data),

  // 充值
  recharge: (data: { code: string }) =>
    apiClient.post<{ success: boolean; message: string; amount?: number }>('/api/user/recharge', data)
}

export default userApi
