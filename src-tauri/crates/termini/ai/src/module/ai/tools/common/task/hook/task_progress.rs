use crate::module::ai::shared::chat_item_format::truncate_chars;
use crate::module::ai::tools::common::task::session::task_registry::TaskEntry;
use rig::agent::{AgentHook, HookContext, StepEventKind, ToolCall, ToolCallAction, ToolResultAction, ToolResultEvent};

#[derive(Clone)]
pub(crate) struct TaskProgressHook {
    pub(crate) task_entry: TaskEntry,
}

impl TaskProgressHook {
    const ARG_CHARS: usize = 80;
    const RESULT_CHARS: usize = 160;

    fn summarize_args(args: &str) -> String {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(args) else {
            return truncate_chars(args, Self::ARG_CHARS);
        };
        let Some(object) = value.as_object() else {
            return truncate_chars(args, Self::ARG_CHARS);
        };
        let mut parts = Vec::new();
        for key in ["title", "path", "root", "cwd", "command", "query", "name", "focus", "glob"] {
            let Some(text) = object.get(key).and_then(|item| item.as_str()) else {
                continue;
            };
            if text.is_empty() {
                continue;
            }
            parts.push(format!("{}={}", key, truncate_chars(text, Self::ARG_CHARS)));
        }
        for key in ["files", "queries", "edits", "actions", "commands"] {
            let Some(items) = object.get(key).and_then(|item| item.as_array()) else {
                continue;
            };
            if !items.is_empty() {
                parts.push(format!("{}×{}", key, items.len()));
            }
        }
        if parts.is_empty() {
            return truncate_chars(args, Self::ARG_CHARS);
        }
        parts.join(" ")
    }
}

impl AgentHook for TaskProgressHook {
    async fn on_tool_call(&self, _ctx: &HookContext, event: ToolCall<'_>) -> ToolCallAction {
        self.task_entry.append_progress(&format!("→ {} {}\n", event.tool_name, Self::summarize_args(event.args)));
        ToolCallAction::Run
    }

    async fn on_tool_result(&self, _ctx: &HookContext, event: ToolResultEvent<'_>) -> ToolResultAction {
        if let Some(result) = event.presentation.as_text() {
            if let Some(line) = result.lines().rev().find(|row| !row.is_empty()) {
                self.task_entry.append_progress(&format!("← {}\n", truncate_chars(line, Self::RESULT_CHARS)));
            }
        }
        ToolResultAction::Keep
    }

    fn observes(&self, kind: StepEventKind) -> bool {
        matches!(kind, StepEventKind::ToolCall | StepEventKind::ToolResult)
    }
}
