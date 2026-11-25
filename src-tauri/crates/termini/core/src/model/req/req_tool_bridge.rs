use serde_json::Value;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct ReqToolBridgeCall {
    pub token: String,
    pub name: String,

    #[serde(default)]
    pub args: Value,
}
