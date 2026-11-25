use std::sync::Arc;
use summer::tracing::warn;
use summer::plugin::Service;
use async_trait::async_trait;
use tokio::sync::Mutex as AsyncMutex;
use crate::web::ws::ws_manager::WsManager;
use crate::base::base_app::{AppContext, BaseApp};
use summer_web::socketioxide::extract::SocketRef;
use crate::service::service_device::DeviceService;
use crate::model::req::req_ws_context::ReqSocketContext;
use crate::model::resp::resp_ws_context::RespSocketContext;
use crate::model::resp::event::resp_event_telnet::RespEventTelnet;
use crate::module::common::context::context_telnet::ContextTelnet;

#[derive(Clone, Service)]
pub struct TelnetService {
    #[inject(component)]
    ws_manager: WsManager,

    #[inject(component)]
    device_service: DeviceService,
}

#[async_trait]
impl BaseApp for TelnetService {
    fn name(&self) -> &'static str {
        "TelnetService"
    }

    async fn start(&self, _ctx: &AppContext) -> anyhow::Result<()> {
        Ok(())
    }
}

impl TelnetService {
    fn update_device_os_type(&self, context: Arc<AsyncMutex<Option<Arc<ContextTelnet>>>>, os_type: i16) {
        let device_service = self.device_service.clone();
        tokio::spawn(async move {
            let device_id = {
                let guard = context.lock().await;
                let Some(context_telnet) = guard.as_ref() else {
                    return;
                };
                let id = context_telnet.device.read().id.clone();
                id
            };
            if let Err(error) = device_service.update_os_type(device_id, os_type).await {
                warn!("[telnet] 更新设备系统类型失败: {}", error);
            }
        });
    }

    pub fn handler_telnet_event(&self, socket: SocketRef, req: Arc<ReqSocketContext>, context: Arc<AsyncMutex<Option<Arc<ContextTelnet>>>>, event: RespEventTelnet) {
        let mut resp = RespSocketContext { r#type: event.to_type(), session_id: req.session_id.clone(), payload: None };
        match event {
            RespEventTelnet::OnOpen => self.ws_manager.send_msg(socket, &resp),
            RespEventTelnet::OnClose => self.ws_manager.send_msg(socket, &resp),
            RespEventTelnet::OnError(err) => {
                resp.payload = Some(serde_json::to_value(err.to_string()).unwrap());
                self.ws_manager.send_msg(socket, &resp);
            }
            RespEventTelnet::OnData(data) => {
                resp.payload = Some(serde_json::to_value(data.clone()).unwrap());
                self.ws_manager.send_msg(socket, &resp);
            }
            RespEventTelnet::OnOpenStep(step) => {
                resp.payload = Some(serde_json::to_value(step).unwrap());
                self.ws_manager.send_msg(socket, &resp);
            }
            RespEventTelnet::OnDeviceOsType(os_type) => {
                self.update_device_os_type(context, os_type);
                resp.payload = Some(serde_json::to_value(os_type).unwrap());
                self.ws_manager.send_msg(socket, &resp);
            }
        }
    }
}
