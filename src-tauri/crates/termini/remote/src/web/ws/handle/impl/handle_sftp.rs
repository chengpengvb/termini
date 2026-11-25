use anyhow::anyhow;
use std::sync::Arc;
use dashmap::DashMap;
use summer::tracing::warn;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use tokio::sync::Mutex as AsyncMutex;
use crate::web::ws::ws_manager::WsManager;
use summer::plugin::component::ComponentRef;
use crate::service::service_sftp::SftpService;
use crate::base::base_app::{AppContext, BaseApp};
use crate::model::resp::resp_sftp::RespSftpError;
use summer_web::socketioxide::extract::SocketRef;
use crate::service::service_device::DeviceService;
use crate::model::req::req_ws_event::ReqWsEventSftp;
use crate::web::ws::handle::handle_base::HandleBase;
use crate::model::dto::cancel_session::CancelSession;
use super::sftp::{RemoteSessionClient, SessionClient};
use crate::model::req::req_ws_context::ReqSocketContext;
use crate::module::sftp::client_transfer::ClientTransfer;
use crate::service::service_config_ftp::ConfigFtpService;
use crate::service::service_config_ssh::ConfigSshService;
use crate::model::resp::event::resp_event_sftp::RespEventSftp;
use crate::module::common::dto::file_edit_session::FileEditSession;
use crate::module::sftp::client_transfer_local::ClientTransferLocal;
use crate::module::ssh::client_session_factory::ClientSessionFactory;
use crate::module::common::dto::sftp_transfer_answer::SftpTransferInbox;

fn create_socket_refs() -> Arc<DashMap<String, SocketRef>> {
    Arc::new(DashMap::new())
}

fn create_client_transfer_local() -> Arc<ClientTransferLocal> {
    Arc::new(ClientTransferLocal::new())
}

fn create_file_edit_sessions() -> Arc<DashMap<String, Arc<DashMap<String, Arc<DashMap<String, FileEditSession>>>>>> {
    Arc::new(DashMap::new())
}

fn create_socket_id_2_session_locks() -> Arc<DashMap<String, Arc<DashMap<String, Arc<AsyncMutex<()>>>>>> {
    Arc::new(DashMap::new())
}

fn create_socket_id_2_remote_clients() -> Arc<DashMap<String, Arc<DashMap<String, RemoteSessionClient>>>> {
    Arc::new(DashMap::new())
}

fn create_socket_id_2_session_id_2_client_transfer_id() -> Arc<DashMap<String, Arc<DashMap<String, String>>>> {
    Arc::new(DashMap::new())
}

fn create_transfer_sessions() -> Arc<DashMap<String, Arc<DashMap<String, Arc<DashMap<String, Arc<CancelSession>>>>>>> {
    Arc::new(DashMap::new())
}

fn create_delete_sessions() -> Arc<DashMap<String, Arc<DashMap<String, Arc<DashMap<String, Arc<CancelSession>>>>>>> {
    Arc::new(DashMap::new())
}

fn create_transfer_answer_requests() -> Arc<DashMap<String, Arc<DashMap<String, Arc<DashMap<String, Arc<SftpTransferInbox>>>>>>> {
    Arc::new(DashMap::new())
}

#[base_app(sort = 902)]
#[derive(Clone, Service)]
pub struct HandleSftp {
    #[inject(component)]
    pub(super) ws_manager: WsManager,

    #[inject(component)]
    pub(super) sftp_service: SftpService,

    #[inject(component)]
    pub(super) device_service: DeviceService,

    #[inject(component)]
    pub(super) config_ssh_service: ConfigSshService,

    #[inject(component)]
    pub(super) config_ftp_service: ConfigFtpService,

    #[inject(func = create_socket_refs())]
    pub(super) socket_refs: Arc<DashMap<String, SocketRef>>,

    #[inject(func = create_client_transfer_local())]
    pub(super) client_transfer_local: Arc<ClientTransferLocal>,

