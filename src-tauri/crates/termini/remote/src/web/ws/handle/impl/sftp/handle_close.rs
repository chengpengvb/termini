use anyhow::anyhow;
use std::sync::Arc;
use dashmap::DashMap;
use super::super::handle_sftp::HandleSftp;
use crate::model::resp::resp_sftp::RespSftpBase;
use summer_web::socketioxide::extract::SocketRef;
use crate::model::req::req_ws_event::ReqWsEventSftp;
use crate::model::dto::cancel_session::CancelSession;
use crate::model::req::req_ws_context::ReqSocketContext;
use crate::model::resp::event::resp_event_sftp::RespEventSftp;

impl HandleSftp {
    pub(in super::super) async fn handle_close_session(&self, socket: SocketRef, req: Arc<ReqSocketContext>) -> anyhow::Result<()> {
        let session_id = req.session_id.clone();
        if session_id.is_empty() {
            return Err(anyhow!("sftp close req session_id is empty"));
        }
        let socket_id = socket.id.to_string();
        self.cleanup_sftp_client(&socket_id, req, true).await;
        Ok(())
    }

    pub(in super::super) async fn cleanup_sftp_socket(&self, socket_id: &str) {
        let session_ids: Vec<String> = if let Some(session_map) = self.socket_id_2_session_locks.get(socket_id) { session_map.iter().map(|entry| entry.key().clone()).collect() } else { Vec::new() };
        for session_id in session_ids {
            let req = Arc::new(ReqSocketContext { session_id, r#type: ReqWsEventSftp::Close as i16, ..Default::default() });
            self.cleanup_sftp_client(socket_id, req, true).await;
        }
    }

    pub(in super::super) async fn cleanup_sftp_client(&self, socket_id: &str, req: Arc<ReqSocketContext>, notify_close: bool) {
        let session_id = req.session_id.clone();
        let client_transfer_id = self.lookup_sftp_client_transfer_id(socket_id, &session_id);
        self.cancel_sftp_client_pending(socket_id, &session_id);

        let Some(session_lock_map) = self.socket_id_2_session_locks.get(socket_id).map(|entry| Arc::clone(entry.value())) else {
            return;
        };
        let Some(session_lock) = session_lock_map.get(&session_id).map(|entry| Arc::clone(entry.value())) else {
            return;
        };
        let _guard = session_lock.lock().await;
        self.teardown_sftp_session_locked(socket_id, &session_id, client_transfer_id.as_deref(), &session_lock_map).await;

        if notify_close {
            self.reply_sftp_session_close(socket_id, req, client_transfer_id);
        }
    }

    fn cancel_sftp_client_pending(&self, socket_id: &str, session_id: &str) {
        self.cancel_task_sessions(&self.transfer_sessions, socket_id, session_id);
        self.cancel_task_sessions(&self.delete_sessions, socket_id, session_id);
        self.remove_transfer_answer_requests(socket_id, session_id);
        self.cleanup_file_edit_sessions_by_session(socket_id, session_id);
        self.client_session_factory.cancel_connect(session_id);
    }

    async fn teardown_sftp_session_locked(&self, socket_id: &str, session_id: &str, client_transfer_id: Option<&str>, session_lock_map: &Arc<DashMap<String, Arc<tokio::sync::Mutex<()>>>>) {
        if let Some(client_transfer_id) = client_transfer_id {
            self.close_sftp_remote_client(socket_id, session_id, client_transfer_id).await;
        }
        self.remove_sftp_session_index(&self.socket_id_2_session_id_2_client_transfer_id, socket_id, session_id);
        session_lock_map.remove(session_id);
        if session_lock_map.is_empty() {
            self.socket_id_2_session_locks.remove(socket_id);
        }
    }

    fn reply_sftp_session_close(&self, socket_id: &str, req: Arc<ReqSocketContext>, client_transfer_id: Option<String>) {
        let Some(client_transfer_id) = client_transfer_id else {
            return;
        };
        let Some(socket) = self.socket_refs.get(socket_id).map(|entry| entry.value().clone()) else {
            return;
        };
        self.sftp_service.handler_sftp_event(socket, req, RespEventSftp::OnClose(RespSftpBase::new(String::new(), client_transfer_id)));
    }

    fn lookup_sftp_client_transfer_id(&self, socket_id: &str, session_id: &str) -> Option<String> {
        self.socket_id_2_session_id_2_client_transfer_id.get(socket_id).and_then(|session_map| session_map.get(session_id).map(|entry| entry.value().clone()))
    }

    fn remove_sftp_session_index<V>(&self, outer: &DashMap<String, Arc<DashMap<String, V>>>, socket_id: &str, session_id: &str) {
        let Some(inner_map) = outer.get(socket_id) else {
            return;
        };
        inner_map.remove(session_id);
        if inner_map.is_empty() {
            drop(inner_map);
            outer.remove(socket_id);
        }
    }

    fn cancel_task_sessions(&self, sessions: &DashMap<String, Arc<DashMap<String, Arc<DashMap<String, Arc<CancelSession>>>>>>, socket_id: &str, session_id: &str) {
        let Some(socket_map) = sessions.get(socket_id) else {
            return;
        };
        let Some(resp_map) = socket_map.get(session_id) else {
            return;
        };
        for entry in resp_map.iter() {
            entry.value().cancel();
        }
        drop(resp_map);
        socket_map.remove(session_id);
        if socket_map.is_empty() {
            drop(socket_map);
            sessions.remove(socket_id);
        }
    }

    fn remove_transfer_answer_requests(&self, socket_id: &str, session_id: &str) {
        let Some(socket_map) = self.transfer_answer_requests.get(socket_id) else {
            return;
        };
        if let Some((_, request_map)) = socket_map.remove(session_id) {
            for entry in request_map.iter() {
                entry.value().notify.notify_waiters();
            }
        }
        if socket_map.is_empty() {
            drop(socket_map);
            self.transfer_answer_requests.remove(socket_id);
        }
    }
}
