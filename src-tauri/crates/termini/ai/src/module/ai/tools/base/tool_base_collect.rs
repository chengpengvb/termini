use std::sync::Arc;
use summer::tracing::warn;
use rig::tool::DynamicTool;
use std::collections::HashSet;
use crate::module::ai::shared::ToolCallDonePush;
use super::{ToolBaseOutWrap, TOOL_OUTPUT_MAX_TOKENS};
use crate::module::ai::mcp::mcp_client_pool::McpClientPool;
use crate::module::ai::tools::tool_base::{ToolBase, ToolDomain};

pub struct ToolBaseCollect;

impl ToolBaseCollect {
    pub fn collect(builtin: &[Arc<dyn ToolBase>], mcp: &McpClientPool, domain: ToolDomain, done_push: &ToolCallDonePush) -> Vec<DynamicTool> {
        let mut out: Vec<DynamicTool> = builtin.iter().filter(|tool| tool.domains().contains(&domain)).map(|tool| tool.as_rig_tool(Some(done_push.clone()))).collect();
        if domain != ToolDomain::Main {
            return out;
        }
        let builtin_names: HashSet<String> = out.iter().map(|tool| tool.name().to_string()).collect();
        out.extend(Self::collect_mcp(mcp, &builtin_names, done_push));
        out
    }

    pub fn collect_mcp(mcp: &McpClientPool, builtin_names: &HashSet<String>, done_push: &ToolCallDonePush) -> Vec<DynamicTool> {
        let mut out = Vec::new();
        for tool in mcp.collect_rig_tools() {
            let name = tool.name().to_string();
            if builtin_names.contains(&name) {
                warn!("[mcp] MCP tool name conflicts with builtin, drop MCP tool: {}", name);
                continue;
            }
            out.push(ToolBaseOutWrap::wrap(tool, TOOL_OUTPUT_MAX_TOKENS, Some(done_push.clone())));
        }
        out
    }
}
