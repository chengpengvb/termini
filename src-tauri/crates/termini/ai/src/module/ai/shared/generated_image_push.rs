use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use crate::base::base_app::BaseApp;
use crate::module::ai::dto::AiRunContext;
use crate::service::service_agent::AgentService;
use crate::module::ai::shared::ChatSessionCacheManager;
use crate::module::ai::shared::generated_image::GeneratedImage;
use crate::model::resp::event::resp_event_agent::RespEventAgent;
use crate::model::resp::resp_agent::{RespAgentBase, RespAgentImage, RespAgentRunBase};

#[base_app(sort = 764)]
#[derive(Clone, Service)]
pub struct GeneratedImagePush {
    #[inject(component)]
    agent_service: AgentService,

    #[inject(component)]
    chat_session_cache_manager: ChatSessionCacheManager,
}

#[async_trait]
impl BaseApp for GeneratedImagePush {
    fn name(&self) -> &'static str {
        "GeneratedImagePush"
    }
}

impl GeneratedImagePush {
    pub fn push(&self, context: &AiRunContext, image: &GeneratedImage) {
        if !context.emit.text {
            return;
        }
        let Some(active_chat_entry) = self.chat_session_cache_manager.get(&context.req_id) else {
            return;
        };
        let ai_chat_id = active_chat_entry.context.ai_chat_id.clone();
        drop(active_chat_entry);
        let path = image.path.clone();
        let media_type = image.media_type.clone();
        let base = RespAgentBase::new(context.req_id.clone());
        let run = RespAgentRunBase { run_id: context.run_id.clone() };
        let event = RespEventAgent::OnImage(RespAgentImage { path, media_type, run, base });
        self.agent_service.handler_agent_event(event, ai_chat_id, None, None, None);
    }
}
