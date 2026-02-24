// ========== 计费相关模型 ==========
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct BalanceResponse {
    pub status: String,
    pub balance: i64,
}

#[derive(Debug, Serialize)]
pub struct HistoryResponse {
    pub records: Vec<HistoryRecord>,
}

#[derive(Debug, Serialize)]
pub struct HistoryRecord {
    pub id: String,
    pub amount: i64,
    pub description: String,
    pub created_at: String,
}
