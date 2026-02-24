import apiClient from './client'
import type { RedeemRequest, RedeemResponse } from './types'

export const rechargeApi = {
  // 兑换充值卡
  redeem: (data: RedeemRequest) =>
    apiClient.post<RedeemResponse>('/recharge/redeem', data)
}

export default rechargeApi
