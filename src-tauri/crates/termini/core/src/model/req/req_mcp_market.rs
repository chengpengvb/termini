use std::collections::BTreeMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReqMcpMarketInstall {
    pub market_id: String,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
}
