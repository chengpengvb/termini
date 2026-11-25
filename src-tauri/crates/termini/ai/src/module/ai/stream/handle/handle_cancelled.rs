use summer::tracing::warn;
use std::collections::HashSet;
use rig::agent::StreamingError;
use rig::completion::PromptError;
use crate::module::ai::dto::AiRunContext;
use crate::module::ai::stream::stream_runner::AiStreamRunner;

impl AiStreamRunner {
    pub(in crate::module::ai::stream) fn handle_cancelled(&self, context: &AiRunContext, error: &StreamingError, active_call_ids: &mut HashSet<String>) -> bool {
        let StreamingError::Prompt(prompt_err) = error else {
            return false;
        };
        let PromptError::PromptCancelled { chat_history, .. } = prompt_err.as_ref() else {
            return false;
        };
        let pending = active_call_ids.len();
        self.finish_active_tool_calls(context, active_call_ids);
        let clean = Self::sanitize_tool_history(chat_history.clone());
        let kept = Self::without_addon_messages(clean, &context.addon_messages);
        let kept_len = kept.len();
        *context.memory.lock() = kept;
        warn!("[ai] 流被取消，已回写记忆 run_id={} pending_tools={} memory_msgs={}", context.run_id, pending, kept_len);
        true
    }
}
