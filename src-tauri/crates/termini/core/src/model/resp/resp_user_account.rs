use serde::{Deserialize, Serialize};
use crate::model::resp::resp_user_subscription::RespUserSubscription;

#[derive(Debug, Clone, Deserialize)]
pub struct RespAccountToken {
    pub token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RespUserAccount {
    pub id: Option<String>,
    pub email: Option<String>,
    #[serde(default, rename = "creditScale")]
    pub credit_scale: Option<f64>,
    pub subscription: Option<RespUserSubscription>,
}
