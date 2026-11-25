use serde_json::Value;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RespAiModelInfo {
    pub name: String,
    pub max_token: u32,
    #[serde(default)]
    pub vision: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub no_thinking: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RespAiProviderInfo {
    pub name: String,
    pub provide_id: i16,
    pub models: Vec<RespAiModelInfo>,
    pub default_url: Option<String>,
}
