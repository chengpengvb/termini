use std::sync::Arc;
use parking_lot::Mutex;
use summer::tracing::warn;
use crate::module::ai::dto::AiToolId;
use crate::module::ai::tools::common::task::rule::search_nomatch::{NoMatchTracker, NoMatchVerdict};
use rig::agent::{AgentHook, HookContext, StepEventKind, ToolCall, ToolCallAction, ToolResultAction, ToolResultEvent};

#[derive(Clone, Default)]
pub(crate) struct SearchNoMatchHook {
    tracker: Arc<Mutex<NoMatchTracker>>,
}

impl AgentHook for SearchNoMatchHook {
    async fn on_tool_call(&self, _ctx: &HookContext, event: ToolCall<'_>) -> ToolCallAction {
        if event.tool_name != AiToolId::ToolCodeSearch.as_str() {
            return ToolCallAction::Run;
        }
        let Ok(value) = serde_json::from_str::<serde_json::Value>(event.args) else {
            return ToolCallAction::Run;
        };
        match self.tracker.lock().filter(&value) {
            NoMatchVerdict::Allow => ToolCallAction::Run,
            NoMatchVerdict::Rewrite { args, dropped } => {
                warn!("[ai] 剔除已知无命中的关键字 dropped={:?}", dropped);
                ToolCallAction::rewrite(args)
            }
            NoMatchVerdict::Skip { dropped } => {
                warn!("[ai] 拦截全部已知无命中的检索 dropped={:?}", dropped);
                ToolCallAction::skip(format!(
                    "not executed: {} have already been searched in several locations with no hits; they are not in the code within this investigation's scope, and searching other directories will not help. \
                     If these are names mentioned in the prompt, state in the report that the symbol was not found within scope; if you are looking for a concept, ask code_semantic a natural-language question about the directory instead.",
                    dropped.iter().map(|item| format!("`{}`", item)).collect::<Vec<_>>().join(", ")
                ))
            }
        }
    }

    async fn on_tool_result(&self, _ctx: &HookContext, event: ToolResultEvent<'_>) -> ToolResultAction {
        if event.tool_name != AiToolId::ToolCodeSearch.as_str() {
            return ToolResultAction::Keep;
        }
        let Ok(value) = serde_json::from_str::<serde_json::Value>(event.args) else {
            return ToolResultAction::Keep;
        };
        let Some(result) = event.presentation.as_text() else {
            return ToolResultAction::Keep;
        };
        self.tracker.lock().record(&value, result);
        ToolResultAction::Keep
    }

    fn observes(&self, kind: StepEventKind) -> bool {
        matches!(kind, StepEventKind::ToolCall | StepEventKind::ToolResult)
    }
}
