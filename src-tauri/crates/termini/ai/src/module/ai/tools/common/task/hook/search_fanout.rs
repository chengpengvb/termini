use std::sync::Arc;
use parking_lot::Mutex;
use summer::tracing::warn;
use crate::module::ai::dto::AiToolId;
use rig::agent::{AgentHook, HookContext, StepEventKind, ToolCall, ToolCallAction};
use crate::module::ai::tools::common::task::rule::search_fanout::{FanoutTracker, FanoutVerdict};

#[derive(Clone, Default)]
pub(crate) struct SearchFanoutHook {
    tracker: Arc<Mutex<FanoutTracker>>,
}

impl AgentHook for SearchFanoutHook {
    async fn on_tool_call(&self, _ctx: &HookContext, event: ToolCall<'_>) -> ToolCallAction {
        if event.tool_name != AiToolId::ToolCodeSearch.as_str() {
            return ToolCallAction::Run;
        }
        let Ok(value) = serde_json::from_str::<serde_json::Value>(event.args) else {
            return ToolCallAction::Run;
        };
        let verdict = self.tracker.lock().observe(&value);
        let FanoutVerdict::Fanout { files } = verdict else {
            return ToolCallAction::Run;
        };
        warn!("[ai] 拦截逐文件搜索 files={}", files.len());
        let listed: Vec<String> = files.iter().map(|path| format!("{{\"path\": \"{}\", \"limit\": 400}}", path)).collect();
        ToolCallAction::skip(format!(
            "not executed: you are searching single files one by one with the same regex, which uses code_search as file_read and costs a turn per file. \
             Read these files in one batch to get their full content instead of searching again: file_read files=[{}]. \
             Then answer from what you read; code_search is only for finding things in a directory when you do not know which file they are in.",
            listed.join(", ")
        ))
    }

    fn observes(&self, kind: StepEventKind) -> bool {
        matches!(kind, StepEventKind::ToolCall)
    }
}
