use rmcp::ServiceExt;
use std::time::Duration;
use summer::tracing::warn;
#[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
use tokio::process::Command;
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use crate::model::ai_mcp_server::AiMcpServer;
use crate::module::ai::mcp::wrap::McpPeerTool;
use rig::tool::rmcp::DEFAULT_MCP_TOOL_TIMEOUT;
use rmcp::service::{Peer, RoleClient, RunningService};
use crate::module::ai::mcp::dto::mcp_tool_snapshot::McpToolSnapshot;
use crate::module::ai::mcp::dto::mcp_transport_kind::MCP_TRANSPORT_HTTP;
#[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
use crate::module::ai::mcp::dto::mcp_transport_kind::MCP_TRANSPORT_STDIO;
use rmcp::model::{ClientCapabilities, ClientInfo, Implementation, Prompt, PromptArgument};
use crate::module::ai::mcp::dto::mcp_prompt_snapshot::{McpPromptArgument, McpPromptSnapshot};

const CONNECT_TIMEOUT_SECS: u64 = 30;

type McpRunningService = RunningService<RoleClient, ClientInfo>;

pub struct McpServerRuntime {
    pub sort: i16,
    pub connected: bool,
    pub server_id: String,
    tools: Vec<McpPeerTool>,
    pub last_error: Option<String>,
    pub snapshots: Vec<McpToolSnapshot>,
    pub prompts: Vec<McpPromptSnapshot>,
    session: Option<McpRunningService>,
}

impl McpServerRuntime {
    pub fn disconnected(server: &AiMcpServer, error: String) -> Self {
        Self { server_id: server.id.clone(), sort: server.sort, connected: false, last_error: Some(error), snapshots: Vec::new(), prompts: Vec::new(), tools: Vec::new(), session: None }
    }

    pub async fn start(server: AiMcpServer) -> Self {
        match Self::connect(&server).await {
            Ok(runtime) => runtime,
            Err(error) => {
                warn!("[mcp] MCP server [{}] connect failed: {}", server.id, error);
                Self::disconnected(&server, error.to_string())
            }
        }
    }

    pub async fn probe(server: &AiMcpServer) -> Result<Vec<McpToolSnapshot>> {
        let runtime = Self::connect(server).await?;
        Ok(runtime.snapshots)
    }

    pub fn tools(&self) -> &[McpPeerTool] {
        &self.tools
    }

    pub fn peer(&self) -> Option<Peer<RoleClient>> {
        self.session.as_ref().map(|session| session.peer().clone())
    }

    async fn connect(server: &AiMcpServer) -> Result<Self> {
        let connect_future = Self::connect_inner(server);
        match tokio::time::timeout(Duration::from_secs(CONNECT_TIMEOUT_SECS), connect_future).await {
            Ok(result) => result,
            Err(_) => Err(anyhow!("MCP connect timed out ({}s)", CONNECT_TIMEOUT_SECS)),
        }
    }

    async fn connect_inner(server: &AiMcpServer) -> Result<Self> {
        let client_info = ClientInfo::new(ClientCapabilities::default(), Implementation::new("termini", env!("CARGO_PKG_VERSION")));
        let session = match server.transport {
            #[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
            MCP_TRANSPORT_STDIO => {
                let transport = build_stdio_transport(server)?;
                client_info.serve(transport).await.map_err(|error| anyhow!("stdio MCP connect failed: {}", error))?
            }
            MCP_TRANSPORT_HTTP => {
                let url = require_non_empty(server.url.as_deref(), "http transport requires url")?;
                if let Some(headers_raw) = server.headers.as_deref() {
                    let trimmed = headers_raw.trim();
                    if !trimmed.is_empty() {
                        let _headers = parse_headers_json(Some(trimmed))?;
                    }
                }
                let transport = rmcp::transport::StreamableHttpClientTransport::from_uri(url);
                client_info.serve(transport).await.map_err(|error| anyhow!("http MCP connect failed: {}", error))?
            }
            other => {
                return Err(anyhow!("unsupported MCP transport: {}", other));
            }
        };

        let tool_defs = session.peer().list_all_tools().await.map_err(|error| anyhow!("list tools failed: {}", error))?;
        let peer = session.peer().clone();
        let timeout = resolve_timeout(server.timeout_secs).unwrap_or_else(|| DEFAULT_MCP_TOOL_TIMEOUT);
        let mut tools = Vec::with_capacity(tool_defs.len());
        let mut snapshots = Vec::with_capacity(tool_defs.len());
        for tool_def in tool_defs {
            let description = match &tool_def.description {
                Some(text) => text.to_string(),
                None => String::new(),
            };
            let snapshot = McpToolSnapshot { server_id: server.id.clone(), server_name: server.name.clone(), name: tool_def.name.to_string(), description, schema: tool_def.schema_as_json_value() };
            snapshots.push(snapshot);
            tools.push(McpPeerTool::new(tool_def, peer.clone(), timeout));
        }
        let prompts = Self::list_prompts(&session, server).await?;

        Ok(Self { server_id: server.id.clone(), sort: server.sort, connected: true, last_error: None, snapshots, prompts, tools, session: Some(session) })
    }

