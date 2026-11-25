use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReqAiChatRevert {
    pub chat_id: String,
    pub item_id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReqAiChatSearch {
    #[serde(default)]
    pub q: String,
}
