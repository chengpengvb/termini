use std::sync::Arc;
use tokio::sync::Mutex;
use summer::tracing::info;
use std::collections::HashSet;
use std::sync::atomic::Ordering;
use tokio_util::sync::CancellationToken;
use crate::module::ai::dto::FoldGeneration;
use crate::module::ai::shared::MemoryCompactor;
use crate::module::ai::shared::mem::fold_plan::FoldState;
use crate::module::ai::stream::stream_runner::AiStreamRunner;
use crate::module::ai::shared::mem::supersede::prune_superseded;
use rig::agent::{AgentHook, CompletionCallAction, CompletionCallEvent, HookContext, RequestPatch, StepEventKind};

pub struct ContextFoldHook {
    pub req_id: String,
    pub state: Arc<Mutex<FoldState>>,
    pub cancel_token: CancellationToken,
    pub fold_generation: FoldGeneration,
    pub memory_compactor: MemoryCompactor,
    pub supersede_tools: HashSet<String>,
}

impl AgentHook for ContextFoldHook {
    async fn on_completion_call(&self, _ctx: &HookContext, event: CompletionCallEvent<'_>) -> CompletionCallAction {
        let mut guard = self.state.lock().await;
        let folded_before = guard.folded;
        let folded = self.memory_compactor.fold_for_turn(&self.req_id, event.history, event.prompt, &mut guard, &self.cancel_token).await;
        if guard.folded != folded_before {
            self.fold_generation.fetch_add(1, Ordering::Relaxed);
        }
        let mut outgoing = folded.unwrap_or_else(|| event.history.to_vec());
        outgoing.push(event.prompt.clone());
        let pruned = prune_superseded(&mut outgoing, &self.supersede_tools);
        if pruned > 0 {
            info!("[ai] 剔除被取代的工具结果 req_id={} 条数={}", self.req_id, pruned);
        }
        let mut sanitized = AiStreamRunner::sanitize_tool_history(outgoing);
        if sanitized.last() == Some(event.prompt) {
            sanitized.pop();
        }
        if sanitized.as_slice() == event.history {
            return CompletionCallAction::Continue;
        }
        CompletionCallAction::patch(RequestPatch::new().history(sanitized))
    }

    fn observes(&self, kind: StepEventKind) -> bool {
        matches!(kind, StepEventKind::CompletionCall)
    }
}
