use anyhow::anyhow;
use std::sync::Arc;
use dashmap::DashMap;
use summer::tracing::info;
use crate::util::id_factory::uuid;
use crate::util::file_open_util::open_local_file;
use summer_web::socketioxide::extract::SocketRef;
use crate::model::resp::resp_sftp::RespSftpFileOpened;
use crate::model::req::req_ws_context::ReqSocketContext;
use crate::module::sftp::client_transfer::ClientTransfer;
use crate::web::ws::handle::r#impl::handle_sftp::HandleSftp;
use crate::model::resp::event::resp_event_sftp::RespEventSftp;
use crate::module::common::dto::file_edit_session::FileEditSession;
use crate::web::ws::handle::r#impl::sftp::session_client::SessionClient;
use crate::model::req::event::ws_sftp::{FileOpenEvent, SftpTransferData};
use crate::web::ws::handle::r#impl::sftp::handle_file_edit::sftp_edit_root_dir;

impl HandleSftp {
    pub(in super::super) async fn handle_file_open(&self, socket: SocketRef, req: Arc<ReqSocketContext>) -> anyhow::Result<()> {
        let payload: FileOpenEvent = req.as_payload()?;
        let data = payload.data;
        let session_id = req.session_id.clone();
        info!("[sftp] sftp file_open session_id: {}, path: {}, open_with: {}", session_id, data.path, data.open_with);
        let client = self.resolve_client(&socket.id.to_string(), &data.client_transfer_id)?;

        match client {
            SessionClient::Local(_) => {
                open_local_file(&data.path, data.open_with)?;
                let resp = RespSftpFileOpened { path: data.path, edit_id: None, client_transfer_id: data.client_transfer_id };
                self.sftp_service.handler_sftp_event(socket, Arc::clone(&req), RespEventSftp::OnFileOpened(resp));
            }
            SessionClient::Remote(_) => {
                let edit_id = uuid();
                let file_name = Self::posix_file_name(&data.path);
                let local_client_id = self.client_transfer_local.get_id();
                let remote_parent = client.parent_path(&data.path).ok_or_else(|| anyhow!("cannot resolve parent path of {}", data.path))?;
                let local_dir = sftp_edit_root_dir().join(&edit_id);
                let local_path = local_dir.join(&file_name);
                let local_dir_str = local_dir.to_str().ok_or_else(|| anyhow!("non-UTF-8 temp path"))?.to_string();
                let local_path_str = local_path.to_str().ok_or_else(|| anyhow!("non-UTF-8 temp path"))?.to_string();

                let download_data = SftpTransferData {
                    names: vec![file_name.clone()],
                    source_path: remote_parent.clone(),
                    target_path: local_dir_str.clone(),
                    side_session_id: session_id.clone(),
                    target_client_transfer_id: local_client_id.clone(),
                    source_client_transfer_id: data.client_transfer_id.clone(),
                };
                self.do_execute_transfer(socket.clone(), Arc::clone(&req), download_data, None).await?;
                open_local_file(&local_path_str, data.open_with)?;

                let socket_id = socket.id.to_string();
                let (watcher, abort_handle) =
                    self.start_edit_watch(edit_id.clone(), file_name, socket.clone(), local_path, local_dir_str, remote_parent, local_client_id, data.client_transfer_id.clone(), Arc::clone(&req))?;

                self.file_edit_sessions
                    .entry(socket_id.clone())
                    .or_insert_with(|| Arc::new(DashMap::new()))
                    .entry(session_id.clone())
                    .or_insert_with(|| Arc::new(DashMap::new()))
                    .insert(edit_id.clone(), FileEditSession { local_dir, abort_handle, _watcher: watcher });

                let resp = RespSftpFileOpened { path: data.path, edit_id: Some(edit_id), client_transfer_id: data.client_transfer_id };
                self.sftp_service.handler_sftp_event(socket, Arc::clone(&req), RespEventSftp::OnFileOpened(resp));
            }
        }
        Ok(())
    }
}
