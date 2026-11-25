use anyhow::anyhow;
use std::sync::Arc;
use dashmap::DashMap;
use summer::tracing::info;
use tokio::sync::Mutex as AsyncMutex;
use crate::model::device::DEVICE_LOCAL_ID;
use super::super::handle_sftp::HandleSftp;
use crate::model::config_ftp::PROTOCOL_FTP;
use crate::service::service_base::BaseService;
use crate::model::resp::resp_sftp::RespSftpBase;
use crate::model::req::event::ws_sftp::OpenEvent;
use summer_web::socketioxide::extract::SocketRef;
use crate::model::req::req_ws_context::ReqSocketContext;
use crate::module::common::client_terminal::EventHandler;
use crate::module::sftp::client_transfer::ClientTransfer;
use crate::module::common::context::context_ftp::ContextFtp;
use crate::module::common::context::context_ssh::ContextSsh;
use crate::module::common::context::context_base::WindowSize;
use crate::model::resp::event::resp_event_sftp::RespEventSftp;
use crate::module::sftp::client_transfer_ftp::ClientTransferFtp;
use crate::module::sftp::client_transfer_sftp::ClientTransferSftp;
use super::session_client::{ClientTransferFtpDyn, ClientTransferSftpDyn, RemoteSessionClient};

impl HandleSftp {
    pub(in super::super) async fn handle_open(&self, socket: SocketRef, req: Arc<ReqSocketContext>) -> anyhow::Result<()> {
        let session_id = req.session_id.clone();
        if session_id.is_empty() {
            return Err(anyhow!("sftp open req session_id is empty"));
        }
        info!("[sftp] open sftp session_id: {}", session_id);
        let payload: OpenEvent = req.as_payload()?;
        let device_id = payload.data.device_id.clone();
        if device_id.is_empty() {
            return Err(anyhow!("sftp open req device_id is empty"));
        }
        if DEVICE_LOCAL_ID == device_id {
            let client_transfer_id = self.client_transfer_local.get_id();
            self.sftp_service.handler_sftp_event(socket, Arc::clone(&req), RespEventSftp::OnOpen(RespSftpBase::new(String::new(), client_transfer_id.clone())));
            return Ok(());
        }
        if payload.data.protocol == PROTOCOL_FTP {
            return self.open_ftp_session(socket, req, session_id, device_id).await;
        }
        self.open_remote_session(socket, req, session_id, device_id).await
    }

    async fn open_remote_session(&self, socket: SocketRef, req: Arc<ReqSocketContext>, session_id: String, device_id: String) -> anyhow::Result<()> {
        let socket_id = socket.id.to_string();
        let session_lock_map = self.socket_id_2_session_locks.entry(socket_id.clone()).or_insert_with(|| Arc::new(DashMap::new())).clone();
        let session_lock = session_lock_map.entry(session_id.clone()).or_insert_with(|| Arc::new(AsyncMutex::new(()))).clone();
        let _guard = session_lock.lock().await;

        let device = self.device_service.find_by_id(device_id.clone()).await?.ok_or_else(|| anyhow!("device not found for device_id: {}", device_id))?;
        let config = self.config_ssh_service.get_by_device_id(device_id.clone()).ok_or_else(|| anyhow!("sftp open config_ssh not found for device_id={}", device_id))?;
        let context_ssh = ContextSsh { size: WindowSize::default(), config, device };

        let client: Arc<ClientTransferSftpDyn> = Arc::new(ClientTransferSftp::new(session_id.clone(), self.client_session_factory.clone()));
        let client_transfer_id = client.get_id();
        self.store_remote_client(&socket_id, &session_id, client_transfer_id.clone(), RemoteSessionClient::Sftp(Arc::clone(&client))).await;

        match self.connect_sftp_client(socket.clone(), session_id.clone(), Arc::clone(&req), client, context_ssh).await {
            Ok(()) => {
                drop(_guard);
                self.sftp_service.handler_sftp_event(socket, Arc::clone(&req), RespEventSftp::OnOpen(RespSftpBase::new(String::new(), client_transfer_id)));
                Ok(())
            }
            Err(error) => {
                drop(_guard);
                self.cleanup_sftp_client(&socket_id, Arc::clone(&req), false).await;
                Err(error)
            }
        }
    }

