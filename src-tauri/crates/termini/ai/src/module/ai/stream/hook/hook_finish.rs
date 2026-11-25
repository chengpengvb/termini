use tokio_util::sync::CancellationToken;
use rig::agent::{AgentHook, CompletionCallAction, CompletionCallEvent, HookContext, StepEventKind};

pub struct FinishHook {
    pub token: CancellationToken,
}

impl AgentHook for FinishHook {
    async fn on_completion_call(&self, _ctx: &HookContext, _event: CompletionCallEvent<'_>) -> CompletionCallAction {
        if self.token.is_cancelled() {
            return CompletionCallAction::stop("tool finish");
        }
        CompletionCallAction::Continue
    }

    fn observes(&self, kind: StepEventKind) -> bool {
        kind == StepEventKind::CompletionCall
    }
}
