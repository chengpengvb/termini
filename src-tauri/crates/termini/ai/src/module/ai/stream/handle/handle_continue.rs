use summer::tracing::info;
use std::collections::HashSet;
use rig::agent::PromptResponse;
use std::sync::atomic::Ordering;
use rig::completion::FinishReason;
use rig::message::AssistantContent;
use crate::module::ai::dto::AiRunContext;
use crate::module::ai::stream::stream_runner::AiStreamRunner;

impl AiStreamRunner {
    pub(in crate::module::ai::stream) fn handle_continue(&self, context: &AiRunContext, response: &PromptResponse, active_call_ids: &HashSet<String>) {
        if !active_call_ids.is_empty() {
            return;
        }
        let Some(last) = response.completion_calls.last() else {
            return;
        };
        if last.finish_reason.as_ref() != Some(&FinishReason::Length) {
            return;
        }
        if response.content.iter().any(|part| matches!(part, AssistantContent::ToolCall(_))) {
            return;
        }
        info!("[ai] 输出被截断 req_id={} run_id={}", context.req_id, context.run_id);
        self.mark_auto_continue(context);
    }

    pub(in crate::module::ai::stream) fn mark_auto_continue(&self, context: &AiRunContext) {
        if !context.emit.pause {
            return;
        }
        let Some(entry) = self.chat_session_cache_manager.get(&context.req_id) else {
            return;
        };
        if entry.parent_req_id.lock().is_some() {
            return;
        }
        entry.auto_continue.store(true, Ordering::SeqCst);
    }
}
