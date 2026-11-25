use summer::tracing::warn;
use crate::module::ai::dto::AiToolId;
use crate::module::ai::shared::ChatSessionCacheManager;
use crate::module::ai::mcp::wrap::mcp_args_with_call_id;
use rig::agent::{AgentHook, HookContext, StepEventKind, ToolCall, ToolCallAction};

pub struct ToolCallIdInjectHook {
    pub req_id: String,
    pub chat_session_cache_manager: ChatSessionCacheManager,
}

impl AgentHook for ToolCallIdInjectHook {
    async fn on_tool_call(&self, _ctx: &HookContext, event: ToolCall<'_>) -> ToolCallAction {
        if let Err(error) = self.chat_session_cache_manager.ensure_tool_item_id(&self.req_id, event.internal_call_id) {
            warn!("[ai] 登记 tool item id 失败 req_id={} call_id={} error={}", self.req_id, event.internal_call_id, error);
        }
        self.chat_session_cache_manager.set_tool_id(&self.req_id, event.internal_call_id, event.tool_name);
        match serde_json::from_str::<serde_json::Value>(event.args) {
            Ok(serde_json::Value::Object(mut map)) => {
                map.insert("__internal_call_id".to_string(), serde_json::json!(event.internal_call_id));
                return ToolCallAction::rewrite(serde_json::Value::Object(map));
            }
            Ok(serde_json::Value::Null) | Err(_) if event.args.trim().is_empty() && AiToolId::from_name(event.tool_name).is_none() => {
                return ToolCallAction::rewrite(mcp_args_with_call_id(event.internal_call_id));
            }
            Ok(_) => {
                warn!("[ai] 注入 internal_call_id 失败：工具参数不是对象 tool={} internal_call_id={}", event.tool_name, event.internal_call_id);
            }
            Err(error) => {
                warn!("[ai] 注入 internal_call_id 失败：参数非法 json tool={} error={}", event.tool_name, error);
            }
        }
        ToolCallAction::Run
    }

    fn observes(&self, kind: StepEventKind) -> bool {
        matches!(kind, StepEventKind::ToolCall)
    }
}
