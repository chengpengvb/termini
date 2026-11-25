use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct McpToolSnapshot {
    pub name: String,
    pub server_id: String,
    pub server_name: String,
    pub description: String,
    pub schema: serde_json::Value,
}
