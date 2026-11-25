use crate::module::ai::dto::AiRunContext;
use crate::model::resp::event::resp_event_agent::RespEventAgent;
use crate::module::ai::stream::stream_runner::{AiStreamRunner, OpenSegment};
use crate::model::resp::resp_agent::{RespAgentBase, RespAgentDelta, RespAgentRunBase};

impl AiStreamRunner {
    pub(in crate::module::ai::stream) async fn handle_reasoning(&self, context: &AiRunContext, pid: String, delta: &str) {
        if !context.emit.reasoning {
            return;
        }
        let Some(active_chat_entry) = self.chat_session_cache_manager.get(&context.req_id) else {
            return;
        };
        let ai_chat_id = active_chat_entry.context.ai_chat_id.clone();
        let resp_agent_reasoning =
            RespEventAgent::OnReasoning(RespAgentDelta { delta: delta.to_string(), run: RespAgentRunBase { run_id: context.run_id.clone() }, base: RespAgentBase::new(context.req_id.clone()) });
        self.agent_service.handler_agent_event(resp_agent_reasoning, ai_chat_id, None, Some(pid), None);
    }

    pub(in crate::module::ai::stream) async fn handle_reasoning_final(&self, context: &AiRunContext, segment: OpenSegment) {
        if !context.emit.reasoning || segment.content.trim().is_empty() {
            return;
        }
        let Some(active_chat_entry) = self.chat_session_cache_manager.get(&context.req_id) else {
            return;
        };
        let ai_chat_id = active_chat_entry.context.ai_chat_id.clone();
        let resp_agent_reasoning_final =
            RespEventAgent::OnReasoningFinal(RespAgentDelta { delta: segment.content, base: RespAgentBase::new(context.req_id.clone()), run: RespAgentRunBase { run_id: context.run_id.clone() } });
        self.agent_service.handler_agent_event(resp_agent_reasoning_final, ai_chat_id, Some(segment.id), None, Some(segment.timestamp));
    }
}
