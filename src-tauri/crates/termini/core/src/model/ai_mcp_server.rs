use termini_macros::base_target;
use serde::{Deserialize, Serialize};

#[base_target(table = "ai_mcp_server", entity_type = 13)]
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Model {
    pub sort: i16,
    #[serde(default, deserialize_with = "crate::model::base_target::deserialize_null_default")]
    pub name: String,
    pub enabled: bool,
    pub transport: i16,
    pub env: Option<String>,
    pub url: Option<String>,
    pub args: Option<String>,
    pub headers: Option<String>,
    pub command: Option<String>,
    pub timeout_secs: Option<i32>,
}

pub type AiMcpServer = Model;
