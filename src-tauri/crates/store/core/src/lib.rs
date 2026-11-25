use serde::{Deserialize, Serialize};

pub const STORE_KIND_APPLE: &str = "apple";
pub const STORE_KIND_GOOGLE: &str = "google";
pub const STORE_KIND_MICROSOFT: &str = "microsoft";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoreProduct {
    pub product_id: String,
    pub title: String,
    pub description: String,
    pub price_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorePurchase {
    pub store: String,
    pub product_id: String,
    pub transaction_id: String,
    pub receipt: String,
}

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("{0} in-app purchases are not supported on this platform")]
    Unsupported(&'static str),

    #[error("store not ready: {0}")]
    NotReady(String),

    #[error("purchase cancelled by user")]
    Cancelled,

    #[error("purchase failed: {0}")]
    Failed(String),
}

pub type StoreResult<T> = Result<T, StoreError>;
