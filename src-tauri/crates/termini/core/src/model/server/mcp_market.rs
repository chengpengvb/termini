use serde::{Deserialize, Serialize};

pub const MCP_MARKET_RUNTIME_NODE: i32 = 0;
pub const MCP_MARKET_RUNTIME_PYTHON: i32 = 1;
pub const MCP_MARKET_RUNTIME_BINARY: i32 = 2;
pub const MCP_MARKET_RUNTIME_HTTP: i32 = 3;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpMarketItem {
    pub id: String,
    #[serde(default)]
    pub sort: i32,
    #[serde(default)]
    pub enabled: bool,
    pub name: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
    pub runtime: i32,
    #[serde(default)]
    pub pkg: Option<String>,
    #[serde(default)]
    pub args: Option<String>,
    #[serde(default)]
    pub env_keys: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub homepage: Option<String>,
    #[serde(default)]
    pub update_time: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpMarketEnvKey {
    pub key: String,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub placeholder: Option<String>,
}
