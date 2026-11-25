use parking_lot::Mutex;
use summer::tracing::warn;
use std::collections::HashSet;
use crate::module::ai::tools::base::INTERNAL_CALL_ID_KEY;
use rig::agent::{AgentHook, HookContext, StepEventKind, ToolCall, ToolCallAction};

pub struct QueryDedupHook {
    dedup_tools: HashSet<String>,
    seen: Mutex<HashSet<String>>,
}

impl QueryDedupHook {
    pub fn new(dedup_tools: HashSet<String>) -> Self {
        Self { dedup_tools, seen: Mutex::new(HashSet::new()) }
    }

    fn build_key(tool_name: &str, args: &str) -> String {
        let Ok(mut value) = serde_json::from_str::<serde_json::Value>(args) else {
            return format!("{tool_name}:{args}");
        };
        if let Some(object) = value.as_object_mut() {
            object.remove(INTERNAL_CALL_ID_KEY);
        }
        format!("{tool_name}:{value}")
    }
}

impl AgentHook for QueryDedupHook {
    async fn on_tool_call(&self, _ctx: &HookContext, event: ToolCall<'_>) -> ToolCallAction {
        if !self.dedup_tools.contains(event.tool_name) {
            return ToolCallAction::Run;
        }
        let key = Self::build_key(event.tool_name, event.args);
        if self.seen.lock().insert(key) {
            return ToolCallAction::Run;
        }
        warn!("[ai] 拦截重复查询 tool={} args={}", event.tool_name, event.args);
        ToolCallAction::skip(format!("tool `{}` was already queried with identical arguments this turn; the result is above, reuse it instead of calling again.", event.tool_name))
    }

    fn observes(&self, kind: StepEventKind) -> bool {
        matches!(kind, StepEventKind::ToolCall)
    }
}