    pub(super) client_session_factory: ComponentRef<ClientSessionFactory>,

    #[inject(func = create_socket_id_2_session_locks())]
    pub(super) socket_id_2_session_locks: Arc<DashMap<String, Arc<DashMap<String, Arc<AsyncMutex<()>>>>>>,

    #[inject(func = create_socket_id_2_remote_clients())]
    pub(super) socket_id_2_remote_clients: Arc<DashMap<String, Arc<DashMap<String, RemoteSessionClient>>>>,

    #[inject(func = create_socket_id_2_session_id_2_client_transfer_id())]
    pub(super) socket_id_2_session_id_2_client_transfer_id: Arc<DashMap<String, Arc<DashMap<String, String>>>>,

    #[inject(func = create_file_edit_sessions())]
    pub(super) file_edit_sessions: Arc<DashMap<String, Arc<DashMap<String, Arc<DashMap<String, FileEditSession>>>>>>, // socket_id -> session_id -> edit_id -> FileEditSession

    #[inject(func = create_delete_sessions())]
    pub(super) delete_sessions: Arc<DashMap<String, Arc<DashMap<String, Arc<DashMap<String, Arc<CancelSession>>>>>>>, // socket_id -> session_id -> resp_id -> CancelSession

    #[inject(func = create_transfer_sessions())]
    pub(super) transfer_sessions: Arc<DashMap<String, Arc<DashMap<String, Arc<DashMap<String, Arc<CancelSession>>>>>>>, // socket_id -> session_id -> resp_id-> CancelSession

    #[inject(func = create_transfer_answer_requests())]
    pub(super) transfer_answer_requests: Arc<DashMap<String, Arc<DashMap<String, Arc<DashMap<String, Arc<SftpTransferInbox>>>>>>>, // socket_id -> session_id -> request_id -> SftpTransferInbox
}

#[async_trait]
impl BaseApp for HandleSftp {
    fn name(&self) -> &'static str {
        "HandleSftp"
    }
    async fn start(&self, _ctx: &AppContext) -> anyhow::Result<()> {
        let ref_self = Arc::new(self.clone());
        self.ws_manager.register(ref_self);
        Self::cleanup_sftp_edit_dir_on_start();
        Self::cleanup_sftp_export_dir_on_start();
        Ok(())
    }
}

#[async_trait]
impl HandleBase for HandleSftp {
    fn get_name(&self) -> &'static str {
        "HandleSftp"
    }

    fn contains_type(&self, type_code: i16) -> bool {
        ReqWsEventSftp::form_number(type_code).is_some()
    }

    async fn handle_open(&self, socket: SocketRef) -> anyhow::Result<()> {
        self.socket_refs.insert(socket.id.to_string(), socket);
        Ok(())
    }

    async fn handle_close(&self, socket: SocketRef) -> anyhow::Result<()> {
        self.cleanup_sftp_socket(&socket.id.to_string()).await;
        Ok(())
    }

    async fn handle_msg(&self, socket: SocketRef, req: Arc<ReqSocketContext>) -> anyhow::Result<()> {
        let socket_for_error = socket.clone();
        let req_for_error = Arc::clone(&req);
        let result = self.dispatch_msg(socket, Arc::clone(&req)).await;
        if let Err(error) = &result {
            let client_transfer_id = req_for_error.payload.as_ref().and_then(|v| v.get("data")).and_then(|v| v.get("clientTransferId")).and_then(|v| v.as_str()).map(String::from).unwrap_or_default();
            let resp = RespSftpError { client_transfer_id, message: format!("{:#}", error) };
            self.sftp_service.handler_sftp_event(socket_for_error, req, RespEventSftp::OnError(resp));
        }
        result
    }
}

