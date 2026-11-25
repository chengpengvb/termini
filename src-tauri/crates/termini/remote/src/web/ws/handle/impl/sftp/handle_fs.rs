use std::sync::Arc;
use summer::tracing::warn;
use super::super::handle_sftp::HandleSftp;
use summer_web::socketioxide::extract::SocketRef;
use crate::model::req::req_ws_context::ReqSocketContext;
use crate::model::resp::event::resp_event_sftp::RespEventSftp;
use crate::module::common::dto::file_entry::{FileEntry, FileKind};
use crate::model::req::event::ws_sftp::{HomeDirEvent, ListDirEvent, StatEvent};
use crate::model::resp::resp_sftp::{RespSftpBase, RespSftpEntries, RespSftpEntry, RespSftpError, RespSftpStatProperties};

impl HandleSftp {
    pub(in super::super) async fn handle_home_dir(&self, socket: SocketRef, req: Arc<ReqSocketContext>) -> anyhow::Result<()> {
        let payload: HomeDirEvent = req.as_payload()?;
        let client_transfer_id = payload.data.client_transfer_id.clone();
        let client = self.resolve_client(&socket.id.to_string(), &client_transfer_id)?;
        let path = client.home_dir().await?;
        self.sftp_service.handler_sftp_event(socket, Arc::clone(&req), RespEventSftp::OnHomeDir(RespSftpBase::new(path, client_transfer_id)));
        Ok(())
    }

    pub(in super::super) async fn handle_list_dir(&self, socket: SocketRef, req: Arc<ReqSocketContext>) -> anyhow::Result<()> {
        let payload: ListDirEvent = req.as_payload()?;
        let client_transfer_id = payload.data.client_transfer_id.clone();
        let client = self.resolve_client(&socket.id.to_string(), &client_transfer_id)?;

        match client.list_dir_resolving_links(&payload.data.path).await {
            Ok(resolved) => {
                let parent_path = client.parent_path(&resolved.effective_path);
                let mut entries: Vec<FileEntry> = resolved.entries;
                if let Some(parent) = parent_path.clone() {
                    let parent_entry = FileEntry { hidden: false, name: "..".to_string(), path: parent, kind: FileKind::Parent, ..Default::default() };
                    entries.insert(0, parent_entry);
                }
                self.sftp_service.handler_sftp_event(socket.clone(), Arc::clone(&req), RespEventSftp::OnPath(RespSftpBase::new(resolved.effective_path, client_transfer_id.clone())));
                let list_event = RespEventSftp::OnListDir(RespSftpEntries { client_transfer_id, entries, has_parent: parent_path.is_some(), parent_path });
                self.sftp_service.handler_sftp_event(socket, Arc::clone(&req), list_event);
            }
            Err(error) => {
                warn!("[sftp] list_dir 失败 path={} error={:#}", payload.data.path, error);
                self.sftp_service.handler_sftp_event(socket, Arc::clone(&req), RespEventSftp::OnError(RespSftpError { client_transfer_id, message: format!("{:#}", error) }));
            }
        }
        Ok(())
    }

    pub(in super::super) async fn handle_stat(&self, socket: SocketRef, req: Arc<ReqSocketContext>) -> anyhow::Result<()> {
        let payload: StatEvent = req.as_payload()?;
        let client_transfer_id = payload.data.client_transfer_id.clone();
        let client = self.resolve_client(&socket.id.to_string(), &client_transfer_id)?;
        let entry = client.stat(&payload.data.path, &payload.data.kind).await?;
        self.sftp_service.handler_sftp_event(socket, Arc::clone(&req), RespEventSftp::OnStat(RespSftpEntry { client_transfer_id, entry }));
        Ok(())
    }

    pub(in super::super) async fn handle_stat_properties(&self, socket: SocketRef, req: Arc<ReqSocketContext>) -> anyhow::Result<()> {
        let payload: StatEvent = req.as_payload()?;
        let client_transfer_id = payload.data.client_transfer_id.clone();
        let client = self.resolve_client(&socket.id.to_string(), &client_transfer_id)?;
        let properties = client.stat_properties(&payload.data.path, &payload.data.kind).await?;
        self.sftp_service.handler_sftp_event(socket, Arc::clone(&req), RespEventSftp::OnStatProperties(RespSftpStatProperties { client_transfer_id, properties }));
        Ok(())
    }
}
