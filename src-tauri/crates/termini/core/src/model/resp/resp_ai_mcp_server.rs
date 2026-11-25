use serde::Serialize;
use crate::model::ai_mcp_server::AiMcpServer;
use crate::model::mcp_tool_snapshot::McpToolSnapshot;

#[derive(Clone, Serialize)]
pub struct RespAiMcpServerRuntime {
    pub server: AiMcpServer,
    pub connected: bool,
    pub tool_count: i32,
    pub last_error: Option<String>,
}

#[derive(Clone, Serialize)]
pub struct RespAiMcpServerTools {
    pub tools: Vec<McpToolSnapshot>,
}