impl HandleSftp {
    async fn dispatch_msg(&self, socket: SocketRef, req: Arc<ReqSocketContext>) -> anyhow::Result<()> {
        let Some(event) = ReqWsEventSftp::form_number(req.r#type) else {
            return Err(anyhow!("invalid sftp event type: {}", req.r#type));
        };
        match event {
            ReqWsEventSftp::Open => self.handle_open(socket, req).await, //需要释放
            ReqWsEventSftp::Stat => self.handle_stat(socket, req).await,
            ReqWsEventSftp::Chmod => self.handle_chmod(socket, req).await,
            ReqWsEventSftp::Rename => self.handle_rename(socket, req).await,
            ReqWsEventSftp::Create => self.handle_create(socket, req).await,
            ReqWsEventSftp::Delete => self.handle_delete(socket, req).await, //需要释放
            ReqWsEventSftp::HomeDir => self.handle_home_dir(socket, req).await,
            ReqWsEventSftp::ListDir => self.handle_list_dir(socket, req).await,
            ReqWsEventSftp::Transfer => self.handle_transfer(socket, req).await, //需要释放
            ReqWsEventSftp::DeleteCancel => self.handle_delete_cancel(socket, req).await,
            ReqWsEventSftp::FileOpen => self.handle_file_open(socket, req).await, //需要释放
            ReqWsEventSftp::Close => self.handle_close_session(socket, req).await,
            ReqWsEventSftp::TransferAnswer => self.handle_transfer_answer(socket, req).await,
            ReqWsEventSftp::ExportToTemp => self.handle_export_to_temp(socket, req).await,
            ReqWsEventSftp::TransferCancel => self.handle_transfer_cancel(socket, req).await,
            ReqWsEventSftp::StatProperties => self.handle_stat_properties(socket, req).await,
        }
    }

    pub(super) fn resolve_client(&self, socket_id: &str, client_transfer_id: &str) -> anyhow::Result<SessionClient> {
        if client_transfer_id == self.client_transfer_local.get_id() {
            return Ok(SessionClient::Local(Arc::clone(&self.client_transfer_local)));
        }
        let client = self
            .socket_id_2_remote_clients
            .get(socket_id)
            .and_then(|m| m.get(client_transfer_id).map(|c| c.value().clone()))
            .ok_or_else(|| anyhow!("sftp client not found: socket={} client_transfer_id={}", socket_id, client_transfer_id))?;
        Ok(SessionClient::Remote(client))
    }

    pub fn resolve_socket_by_session(&self, session_id: &str) -> anyhow::Result<SocketRef> {
        let socket_id = self.resolve_socket_id_by_session(session_id).ok_or_else(|| anyhow!("socket not found for session: {}", session_id))?;
        self.socket_refs.get(&socket_id).map(|entry| entry.value().clone()).ok_or_else(|| anyhow!("socket not found for socket_id: {}", socket_id))
    }

    fn resolve_socket_id_by_session(&self, session_id: &str) -> Option<String> {
        self.socket_id_2_session_id_2_client_transfer_id.iter().find(|entry| entry.value().contains_key(session_id)).map(|entry| entry.key().clone())
    }

    pub fn local_client_transfer_id(&self) -> String {
        self.client_transfer_local.get_id()
    }

    pub fn local_session_client(&self) -> SessionClient {
        SessionClient::Local(Arc::clone(&self.client_transfer_local))
    }

    pub(super) async fn close_sftp_remote_client(&self, socket_id: &str, session_id: &str, client_transfer_id: &str) {
        let Some(remote_map) = self.socket_id_2_remote_clients.get(socket_id) else {
            return;
        };
        let Some(client) = remote_map.get(client_transfer_id).map(|entry| entry.value().clone()) else {
            return;
        };
        if let Err(error) = client.close().await {
            warn!("[sftp] Warning: failed to close sftp client session {}: {}", session_id, error);
        }
        remote_map.remove(client_transfer_id);
        if remote_map.is_empty() {
            drop(remote_map);
            self.socket_id_2_remote_clients.remove(socket_id);
        }
    }
}
