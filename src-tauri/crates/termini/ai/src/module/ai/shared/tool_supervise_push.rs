use summer::tracing::warn;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use crate::base::base_app::BaseApp;
use crate::module::ai::dto::AiRunContext;
use crate::service::service_agent::AgentService;
use crate::module::ai::dto::ShellSuperviseDecision;
use crate::module::ai::shared::ChatSessionCacheManager;
use crate::model::resp::event::resp_event_agent::RespEventAgent;
use crate::model::resp::resp_agent::{RespAgentBase, RespAgentRunBase, RespAgentToolSupervise};

#[base_app(sort = 761)]
#[derive(Clone, Service)]
pub struct ToolSupervisePush {
    #[inject(component)]
    agent_service: AgentService,

    #[inject(component)]
    chat_session_cache_manager: ChatSessionCacheManager,
}

#[async_trait]
impl BaseApp for ToolSupervisePush {
    fn name(&self) -> &'static str {
        "ToolSupervisePush"
    }
}

impl ToolSupervisePush {
    pub fn push_tool_supervise_event(&self, context: &AiRunContext, call_id: &str, tool_id: &str, round: usize, decision: &ShellSuperviseDecision) {
        let Some(chat_session_cache_entry) = self.chat_session_cache_manager.get(&context.req_id) else {
            warn!("[ai] 未知 req_id: {}", context.req_id);
            return;
        };
        let ai_chat_id = chat_session_cache_entry.context.ai_chat_id.clone();
        drop(chat_session_cache_entry);

        let pid = self.chat_session_cache_manager.get_tool_item_id(&context.req_id, call_id);
        let event = RespEventAgent::OnToolSupervise(RespAgentToolSupervise {
            round,
            call_id: call_id.to_string(),
            tool_id: tool_id.to_string(),
            action: decision.action.as_i16(),
            reason: decision.reason.clone(),
            wait_seconds: decision.wait_seconds,
            base: RespAgentBase::new(context.req_id.clone()),
            run: RespAgentRunBase { run_id: context.run_id.clone() },
        });
        self.agent_service.handler_agent_event(event, ai_chat_id.to_string(), None, pid, None);
    }
}
