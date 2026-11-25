use anyhow::anyhow;
use std::sync::Arc;
use crate::util::id_factory::uuid;
use super::super::handle_sftp::HandleSftp;
use summer_web::socketioxide::extract::SocketRef;
use crate::model::resp::resp_sftp::RespSftpExported;
use crate::model::req::req_ws_context::ReqSocketContext;
use crate::module::sftp::client_transfer::ClientTransfer;
use crate::model::resp::event::resp_event_sftp::RespEventSftp;
use crate::model::req::event::ws_sftp::{ExportToTempEvent, SftpTransferData};
use crate::web::ws::handle::r#impl::sftp::handle_file_edit::sftp_export_root_dir;

impl HandleSftp {
    pub(in super::super) fn cleanup_sftp_export_dir_on_start() {
        let root = sftp_export_root_dir();
        if !root.exists() {
            return;
        }
        let _ = std::fs::remove_dir_all(&root);
    }

    pub(in super::super) async fn handle_export_to_temp(&self, socket: SocketRef, req: Arc<ReqSocketContext>) -> anyhow::Result<()> {
        let export_id = uuid();
        let payload: ExportToTempEvent = req.as_payload()?;
        let data = payload.data;
        let session_id = req.session_id.clone();

        let temp_dir = sftp_export_root_dir().join(&export_id);
        tokio::fs::create_dir_all(&temp_dir).await.map_err(|e| anyhow!("create temp dir: {}", e))?;

        let temp_dir_str = temp_dir.to_str().ok_or_else(|| anyhow!("non-UTF-8 temp path"))?.to_string();
        let local_client_id = self.client_transfer_local.get_id();
        let source_client_transfer_id = data.source_client_transfer_id.clone();

        let transfer_data = SftpTransferData {
            names: data.names.clone(),
            source_client_transfer_id,
            source_path: data.source_path,
            target_path: temp_dir_str.clone(),
            side_session_id: session_id.clone(),
            target_client_transfer_id: local_client_id,
        };
        self.do_execute_transfer(socket.clone(), Arc::clone(&req), transfer_data, None).await?;
        let paths: Vec<String> = data.names.iter().map(|name| temp_dir.join(name).to_str().unwrap_or(name.as_str()).replace('\\', "/")).collect();
        let resp = RespSftpExported { paths, client_transfer_id: data.source_client_transfer_id };
        self.sftp_service.handler_sftp_event(socket, req, RespEventSftp::OnExported(resp));
        Ok(())
    }
}
