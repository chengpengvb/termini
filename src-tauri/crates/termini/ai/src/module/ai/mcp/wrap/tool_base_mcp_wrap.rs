use std::sync::Arc;
use super::McpPeerTool;
use serde_json::{json, Value};
use crate::module::ai::dto::AiRunContext;
use crate::module::ai::tools::base::WrapItem;
use crate::module::ai::shared::ToolConflictManager;
use crate::module::ai::shared::ChatSessionCacheManager;
use crate::module::ai::tools::ai_tool_error::AiToolError;
use rig::tool::{DynamicTool, ToolContext, ToolExecutionError, ToolOutput};

const CANCELLED_RESULT: &str = "exit_code=cancelled";
const INTERNAL_CALL_ID_KEY: &str = "__internal_call_id";

#[derive(Clone)]
pub struct ToolBaseMcpWrap {
    inner: McpPeerTool,
    tool_conflict_manager: ToolConflictManager,
    chat_session_cache_manager: ChatSessionCacheManager,
}

impl ToolBaseMcpWrap {
    pub fn new(inner: McpPeerTool, tool_conflict_manager: ToolConflictManager, chat_session_cache_manager: ChatSessionCacheManager) -> Self {
        Self { inner, tool_conflict_manager, chat_session_cache_manager }
    }

    pub fn as_rig_tool(&self) -> DynamicTool {
        let inner = self.inner.clone();
        let tool_conflict_manager = self.tool_conflict_manager.clone();
        let chat_session_cache_manager = self.chat_session_cache_manager.clone();
        DynamicTool::new(inner.name(), inner.description(), inner.parameters(), move |context, args| {
            let inner = inner.clone();
            let tool_conflict_manager = tool_conflict_manager.clone();
            let chat_session_cache_manager = chat_session_cache_manager.clone();
            Box::pin(async move { Self::execute(inner, tool_conflict_manager, chat_session_cache_manager, context, args).await })
        })
    }

    async fn execute(
        inner: McpPeerTool,
        tool_conflict_manager: ToolConflictManager,
        chat_session_cache_manager: ChatSessionCacheManager,
        context: &mut ToolContext,
        args: Value,
    ) -> Result<ToolOutput, ToolExecutionError> {
        let run_context = context.require::<Arc<AiRunContext>>().map_err(|error| ToolExecutionError::other(error.to_string()))?.clone();
        let (call_id, cleaned_args, confirm_args) = Self::split_call_id(args).map_err(|error| ToolExecutionError::other(error.to_string()))?;
        let session_cancel = run_context.cancel_token.clone();
        let Some(tool_cancel) = chat_session_cache_manager.get_tool_cancel(&run_context.req_id, &run_context.run_id, &call_id) else {
            return Err(ToolExecutionError::other(format!("tool cancel not found: req_id={} run_id={} call_id={}", run_context.req_id, run_context.run_id, call_id)));
        };
        let tool_name = inner.name();
        tool_conflict_manager.confirm_before_execute(&run_context, &call_id, &tool_name, &confirm_args).await.map_err(|error| ToolExecutionError::other(error.to_string()))?;
        let body = tokio::select! {
            biased;
            _ = tool_cancel.cancelled() => CANCELLED_RESULT.to_string(),
            _ = session_cancel.cancelled() => CANCELLED_RESULT.to_string(),
            res = inner.invoke(&cleaned_args) => res.map_err(|error| ToolExecutionError::other(error.to_string()))?,
        };
        context.insert_result(vec![WrapItem::wrap_item(None, body.clone())]);
        Ok(ToolOutput::text(body))
    }

    fn split_call_id(args: Value) -> Result<(String, String, Value), AiToolError> {
        let Value::Object(mut map) = args else {
            return Err(AiToolError::Execution("MCP tool arguments must be an object".to_string()));
        };
        let call_id = match map.remove(INTERNAL_CALL_ID_KEY) {
            Some(Value::String(id)) if !id.is_empty() => id,
            _ => {
                return Err(AiToolError::Execution("internal_call_id of the tool call is missing".to_string()));
            }
        };
        let confirm_args = Value::Object(map.clone());
        let cleaned = serde_json::to_string(&Value::Object(map)).map_err(|error| AiToolError::Execution(error.to_string()))?;
        Ok((call_id, cleaned, confirm_args))
    }
}

pub fn mcp_args_with_call_id(internal_call_id: &str) -> Value {
    json!({ INTERNAL_CALL_ID_KEY: internal_call_id })
}
