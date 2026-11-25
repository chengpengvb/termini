use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RespUserSubscription {
    pub plan: i16,
    pub store: i16,
    pub start_time: i64,
    pub expire_time: i64,
    pub monthly_quota: f64,
    pub used_quota: f64,
    pub balance_quota: f64,
    pub quota_reset_time: i64,
}
