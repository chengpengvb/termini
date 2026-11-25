use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RespAiSshBatchItem {
    pub ok: bool,
    pub name: String,
    pub host: String,
    pub output: String,
    pub device_id: String,
    pub error: Option<String>,
    pub exit_code: Option<i32>,
}
