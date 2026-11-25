use std::sync::Arc;
use dashmap::DashMap;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use summer::tracing::{info, warn};
use crate::web::ws::ws_manager::WsManager;
use crate::base::base_app::{AppContext, BaseApp};
use crate::model::req::event::ws_sftp::OpenEvent;
use summer_web::socketioxide::extract::SocketRef;
use crate::model::req::req_ws_context::ReqSocketContext;
use crate::model::dto::ssh_identity_maps::SshIdentityMaps;
use crate::model::resp::resp_ws_context::RespSocketContext;
use crate::model::resp::event::resp_event_sftp::RespEventSftp;
use crate::service::service_sftp_identity::SftpIdentityService;

fn create_identity_map() -> Arc<DashMap<String, SshIdentityMaps>> {
    Arc::new(DashMap::new())
}

#[base_app(sort = 903)]
#[derive(Clone, Service)]
pub struct SftpService {
    #[inject(component)]
    ws_manager: WsManager,

    #[inject(component)]
    ssh_identity_service: SftpIdentityService,

    #[inject(func = create_identity_map())]
    device_id_2_identity: Arc<DashMap<String, SshIdentityMaps>>,
}

#[async_trait]
impl BaseApp for SftpService {
    fn name(&self) -> &'static str {
        "SftpService"
    }

    async fn start(&self, _ctx: &AppContext) -> anyhow::Result<()> {
        Ok(())
    }
}

impl SftpService {
    fn handle_identity(&self, req: Arc<ReqSocketContext>) {
        let Some(payload_value) = req.payload.clone() else {
            return;
        };
        let Ok(payload) = serde_json::from_value::<OpenEvent>(payload_value) else {
            return;
        };
        let device_id = payload.data.device_id;
        if self.device_id_2_identity.get(&device_id).is_some() {
            return;
        }
        let self_ref = self.clone();
        tokio::spawn(async move {
            let maps = self_ref.ssh_identity_service.load_identity_maps(device_id.clone()).await;
            if maps.is_empty() {
                warn!("[sftp] 权限身份映射为空 device_id: {}", device_id);
                return;
            }
            info!("[sftp] 权限身份映射加载完成 device_id: {}, uid: {}, gid: {}", device_id, maps.uid_to_name.len(), maps.gid_to_name.len());
            self_ref.device_id_2_identity.insert(device_id, maps);
        });
    }

    pub fn get_identity_maps(&self, device_id: &str) -> Option<SshIdentityMaps> {
        self.device_id_2_identity.get(device_id).map(|entry| entry.value().clone())
    }

    pub fn handler_sftp_event(&self, socket: SocketRef, req: Arc<ReqSocketContext>, event: RespEventSftp) {
        let mut resp = RespSocketContext { r#type: event.to_type(), session_id: req.session_id.clone(), payload: None };
        match event {
            RespEventSftp::OnOpen(base) => {
                self.handle_identity(Arc::clone(&req));
                resp.payload = Some(serde_json::to_value(base).unwrap());
                self.ws_manager.send_msg(socket, &resp);
            }
            other => {
                resp.payload = Some(other.to_payload());
                self.ws_manager.send_msg(socket, &resp);
            }
        }
    }
}
