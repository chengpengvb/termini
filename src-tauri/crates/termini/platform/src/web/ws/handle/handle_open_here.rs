use anyhow::Result;
use std::sync::Arc;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use crate::module::open_here::OpenHere;
use crate::web::ws::ws_manager::WsManager;
use crate::base::base_app::{AppContext, BaseApp};
use summer_web::socketioxide::extract::SocketRef;
use crate::web::ws::handle::handle_base::HandleBase;
use crate::model::req::req_ws_context::ReqSocketContext;
use crate::model::resp::event::resp_event_window::RespEventWindow;

#[base_app(sort = 300)]
#[derive(Clone, Service)]
pub struct HandleOpenHere {
    #[inject(component)]
    ws_manager: WsManager,
}

#[async_trait]
impl BaseApp for HandleOpenHere {
    fn name(&self) -> &'static str {
        "HandleOpenHere"
    }

    async fn start(&self, _ctx: &AppContext) -> Result<()> {
        let ref_self = Arc::new(self.clone());
        self.ws_manager.register(ref_self);
        Ok(())
    }
}

#[async_trait]
impl HandleBase for HandleOpenHere {
    fn get_name(&self) -> &'static str {
        "HandleOpenHere"
    }

    fn contains_type(&self, _type_code: i16) -> bool {
        false
    }

    async fn handle_open(&self, socket: SocketRef) -> Result<()> {
        for item in OpenHere::snapshot() {
            self.ws_manager.send_msg(socket.clone(), &RespEventWindow::OnOpenHere { path: item.path, seq: item.seq }.to_resp());
        }
        Ok(())
    }

    async fn handle_msg(&self, _socket: SocketRef, _req: Arc<ReqSocketContext>) -> Result<()> {
        Ok(())
    }
}
