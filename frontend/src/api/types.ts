// API 类型定义
export interface User {
  user_id: string
  username: string
  email: string | null
  tier: string
  balance: number
}

export interface LoginRequest {
  account: string
  password: string
}

export interface LoginResponse {
  success: boolean
  data: {
    api_key: string
    user_id: string
    username: string
    email: string | null
    tier: string
    balance: number
  }
  error?: string
  message?: string
}

export interface RegisterRequest {
  username: string
  email: string
  password: string
}

export interface BalanceResponse {
  balance: number
}

export interface HistoryRecord {
  created_at: number
  model: string
  input_tokens: number
  output_tokens: number
  total_tokens: number
  cost: number
}

export interface HistoryResponse {
  records: HistoryRecord[]
}

export interface RedeemRequest {
  code: string
}

export interface RedeemResponse {
  success: boolean
  message: string
  amount?: number
}

export interface ChangePasswordRequest {
  old_password: string
  new_password: string
}

export interface ChangePasswordResponse {
  success: boolean
  message: string
}

export interface ProfileRequest {
  username?: string
}

export interface ProfileResponse {
  success: boolean
  data: {
    user_id: string
    username: string
    email: string | null
    tier: string
    balance: number
    created_at: number
  }
}

export interface StatsResponse {
  total_users: number
  active_users: number
  total_requests: number
}

export interface HealthResponse {
  status: string
  timestamp: string
  version: string
}

export interface ModelsResponse {
  object: string
  data: Array<{
    id: string
    object: string
    owned_by: string
  }>
}
