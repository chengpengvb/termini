use anyhow::anyhow;
use std::sync::Arc;
use summer::tracing::info;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use crate::web::ws::ws_manager::WsManager;
use summer::plugin::component::ComponentRef;
use crate::base::base_app::{AppContext, BaseApp};
use summer_web::socketioxide::extract::SocketRef;
use crate::web::ws::handle::handle_base::HandleBase;
use crate::model::req::event::ws_ssh::AuthAnswerEvent;
use crate::model::req::req_ws_event::ReqWsEventSshAuth;
use crate::model::req::req_ws_context::ReqSocketContext;
use crate::module::common::dto::ssh_auth_input::SshAuthAnswer;
use crate::module::ssh::client_session_factory::ClientSessionFactory;

#[base_app(sort = 905)]
#[derive(Clone, Service)]
pub struct HandleSshAuth {
    #[inject(component)]
    ws_manager: WsManager,

    client_session_factory: ComponentRef<ClientSessionFactory>,
}

#[async_trait]
impl BaseApp for HandleSshAuth {
    fn name(&self) -> &'static str {
        "HandleSshAuth"
    }
    async fn start(&self, _ctx: &AppContext) -> anyhow::Result<()> {
        let ref_self = Arc::new(self.clone());
        self.ws_manager.register(ref_self);
        Ok(())
    }
}

#[async_trait]
impl HandleBase for HandleSshAuth {
    fn get_name(&self) -> &'static str {
        "HandleSshAuth"
    }

    fn contains_type(&self, type_code: i16) -> bool {
        ReqWsEventSshAuth::form_number(type_code).is_some()
    }

    async fn handle_msg(&self, _socket: SocketRef, req: Arc<ReqSocketContext>) -> anyhow::Result<()> {
        let Some(event) = ReqWsEventSshAuth::form_number(req.r#type) else {
            return Err(anyhow!("invalid event type: {}", req.r#type));
        };
        match event {
            ReqWsEventSshAuth::AuthAnswer => {
                self.handle_auth_answer(req).await?;
            }
        }
        Ok(())
    }
}

impl HandleSshAuth {
    async fn handle_auth_answer(&self, req: Arc<ReqSocketContext>) -> anyhow::Result<()> {
        let payload: AuthAnswerEvent = req.as_payload()?;
        let data = payload.data;
        let answer: SshAuthAnswer = SshAuthAnswer::from_code(data.code).ok_or_else(|| anyhow!("ssh auth answer code invalid: {}", data.code))?;
        info!("[ssh] 收到 SSH 认证交互的入站答案：request_id={}, answer={:?}", data.request_id, answer);
        self.client_session_factory.notify_auth_updated(data.request_id, answer).await?;
        Ok(())
    }
}
