use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RespAiFileChangeView {
    pub state: i16,
    pub path: String,
    pub timestamp: i64,
    pub chat_id: String,
    pub edit_count: usize,
    pub device_id: Option<String>,
    pub current_text: Option<String>,
    pub baseline_text: Option<String>,
}
