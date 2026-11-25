use std::sync::Arc;
use summer::plugin::Service;
use async_trait::async_trait;
use crate::web::ws::ws_manager::WsManager;
use crate::base::base_app::{AppContext, BaseApp};
use summer_web::socketioxide::extract::SocketRef;
use crate::model::req::req_ws_context::ReqSocketContext;
use crate::model::resp::resp_ws_context::RespSocketContext;
use crate::model::resp::event::resp_event_rdp::RespEventRdp;
use crate::model::resp::resp_ws_bin_context::RespSocketBinContext;

#[derive(Clone, Service)]
pub struct RdpService {
    #[inject(component)]
    ws_manager: WsManager,
}

#[async_trait]
impl BaseApp for RdpService {
    fn name(&self) -> &'static str {
        "RdpService"
    }

    async fn start(&self, _ctx: &AppContext) -> anyhow::Result<()> {
        Ok(())
    }
}

impl RdpService {
    pub fn handler_rdp_event(&self, socket: SocketRef, req: Arc<ReqSocketContext>, event: RespEventRdp) {
        let r#type = event.to_type();
        let session_id = req.session_id.clone();
        match event {
            RespEventRdp::OnFrame(payload) => {
                self.ws_manager.send_bin(socket, &RespSocketBinContext { r#type, session_id, payload });
            }
            RespEventRdp::OnCursor(payload) => {
                self.ws_manager.send_bin(socket, &RespSocketBinContext { r#type, session_id, payload });
            }
            RespEventRdp::OnOpen => {
                self.ws_manager.send_msg(socket, &RespSocketContext { r#type, session_id, payload: None });
            }
            RespEventRdp::OnClose => {
                self.ws_manager.send_msg(socket, &RespSocketContext { r#type, session_id, payload: None });
            }
            RespEventRdp::OnError(error) => {
                let payload = Some(serde_json::to_value(error).unwrap());
                self.ws_manager.send_msg(socket, &RespSocketContext { r#type, session_id, payload });
            }
            RespEventRdp::OnOpenStep(step) => {
                let payload = Some(serde_json::to_value(step).unwrap());
                self.ws_manager.send_msg(socket, &RespSocketContext { r#type, session_id, payload });
            }
            RespEventRdp::OnDesktopResize(size) => {
                let payload = Some(serde_json::to_value(size).unwrap());
                self.ws_manager.send_msg(socket, &RespSocketContext { r#type, session_id, payload });
            }
            RespEventRdp::OnNetwork(bars) => {
                let payload = Some(serde_json::to_value(bars).unwrap());
                self.ws_manager.send_msg(socket, &RespSocketContext { r#type, session_id, payload });
            }
        }
    }
}
