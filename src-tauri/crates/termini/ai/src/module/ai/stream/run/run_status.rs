use rig::completion::Usage;
use std::collections::HashSet;
use crate::module::ai::dto::AiRunContext;
use crate::model::resp::resp_agent::RespAgentStatus;
use crate::module::ai::stream::stream_runner::AiStreamRunner;
use crate::model::resp::event::resp_event_agent::RespEventAgent;

impl AiStreamRunner {
    pub(in crate::module::ai::stream) fn collect_dedup_tools(&self, req_id: &str) -> HashSet<String> {
        let Some(entry) = self.chat_session_cache_manager.get(req_id) else {
            return HashSet::new();
        };
        entry.tools.iter().filter(|tool| tool.dedup_in_run()).map(|tool| tool.tool_id().as_str().to_string()).collect()
    }

    pub(in crate::module::ai::stream) fn collect_supersede_tools(&self, req_id: &str) -> HashSet<String> {
        let Some(entry) = self.chat_session_cache_manager.get(req_id) else {
            return HashSet::new();
        };
        entry.tools.iter().filter(|tool| tool.supersede_in_run()).map(|tool| tool.tool_id().as_str().to_string()).collect()
    }

    pub(in crate::module::ai::stream) fn record_usage(&self, context: &AiRunContext, usage: &Usage, completion_calls: usize) {
        let Some(entry) = self.chat_session_cache_manager.get(&context.req_id) else {
            return;
        };
        entry.run_trace.lock().absorb(usage, completion_calls);
    }

    pub(in crate::module::ai::stream) fn record_context_tokens(&self, context: &AiRunContext, usage: &Usage) {
        self.chat_session_cache_manager.record_context_tokens(&context.req_id, usage.input_tokens);
    }

    pub(in crate::module::ai::stream) fn emit_thinking_status(&self, context: &AiRunContext) {
        self.emit_status(context, RespAgentStatus::stage(&context.req_id, "thinking"));
    }

    pub(in crate::module::ai::stream) fn emit_retry_status(&self, context: &AiRunContext, attempt: usize, max_attempts: usize) {
        if attempt <= 1 {
            self.emit_thinking_status(context);
            return;
        }
        self.emit_status(context, RespAgentStatus::retrying(&context.req_id, attempt, max_attempts));
    }

    fn emit_status(&self, context: &AiRunContext, status: RespAgentStatus) {
        let Some(entry) = self.chat_session_cache_manager.get(&context.req_id) else {
            return;
        };
        let ai_chat_id = entry.context.ai_chat_id.clone();
        drop(entry);
        let event = RespEventAgent::OnStatus(status);
        self.agent_service.handler_agent_event(event, ai_chat_id, None, None, None);
    }
}
