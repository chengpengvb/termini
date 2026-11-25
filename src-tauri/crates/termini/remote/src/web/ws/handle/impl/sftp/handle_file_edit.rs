use anyhow::anyhow;
use std::sync::Arc;
use notify::Watcher;
use std::path::PathBuf;
use std::time::Duration;
use summer::tracing::warn;
use tokio::task::AbortHandle;
use crate::model::resp::resp_sftp::RespSftpError;
use summer_web::socketioxide::extract::SocketRef;
use crate::model::dto::cancel_session::is_cancelled;
use crate::model::req::event::ws_sftp::SftpTransferData;
use crate::model::req::req_ws_context::ReqSocketContext;
use crate::web::ws::handle::r#impl::handle_sftp::HandleSftp;
use crate::model::resp::event::resp_event_sftp::RespEventSftp;
use crate::module::common::dto::file_edit_session::FileEditSession;

fn sftp_scratch_root() -> PathBuf {
    #[cfg(target_os = "ios")]
    {
        match crate::util::ios_local_dir::runtime_dir() {
            Ok(dir) => return dir,
            Err(error) => warn!("[sftp] ios runtime dir unavailable, falling back to temp dir: {:#}", error),
        }
    }
    std::env::temp_dir().join("termini")
}

pub fn sftp_edit_root_dir() -> PathBuf {
    sftp_scratch_root().join("sftp-edit")
}

pub fn sftp_export_root_dir() -> PathBuf {
    sftp_scratch_root().join("sftp-export")
}

impl HandleSftp {
    pub(in super::super) fn cleanup_sftp_edit_dir_on_start() {
        let root = sftp_edit_root_dir();
        if !root.exists() {
            return;
        }
        let _ = std::fs::remove_dir_all(&root);
    }

    pub(in super::super) fn cleanup_file_edit_sessions_by_session(&self, socket_id: &str, session_id: &str) {
        let edit_ids: Vec<String> = if let Some(session_map) = self.file_edit_sessions.get(socket_id) {
            if let Some(edit_map) = session_map.get(session_id) {
                edit_map.iter().map(|entry| entry.key().clone()).collect()
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };
        for edit_id in edit_ids {
            self.cleanup_file_edit_session(socket_id, session_id, &edit_id);
        }
    }

    fn take_file_edit_session(&self, socket_id: &str, session_id: &str, edit_id: &str) -> Option<FileEditSession> {
        let Some(socket_map) = self.file_edit_sessions.get(socket_id) else {
            return None;
        };
        let Some(edit_map) = socket_map.get(session_id) else {
            return None;
        };
        let Some((_, session)) = edit_map.remove(edit_id) else {
            return None;
        };
        drop(edit_map);
        self.compact_file_edit_layers(socket_id, session_id);
        Some(session)
    }

    fn compact_file_edit_layers(&self, socket_id: &str, session_id: &str) {
        let Some(socket_map) = self.file_edit_sessions.get(socket_id) else {
            return;
        };
        if let Some(edit_map) = socket_map.get(session_id) {
            if edit_map.is_empty() {
                drop(edit_map);
                socket_map.remove(session_id);
            }
        }
        if socket_map.is_empty() {
            drop(socket_map);
            self.file_edit_sessions.remove(socket_id);
        }
    }

    fn cleanup_file_edit_session(&self, socket_id: &str, session_id: &str, edit_id: &str) {
        let Some(session) = self.take_file_edit_session(socket_id, session_id, edit_id) else {
            return;
        };
        session.abort_handle.abort();
        let _ = std::fs::remove_dir_all(&session.local_dir);
    }

    fn end_file_edit_session(&self, socket_id: &str, session_id: &str, edit_id: &str) -> Option<PathBuf> {
        let session = self.take_file_edit_session(socket_id, session_id, edit_id)?;
        Some(session.local_dir)
    }

    fn fail_file_edit_session(&self, socket_id: &str, session_id: &str, edit_id: &str, socket: SocketRef, req: Arc<ReqSocketContext>, client_transfer_id: String, error: anyhow::Error) {
        if let Some(local_dir) = self.end_file_edit_session(socket_id, session_id, edit_id) {
            let _ = std::fs::remove_dir_all(&local_dir);
        }
        if is_cancelled(&error) {
            return;
        }
        warn!("[file] upload failed, edit_id={} error={}", edit_id, error);
        let resp = RespSftpError { client_transfer_id, message: format!("{:#}", error) };
        self.sftp_service.handler_sftp_event(socket, req, RespEventSftp::OnError(resp));
    }

    pub(in super::super) fn start_edit_watch(
        &self,
        edit_id: String,
        file_name: String,
        socket: SocketRef,
        local_path: PathBuf,
        local_dir_str: String,
        remote_parent: String,
        local_client_id: String,
        remote_client_id: String,
        req: Arc<ReqSocketContext>,
    ) -> anyhow::Result<(notify::RecommendedWatcher, AbortHandle)> {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<()>();

        let watched_file_name = local_path.file_name().ok_or_else(|| anyhow!("cannot resolve file name for watch"))?.to_os_string();
        let mut watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
            if let Ok(event) = res {
                let is_relevant = matches!(event.kind, notify::EventKind::Modify(_) | notify::EventKind::Create(_)) && event.paths.iter().any(|p| p.file_name() == Some(watched_file_name.as_os_str()));
                if is_relevant {
                    let _ = tx.send(());
                }
            }
        })?;

        let parent_dir = local_path.parent().ok_or_else(|| anyhow!("cannot resolve parent dir for watch"))?;
        watcher.watch(parent_dir, notify::RecursiveMode::NonRecursive)?;

        let socket_id = socket.id.to_string();
        let handler_clone = self.clone();
        let join_handle = tokio::spawn(async move {
            let _ = Self::file_edit_upload_loop(edit_id, socket_id, file_name, socket, handler_clone, local_dir_str, remote_parent, local_client_id, remote_client_id, Arc::clone(&req), rx).await;
        });
        Ok((watcher, join_handle.abort_handle()))
    }

    async fn file_edit_upload_loop(
        edit_id: String,
        socket_id: String,
        file_name: String,
        socket: SocketRef,
        handler: HandleSftp,
        local_dir_str: String,
        remote_parent: String,
        local_client_id: String,
        remote_client_id: String,
        req: Arc<ReqSocketContext>,
        mut rx: tokio::sync::mpsc::UnboundedReceiver<()>,
    ) -> anyhow::Result<()> {
        loop {
            let Some(()) = rx.recv().await else { break };
            tokio::time::sleep(Duration::from_millis(400)).await;
            while rx.try_recv().is_ok() {}

            let session_id = req.session_id.clone();
            let upload_data = SftpTransferData {
                names: vec![file_name.clone()],
                source_path: local_dir_str.clone(),
                target_path: remote_parent.clone(),
                side_session_id: session_id.clone(),
                source_client_transfer_id: local_client_id.clone(),
                target_client_transfer_id: remote_client_id.clone(),
            };
            if let Err(error) = handler.do_execute_transfer(socket.clone(), Arc::clone(&req), upload_data, Some(true)).await {
                handler.fail_file_edit_session(&socket_id, &session_id, &edit_id, socket, Arc::clone(&req), remote_client_id, error);
                return Err(anyhow!("file edit session terminated, edit_id={}", edit_id));
            }
        }
        Ok(())
    }

    pub(in super::super) fn posix_file_name(path: &str) -> String {
        path.trim_end_matches('/').rsplit('/').next().filter(|s| !s.is_empty()).unwrap_or(path).to_string()
    }
}
