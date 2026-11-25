use anyhow::Result;
use std::sync::Arc;
use tauri::Manager;
use std::path::PathBuf;
use crate::model::req::req_ws_event::ReqWsEventSftp;
use crate::model::resp::resp_sftp::RespSftpRemoteDrag;
use crate::model::req::req_ws_context::ReqSocketContext;
use crate::web::ws::handle::r#impl::handle_sftp::HandleSftp;
use crate::model::req::event::ws_sftp::{SftpTransferData, TransferEvent};

pub fn start_remote_drag(handle: tauri::AppHandle, sftp_handler: HandleSftp, req: RespSftpRemoteDrag) -> Result<()> {
    let export_id = uuid::Uuid::new_v4().to_string();
    let temp_dir = std::env::temp_dir().join("termini").join("sftp-export").join(&export_id);
    let temp_str = temp_dir.to_str().ok_or_else(|| anyhow::anyhow!("non-utf8 temp path"))?.to_string();

    let data = SftpTransferData {
        names: req.names.clone(),
        target_path: temp_str.clone(),
        source_path: req.source_path.clone(),
        side_session_id: req.session_id.clone(),
        source_client_transfer_id: req.client_transfer_id.clone(),
        target_client_transfer_id: req.local_client_transfer_id.clone(),
    };
    let names = req.names.clone();
    let session_id = req.session_id.clone();
    let handle_for_closure = handle.clone();

    tokio::runtime::Handle::current().spawn(async move {
        let payload = serde_json::to_value(TransferEvent { data: data.clone() }).ok();
        let request = Arc::new(ReqSocketContext { r#type: ReqWsEventSftp::Transfer as i16, session_id: session_id.clone(), payload });
        let result = match sftp_handler.resolve_socket_by_session(&session_id) {
            Ok(socket) => sftp_handler.do_execute_transfer(socket, request, data, None).await,
            Err(error) => Err(error),
        };
        if let Err(e) = result {
            summer::tracing::error!("[file] linux drag pre-download failed: {e}");
            return;
        }
        let paths: Vec<String> = names.iter().map(|n| format!("{}/{}", temp_str, n)).collect();
        let _ = handle.run_on_main_thread(move || {
            let Some(window) = handle_for_closure.get_webview_window("main") else {
                return;
            };
            let Ok(gtk_win) = window.gtk_window() else {
                return;
            };
            let paths_buf: Vec<PathBuf> = paths.iter().map(PathBuf::from).collect();
            let item = drag::DragItem::Files(paths_buf);
            let image = drag::Image::Raw(vec![]);
            let _ = drag::start_drag(&gtk_win, item, image, |_, _| {}, drag::Options::default());
        });
    });

    Ok(())
}