    async fn open_ftp_session(&self, socket: SocketRef, req: Arc<ReqSocketContext>, session_id: String, device_id: String) -> anyhow::Result<()> {
        let socket_id = socket.id.to_string();
        let session_lock_map = self.socket_id_2_session_locks.entry(socket_id.clone()).or_insert_with(|| Arc::new(DashMap::new())).clone();
        let session_lock = session_lock_map.entry(session_id.clone()).or_insert_with(|| Arc::new(AsyncMutex::new(()))).clone();
        let _guard = session_lock.lock().await;

        let device = self.device_service.find_by_id(device_id.clone()).await?.ok_or_else(|| anyhow!("device not found for device_id: {}", device_id))?;
        let config = self.config_ftp_service.get_by_device_id(device_id.clone()).ok_or_else(|| anyhow!("ftp open config_ftp not found for device_id={}", device_id))?;
        let context_ftp = ContextFtp { size: WindowSize::default(), config, device };

        let client: Arc<ClientTransferFtpDyn> = Arc::new(ClientTransferFtp::new());
        let client_transfer_id = client.get_id();
        self.store_remote_client(&socket_id, &session_id, client_transfer_id.clone(), RemoteSessionClient::Ftp(Arc::clone(&client))).await;

        match self.connect_ftp_client(socket.clone(), Arc::clone(&req), client, context_ftp).await {
            Ok(()) => {
                drop(_guard);
                self.sftp_service.handler_sftp_event(socket, Arc::clone(&req), RespEventSftp::OnOpen(RespSftpBase::new(String::new(), client_transfer_id)));
                Ok(())
            }
            Err(error) => {
                drop(_guard);
                self.cleanup_sftp_client(&socket_id, Arc::clone(&req), false).await;
                Err(error)
            }
        }
    }

    async fn store_remote_client(&self, socket_id: &str, session_id: &str, client_transfer_id: String, client: RemoteSessionClient) {
        let remote_map = self.socket_id_2_remote_clients.entry(socket_id.to_string()).or_insert_with(|| Arc::new(DashMap::new())).clone();
        let session_map = self.socket_id_2_session_id_2_client_transfer_id.entry(socket_id.to_string()).or_insert_with(|| Arc::new(DashMap::new())).clone();
        if let Some(prev_ctid) = session_map.insert(session_id.to_string(), client_transfer_id.clone()) {
            if prev_ctid != client_transfer_id {
                self.close_sftp_remote_client(socket_id, session_id, &prev_ctid).await;
            }
        }
        remote_map.insert(client_transfer_id, client);
    }

    async fn connect_sftp_client(&self, socket: SocketRef, _session_id: String, req: Arc<ReqSocketContext>, client: Arc<ClientTransferSftpDyn>, context_ssh: ContextSsh) -> anyhow::Result<()> {
        let client_transfer_id = client.get_id();
        let sftp_service = self.sftp_service.clone();
        let handler: EventHandler<RespEventSftp> = Arc::new(move |event| {
            let socket = socket.clone();
            let req = Arc::clone(&req);
            let sftp_service = sftp_service.clone();
            let client_transfer_id = client_transfer_id.clone();
            Box::pin(async move {
                let event = inject_client_transfer_id(event, client_transfer_id);
                sftp_service.handler_sftp_event(socket, req, event);
                Ok(())
            })
        });
        client.open(&context_ssh, handler).await?;
        Ok(())
    }

    async fn connect_ftp_client(&self, socket: SocketRef, req: Arc<ReqSocketContext>, client: Arc<ClientTransferFtpDyn>, context_ftp: ContextFtp) -> anyhow::Result<()> {
        let client_transfer_id = client.get_id();
        let sftp_service = self.sftp_service.clone();
        let handler: EventHandler<RespEventSftp> = Arc::new(move |event| {
            let socket = socket.clone();
            let req = Arc::clone(&req);
            let sftp_service = sftp_service.clone();
            let client_transfer_id = client_transfer_id.clone();
            Box::pin(async move {
                let event = inject_client_transfer_id(event, client_transfer_id);
                sftp_service.handler_sftp_event(socket, req, event);
                Ok(())
            })
        });
        client.open(&context_ftp, handler).await?;
        Ok(())
    }
}

fn inject_client_transfer_id(event: RespEventSftp, client_transfer_id: String) -> RespEventSftp {
    match event {
        RespEventSftp::OnOpenStep(mut step) => {
            step.client_transfer_id = client_transfer_id;
            RespEventSftp::OnOpenStep(step)
        }
        RespEventSftp::OnError(mut err) => {
            err.client_transfer_id = client_transfer_id;
            RespEventSftp::OnError(err)
        }
        RespEventSftp::OnClose(mut base) => {
            base.client_transfer_id = client_transfer_id;
            RespEventSftp::OnClose(base)
        }
        other => other,
    }
}
