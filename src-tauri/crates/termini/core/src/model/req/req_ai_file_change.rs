use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReqAiFileChangeAction {
    pub path: String,
    pub chat_id: String,
    pub device_id: Option<String>,
}
