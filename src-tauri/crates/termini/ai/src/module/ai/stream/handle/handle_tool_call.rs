use rig::message::ToolCall;
use crate::module::ai::dto::AiRunContext;
use crate::module::ai::stream::stream_runner::AiStreamRunner;
use crate::model::resp::event::resp_event_agent::RespEventAgent;
use crate::model::resp::resp_agent::{RespAgentBase, RespAgentRunBase, RespAgentToolCall};

impl AiStreamRunner {
    pub(in crate::module::ai::stream) async fn handle_tool_call(&self, context: &AiRunContext, internal_call_id: &str, tool_call: &ToolCall, id: String) {
        let Some(active_chat_entry) = self.chat_session_cache_manager.get(&context.req_id) else {
            return;
        };
        let tool_id = match active_chat_entry.tools.iter().find(|tool| tool.tool_id().as_str() == tool_call.function.name) {
            Some(tool) => tool.tool_id().as_str().to_string(),
            None => tool_call.function.name.clone(),
        };

        let ai_chat_id = active_chat_entry.context.ai_chat_id.clone();
        drop(active_chat_entry);

        self.chat_session_cache_manager.set_tool_id(&context.req_id, internal_call_id, &tool_id);
        let resp_agent_tool_call = RespEventAgent::OnToolCall(RespAgentToolCall {
            tool_id,
            call_id: internal_call_id.to_string(),
            args: tool_call.function.arguments.clone(),
            base: RespAgentBase::new(context.req_id.clone()),
            run: RespAgentRunBase { run_id: context.run_id.clone() },
        });
        self.agent_service.handler_agent_event(resp_agent_tool_call, ai_chat_id, Some(id), None, None);
    }
}
