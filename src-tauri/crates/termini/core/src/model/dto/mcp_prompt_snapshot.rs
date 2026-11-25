use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct McpPromptArgument {
    pub name: String,
    pub required: bool,
    pub description: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct McpPromptSnapshot {
    pub name: String,
    pub server_id: String,
    pub server_name: String,
    pub description: String,
    pub arguments: Vec<McpPromptArgument>,
}
