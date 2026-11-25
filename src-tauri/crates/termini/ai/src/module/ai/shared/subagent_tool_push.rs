use anyhow::{bail, Result};
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use crate::base::base_app::BaseApp;
use crate::module::ai::dto::AiRunContext;
use crate::service::service_agent::AgentService;
use crate::module::ai::shared::ChatSessionCacheManager;
use crate::model::resp::event::resp_event_agent::RespEventAgent;
use crate::model::resp::resp_agent::{RespAgentBase, RespAgentRunBase, RespAgentSubagentOpen};

#[base_app(sort = 765)]
#[derive(Clone, Service)]
pub struct SubagentToolPush {
    #[inject(component)]
    agent_service: AgentService,

    #[inject(component)]
    chat_session_cache_manager: ChatSessionCacheManager,
}

#[async_trait]
impl BaseApp for SubagentToolPush {
    fn name(&self) -> &'static str {
        "SubagentToolPush"
    }
}

impl SubagentToolPush {
    pub fn push_open(&self, context: &AiRunContext, call_id: &str, sub_req_id: &str, title: Option<String>) -> Result<()> {
        let Some(entry) = self.chat_session_cache_manager.get(&context.req_id) else {
            bail!("unknown req_id: {}", context.req_id);
        };
        let ai_chat_id = entry.context.ai_chat_id.clone();
        drop(entry);
        let pid = self.chat_session_cache_manager.get_tool_item_id(&context.req_id, call_id);
        let event = RespEventAgent::OnSubagentOpen(RespAgentSubagentOpen {
            title,
            call_id: call_id.to_string(),
            sub_req_id: sub_req_id.to_string(),
            base: RespAgentBase::new(context.req_id.clone()),
            run: RespAgentRunBase { run_id: context.run_id.clone() },
        });
        self.agent_service.handler_agent_event(event, ai_chat_id, None, pid, None);
        Ok(())
    }
}
