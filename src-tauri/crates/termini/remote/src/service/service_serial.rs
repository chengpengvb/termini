use std::sync::Arc;
use summer::plugin::Service;
use async_trait::async_trait;
use crate::web::ws::ws_manager::WsManager;
use crate::base::base_app::{AppContext, BaseApp};
use summer_web::socketioxide::extract::SocketRef;
use crate::model::req::req_ws_context::ReqSocketContext;
use crate::model::resp::resp_ws_context::RespSocketContext;
use crate::model::resp::event::resp_event_serial::RespEventSerial;

#[derive(Clone, Service)]
pub struct SerialService {
    #[inject(component)]
    ws_manager: WsManager,
}

#[async_trait]
impl BaseApp for SerialService {
    fn name(&self) -> &'static str {
        "SerialService"
    }

    async fn start(&self, _ctx: &AppContext) -> anyhow::Result<()> {
        Ok(())
    }
}

impl SerialService {
    pub fn handler_serial_event(&self, socket: SocketRef, req: Arc<ReqSocketContext>, event: RespEventSerial) {
        let mut resp = RespSocketContext { r#type: event.to_type(), session_id: req.session_id.clone(), payload: None };
        match event {
            RespEventSerial::OnOpen => self.ws_manager.send_msg(socket, &resp),
            RespEventSerial::OnClose => self.ws_manager.send_msg(socket, &resp),
            RespEventSerial::OnError(err) => {
                resp.payload = Some(serde_json::to_value(err.to_string()).unwrap());
                self.ws_manager.send_msg(socket, &resp);
            }
            RespEventSerial::OnData(data) => {
                resp.payload = Some(serde_json::to_value(data.clone()).unwrap());
                self.ws_manager.send_msg(socket, &resp);
            }
        }
    }
}
