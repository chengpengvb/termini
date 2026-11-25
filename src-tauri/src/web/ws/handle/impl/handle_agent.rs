use anyhow::anyhow;
use std::sync::Arc;
use summer::tracing::error;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use termini_core::util::id_factory::uuid;
use crate::web::ws::ws_manager::WsManager;
use crate::module::ai::agent_chat::AgentChat;
use crate::service::service_agent::AgentService;
use crate::base::base_app::{AppContext, BaseApp};
use summer_web::socketioxide::extract::SocketRef;
use crate::web::ws::handle::handle_base::HandleBase;
use crate::model::req::req_chat_context::ConfirmMode;
use crate::model::req::req_ws_event::ReqWsEventAgent;
use crate::module::ai::shared::ChatSessionCacheManager;
use crate::model::req::req_ws_context::ReqSocketContext;
use crate::model::resp::event::resp_event_agent::RespEventAgent;
use crate::model::resp::resp_agent::{RespAgentBase, RespAgentError};
use crate::model::req::event::ws_agent::{AgentAnswerCode, CancelEvent, ConfirmModeEvent, QuestionAnswerEvent, SubscribeEvent, ToolAnswerEvent, ToolCancelEvent};

#[base_app(sort = 902)]
#[derive(Clone, Service)]
pub struct HandleAgent {
    #[inject(component)]
    pub(super) agent_chat: AgentChat,

    #[inject(component)]
    pub(super) ws_manager: WsManager,

    #[inject(component)]
    pub(super) agent_service: AgentService,

    #[inject(component)]
    chat_session_cache_manager: ChatSessionCacheManager,
}

#[async_trait]
impl BaseApp for HandleAgent {
    fn name(&self) -> &'static str {
        "HandleAgent"
    }
    async fn start(&self, _ctx: &AppContext) -> anyhow::Result<()> {
        let ref_self = Arc::new(self.clone());
        self.ws_manager.register(ref_self);
        Ok(())
    }
}

#[async_trait]
impl HandleBase for HandleAgent {
    fn get_name(&self) -> &'static str {
        "HandleAgent"
    }

    fn contains_type(&self, type_code: i16) -> bool {
        ReqWsEventAgent::form_number(type_code).is_some()
    }

    async fn handle_msg(&self, socket: SocketRef, req: Arc<ReqSocketContext>) -> anyhow::Result<()> {
        let req_id = uuid();
        let result = self.dispatch_msg(socket, req_id.clone(), Arc::clone(&req)).await;
        if let Err(error) = &result {
            error!("[ai] dispatch_msg error={:#}", error);
            let Some(chat_session_cache_entry) = self.chat_session_cache_manager.get(&req_id) else {
                return result;
            };
            let ai_chat_id = chat_session_cache_entry.context.ai_chat_id.clone();
            let event = RespEventAgent::OnError(RespAgentError { code: None, error: format!("{:#}", error), base: RespAgentBase::new(req_id.clone()) });
            self.agent_service.handler_agent_event(event, ai_chat_id.to_string(), None, None, None);
        }
        result
    }
}

impl HandleAgent {
    async fn dispatch_msg(&self, socket: SocketRef, req_id: String, req: Arc<ReqSocketContext>) -> anyhow::Result<()> {
        let Some(event) = ReqWsEventAgent::form_number(req.r#type) else {
            return Err(anyhow!("invalid agent event type: {}", req.r#type));
        };
        match event {
            ReqWsEventAgent::Chat => self.handle_chat(req_id.clone(), req).await,
            ReqWsEventAgent::Cancel => self.handle_cancel(req).await,
            ReqWsEventAgent::ToolAnswer => self.handle_tool_answer(req).await,
            ReqWsEventAgent::ToolCancel => self.handle_tool_cancel(req).await,
            ReqWsEventAgent::Subscribe => self.handle_subscribe(socket, req),
            ReqWsEventAgent::Unsubscribe => self.handle_unsubscribe(socket, req),
            ReqWsEventAgent::ConfirmMode => self.handle_confirm_mode(req),
            ReqWsEventAgent::QuestionAnswer => self.handle_question_answer(req).await,
        }
    }

    /// 前端打开某个会话就进房，刷新或多开都只是房间里多一个成员
    fn handle_subscribe(&self, socket: SocketRef, req: Arc<ReqSocketContext>) -> anyhow::Result<()> {
        let ai_chat_id = Self::subscribe_chat_id(req)?;
        self.ws_manager.join(&ai_chat_id, socket);
        Ok(())
    }

    fn handle_unsubscribe(&self, socket: SocketRef, req: Arc<ReqSocketContext>) -> anyhow::Result<()> {
        let ai_chat_id = Self::subscribe_chat_id(req)?;
        self.ws_manager.leave(&ai_chat_id, &socket.id.to_string());
        Ok(())
    }

    fn subscribe_chat_id(req: Arc<ReqSocketContext>) -> anyhow::Result<String> {
        let payload: SubscribeEvent = req.as_payload()?;
        if payload.data.ai_chat_id.trim().is_empty() {
            return Err(anyhow!("agent subscribe ai_chat_id is empty"));
        }
        Ok(payload.data.ai_chat_id)
    }

    async fn handle_chat(&self, req_id: String, req: Arc<ReqSocketContext>) -> anyhow::Result<()> {
        if let Err(error) = self.agent_chat.chat(req_id, req).await {
            error!("[ai] chat error={:#}", error);
        }
        Ok(())
    }

    async fn handle_cancel(&self, req: Arc<ReqSocketContext>) -> anyhow::Result<()> {
        let payload: CancelEvent = req.as_payload()?;
        self.agent_chat.cancel(&payload.data.req_id)?;
        Ok(())
    }

    async fn handle_tool_answer(&self, req: Arc<ReqSocketContext>) -> anyhow::Result<()> {
        let payload: ToolAnswerEvent = req.as_payload()?;
        let code = AgentAnswerCode::from_i16(payload.data.code).ok_or_else(|| anyhow!("unknown tool answer code: {}", payload.data.code))?;
        self.agent_chat.tool_answer(&payload.data.call_id, code).await?;
        Ok(())
    }

    async fn handle_question_answer(&self, req: Arc<ReqSocketContext>) -> anyhow::Result<()> {
        let payload: QuestionAnswerEvent = req.as_payload()?;
        self.agent_chat.question_answer(payload.data).await
    }

    fn handle_confirm_mode(&self, req: Arc<ReqSocketContext>) -> anyhow::Result<()> {
        let payload: ConfirmModeEvent = req.as_payload()?;
        let mode = ConfirmMode::from_i8(payload.data.confirm_mode).ok_or_else(|| anyhow!("unknown confirm mode: {}", payload.data.confirm_mode))?;
        self.agent_chat.set_confirm_mode(&payload.data.ai_chat_id, mode);
        Ok(())
    }

    async fn handle_tool_cancel(&self, req: Arc<ReqSocketContext>) -> anyhow::Result<()> {
        let payload: ToolCancelEvent = req.as_payload()?;
        self.agent_chat.tool_cancel(&payload.data.req_id, &payload.data.run_id, &payload.data.call_id)?;
        Ok(())
    }
}
