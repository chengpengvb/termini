use summer::tracing::warn;
use rig::agent::{AgentHook, HookContext, InvalidToolCallAction, InvalidToolCallContext, StepEventKind};

pub struct InvalidToolCallSkipHook;

impl AgentHook for InvalidToolCallSkipHook {
    async fn on_invalid_tool_call(&self, _ctx: &HookContext, event: &InvalidToolCallContext) -> Option<InvalidToolCallAction> {
        warn!("[ai] 模型调用了本轮不可用的工具 tool={} allowed={:?}", event.tool_name, event.allowed_tools);
        Some(InvalidToolCallAction::skip(format!(
            "tool `{}` is not available this turn and was not executed. Available tools this turn: {}. Do not call it again; use an available tool or answer with text directly.",
            event.tool_name,
            event.allowed_tools.join(", ")
        )))
    }

    fn observes(&self, kind: StepEventKind) -> bool {
        matches!(kind, StepEventKind::InvalidToolCall)
    }
}
