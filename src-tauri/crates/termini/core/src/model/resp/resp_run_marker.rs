use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RespRunMarker {
    pub pid: u32,
    pub version: String,
    pub started_at: i64,
}
