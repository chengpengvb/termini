use std::sync::Arc;
use dashmap::DashMap;
use summer::plugin::Service;
use async_trait::async_trait;
use summer::tracing::{info, warn};
use crate::web::ws::ws_manager::WsManager;
use crate::model::req::event::ws_ssh::OpenEvent;
use crate::base::base_app::{AppContext, BaseApp};
use summer_web::socketioxide::extract::SocketRef;
use crate::service::service_device::DeviceService;
use crate::model::req::req_ws_context::ReqSocketContext;
use crate::model::resp::resp_ws_context::RespSocketContext;
use crate::service::service_ssh_history::SshHistoryService;
use crate::model::resp::event::resp_event_ssh::RespEventSsh;

fn create_history_map() -> Arc<DashMap<String, Vec<String>>> {
    Arc::new(DashMap::new())
}

#[derive(Clone, Service)]
pub struct SshService {
    #[inject(component)]
    ws_manager: WsManager,

    #[inject(component)]
    device_service: DeviceService,

    #[inject(component)]
    ssh_history_service: SshHistoryService,

    #[inject(func = create_history_map())]
    device_id_2_history: Arc<DashMap<String, Vec<String>>>,
}

#[async_trait]
impl BaseApp for SshService {
    fn name(&self) -> &'static str {
        "SshService"
    }

    async fn start(&self, _ctx: &AppContext) -> anyhow::Result<()> {
        Ok(())
    }
}

impl SshService {
    fn resolve_device_id(req: &ReqSocketContext) -> Option<String> {
        let payload_value = req.payload.clone()?;
        let payload = serde_json::from_value::<OpenEvent>(payload_value).ok()?;
        Some(payload.data.device_id)
    }

    async fn update_device_os_type(&self, req: &ReqSocketContext, os_type: i16) {
        let Some(device_id) = Self::resolve_device_id(req) else {
            return;
        };
        if let Err(error) = self.device_service.update_os_type(device_id, os_type).await {
            warn!("[ssh] 更新设备系统类型失败: {}", error);
        }
    }

    async fn handle_history(&self, req: Arc<ReqSocketContext>) {
        let Some(device_id) = Self::resolve_device_id(&req) else {
            return;
        };
        if self.device_id_2_history.get(&device_id).is_some() {
            return;
        }
        let history = self.ssh_history_service.get_cmd_history(device_id.clone()).await;
        if history.is_empty() {
            warn!("[ssh] 获取历史命令为空 device_id: {}", device_id);
            return;
        }
        info!("[ssh] 历史命令加载完成 device_id: {}, 数量: {}", device_id, history.len());
        self.device_id_2_history.insert(device_id, history.clone());
    }

    pub fn get_history(&self, device_id: &str) -> Option<Vec<String>> {
        self.device_id_2_history.get(device_id).map(|entry| entry.value().clone())
    }

    pub async fn handler_ssh_event(&self, socket: SocketRef, req: Arc<ReqSocketContext>, event: RespEventSsh) {
        let mut resp = RespSocketContext { r#type: event.to_type(), session_id: req.session_id.clone(), payload: None };
        match event {
            RespEventSsh::OnOpen => {
                self.ws_manager.send_msg(socket, &resp);
                let service = self.clone();
                tokio::spawn(async move {
                    service.handle_history(req).await;
                });
            }
            RespEventSsh::OnClose => self.ws_manager.send_msg(socket, &resp),
            RespEventSsh::OnError(err) => {
                resp.payload = Some(serde_json::to_value(err.to_string()).unwrap());
                self.ws_manager.send_msg(socket, &resp);
            }
            RespEventSsh::OnData(data) => {
                resp.payload = Some(serde_json::to_value(data.clone()).unwrap());
                self.ws_manager.send_msg(socket, &resp);
            }
            RespEventSsh::OnOpenStep(step) => {
                resp.payload = Some(serde_json::to_value(step).unwrap());
                self.ws_manager.send_msg(socket, &resp);
            }
            RespEventSsh::OnDeviceOsType(os_type) => {
                self.update_device_os_type(&req, os_type).await;
                resp.payload = Some(serde_json::to_value(os_type).unwrap());
                self.ws_manager.send_msg(socket, &resp);
            }
        }
    }
}