    async fn list_prompts(session: &McpRunningService, server: &AiMcpServer) -> Result<Vec<McpPromptSnapshot>> {
        let supported = match session.peer_info() {
            Some(info) => info.capabilities.prompts.is_some(),
            None => false,
        };
        if !supported {
            return Ok(Vec::new());
        }
        let prompt_defs = session.peer().list_all_prompts().await.map_err(|error| anyhow!("list prompts failed: {}", error))?;
        Ok(prompt_defs.into_iter().map(|prompt| Self::prompt_snapshot(server, prompt)).collect())
    }

    fn prompt_snapshot(server: &AiMcpServer, prompt: Prompt) -> McpPromptSnapshot {
        let description = prompt.description.unwrap_or_else(|| String::new());
        let arguments = match prompt.arguments {
            Some(list) => list.into_iter().map(Self::prompt_argument).collect(),
            None => Vec::new(),
        };
        McpPromptSnapshot { server_id: server.id.clone(), server_name: server.name.clone(), name: prompt.name, description, arguments }
    }

    fn prompt_argument(argument: PromptArgument) -> McpPromptArgument {
        let description = argument.description.unwrap_or_else(|| String::new());
        let required = argument.required.unwrap_or_else(|| false);
        McpPromptArgument { name: argument.name, description, required }
    }
}

fn resolve_timeout(timeout_secs: Option<i32>) -> Option<Duration> {
    let Some(secs) = timeout_secs else {
        return None;
    };
    if secs <= 0 {
        return None;
    }
    Some(Duration::from_secs(secs as u64))
}

fn require_non_empty(value: Option<&str>, message: &str) -> Result<String> {
    let Some(text) = value else {
        return Err(anyhow!(message.to_string()));
    };
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err(anyhow!(message.to_string()));
    }
    Ok(trimmed.to_string())
}

#[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
fn build_stdio_transport(server: &AiMcpServer) -> Result<rmcp::transport::TokioChildProcess> {
    let command = require_non_empty(server.command.as_deref(), "stdio transport requires command")?;
    let args = parse_args_json(server.args.as_deref())?;
    let env = parse_env_json(server.env.as_deref())?;
    let mut cmd = Command::new(command);
    for arg in args {
        cmd.arg(arg);
    }
    for (key, value) in env {
        cmd.env(key, value);
    }
    #[cfg(windows)]
    cmd.creation_flags(0x0800_0000);
    rmcp::transport::TokioChildProcess::new(cmd).map_err(|error| anyhow!("failed to spawn stdio process: {}", error))
}

#[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
fn parse_args_json(raw: Option<&str>) -> Result<Vec<String>> {
    let Some(text) = raw else {
        return Ok(Vec::new());
    };
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }
    let value: serde_json::Value = serde_json::from_str(trimmed).map_err(|error| anyhow!("args must be a JSON array: {}", error))?;
    let Some(items) = value.as_array() else {
        return Err(anyhow!("args must be a JSON array"));
    };
    let mut args = Vec::with_capacity(items.len());
    for item in items {
        let Some(text) = item.as_str() else {
            return Err(anyhow!("args array elements must be strings"));
        };
        args.push(text.to_string());
    }
    Ok(args)
}

#[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
fn parse_env_json(raw: Option<&str>) -> Result<HashMap<String, String>> {
    parse_string_map_json(raw, "env")
}

fn parse_headers_json(raw: Option<&str>) -> Result<HashMap<String, String>> {
    parse_string_map_json(raw, "headers")
}

fn parse_string_map_json(raw: Option<&str>, field: &str) -> Result<HashMap<String, String>> {
    let Some(text) = raw else {
        return Ok(HashMap::new());
    };
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Ok(HashMap::new());
    }
    let value: serde_json::Value = serde_json::from_str(trimmed).map_err(|error| anyhow!("{} must be a JSON object: {}", field, error))?;
    let Some(object) = value.as_object() else {
        return Err(anyhow!("{} must be a JSON object", field));
    };
    let mut map = HashMap::with_capacity(object.len());
    for (key, item) in object {
        let Some(text) = item.as_str() else {
            return Err(anyhow!("{} values must be strings", field));
        };
        map.insert(key.clone(), text.to_string());
    }
    Ok(map)
}
