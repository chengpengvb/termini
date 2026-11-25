use std::sync::Arc;
use summer::tracing::{info, warn};
use crate::module::ai::tools::common::task::rule::scope;
use rig::agent::{AgentHook, HookContext, StepEventKind, ToolCall, ToolCallAction};

#[derive(Clone)]
pub(crate) struct ScopeGuardHook {
    scope: Arc<Vec<String>>,
}

impl ScopeGuardHook {
    pub(crate) fn new(scope: Vec<String>) -> Self {
        Self { scope: Arc::new(scope) }
    }
}

impl AgentHook for ScopeGuardHook {
    async fn on_tool_call(&self, _ctx: &HookContext, event: ToolCall<'_>) -> ToolCallAction {
        if self.scope.is_empty() {
            return ToolCallAction::Run;
        }
        let Ok(value) = serde_json::from_str::<serde_json::Value>(event.args) else {
            return ToolCallAction::Run;
        };
        let offending = scope::offending_paths(&self.scope, &value);
        if offending.is_empty() {
            return ToolCallAction::Run;
        }
        if let Some(next) = scope::narrow_to_scope(event.tool_name, &self.scope, &value) {
            info!(
                "[ai] 检索 root 在范围上层, 已收窄 tool={} root={:?} -> {}",
                event.tool_name,
                value.get("root"),
                next.get("glob").or(next.get("root")).map(|v| v.to_string()).unwrap_or_default()
            );
            return ToolCallAction::rewrite(next);
        }
        warn!("[ai] 拦截越界调查 tool={} outside={:?}", event.tool_name, offending);
        ToolCallAction::skip(format!(
            "tool `{}` accessed paths outside the scope of this investigation: {}. This investigation is limited to these directories: {}. Narrow root/path to the allowed scope and retry; do not browse modules outside the scope. If it truly cannot be found within scope, conclude from what you have found.",
            event.tool_name,
            offending.join(", "),
            self.scope.join(", ")
        ))
    }

    fn observes(&self, kind: StepEventKind) -> bool {
        matches!(kind, StepEventKind::ToolCall)
    }
}
