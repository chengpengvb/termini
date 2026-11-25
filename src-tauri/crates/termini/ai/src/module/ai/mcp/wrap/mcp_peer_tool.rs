use serde_json::Value;
use std::time::Duration;
use rmcp::service::{Peer, RoleClient};
use crate::module::ai::tools::ai_tool_error::AiToolError;
use rmcp::model::{CallToolRequestParams, CallToolResult, JsonObject, Tool};

#[derive(Clone)]
pub struct McpPeerTool {
    definition: Tool,
    peer: Peer<RoleClient>,
    timeout: Duration,
}

impl McpPeerTool {
    pub fn new(definition: Tool, peer: Peer<RoleClient>, timeout: Duration) -> Self {
        Self { definition, peer, timeout }
    }

    pub fn name(&self) -> String {
        self.definition.name.to_string()
    }

    pub fn description(&self) -> String {
        match &self.definition.description {
            Some(text) => text.to_string(),
            None => String::new(),
        }
    }

    pub fn parameters(&self) -> Value {
        self.definition.schema_as_json_value()
    }

    pub async fn invoke(&self, args: &str) -> Result<String, AiToolError> {
        let arguments = Self::parse_arguments(args)?;
        let mut params = CallToolRequestParams::new(self.definition.name.clone());
        if let Some(arguments) = arguments {
            params = params.with_arguments(arguments);
        }
        let result = match tokio::time::timeout(self.timeout, self.peer.call_tool(params)).await {
            Ok(Ok(result)) => result,
            Ok(Err(error)) => return Err(AiToolError::Execution(format!("MCP tool '{}' request failed: {}", self.definition.name, error))),
            Err(_) => return Err(AiToolError::Execution(format!("MCP tool '{}' timed out after {:?}", self.definition.name, self.timeout))),
        };
        let body = Self::present(&result)?;
        if result.is_error == Some(true) {
            return Err(AiToolError::Execution(format!("MCP tool '{}' reported an execution error: {}", self.definition.name, body)));
        }
        Ok(body)
    }

    fn parse_arguments(args: &str) -> Result<Option<JsonObject>, AiToolError> {
        let trimmed = args.trim();
        if trimmed.is_empty() {
            return Ok(None);
        }
        let value: Value = serde_json::from_str(trimmed).map_err(|error| AiToolError::Execution(format!("MCP arguments are not valid json: {}", error)))?;
        match value {
            Value::Null => Ok(None),
            Value::Object(map) => Ok(Some(map)),
            _ => Err(AiToolError::Execution("MCP tool arguments must be an object".to_string())),
        }
    }

    fn present(result: &CallToolResult) -> Result<String, AiToolError> {
        let mut parts: Vec<String> = Vec::new();
        for block in &result.content {
            if let Some(text) = block.as_text() {
                parts.push(text.text.clone());
            }
        }
        if !parts.is_empty() {
            return Ok(parts.join("\n"));
        }
        match &result.structured_content {
            Some(value) => serde_json::to_string(value).map_err(|error| AiToolError::Execution(error.to_string())),
            None => Ok(String::new()),
        }
    }
}
