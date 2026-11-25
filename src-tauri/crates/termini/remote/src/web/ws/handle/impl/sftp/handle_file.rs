use anyhow::anyhow;
use std::sync::Arc;
use dashmap::DashMap;
use super::SessionClient;
use summer::tracing::info;
use crate::util::path_util;
use crate::util::id_factory::uuid;
use super::super::handle_sftp::HandleSftp;
use crate::service::service_sftp::SftpService;
use summer_web::socketioxide::extract::SocketRef;
use crate::model::req::event::ws_sftp::SftpDeleteData;
use crate::model::req::req_ws_context::ReqSocketContext;
use crate::model::resp::event::resp_event_sftp::RespEventSftp;
use crate::module::common::dto::file_entry::{FileEntry, FileKind, PlannedDelete};
use crate::model::dto::cancel_session::{cancelled, is_cancelled, CancelNotify, CancelSession};
use crate::model::req::event::ws_sftp::{ChmodEvent, CreateEvent, DeleteCancelEvent, DeleteEvent, RenameEvent};
use crate::model::resp::resp_sftp::{RespSftpBase, RespSftpDeleteCount, RespSftpDeleteProgress, RespSftpDeleteProgressHandler, DELETE_COUNT_STATUS_COUNTING, DELETE_COUNT_STATUS_DONE};

impl HandleSftp {
    pub(in super::super) async fn handle_rename(&self, socket: SocketRef, req: Arc<ReqSocketContext>) -> anyhow::Result<()> {
        let payload: RenameEvent = req.as_payload()?;
        let client_transfer_id = payload.data.client_transfer_id.clone();
        let client = self.resolve_client(&socket.id.to_string(), &client_transfer_id)?;
        let from_path = path_util::join_frontend_path(&payload.data.path, &payload.data.from_name);
        let to_path = path_util::join_frontend_path(&payload.data.path, &payload.data.to_name);
        client.rename(&from_path, &to_path).await?;
        self.sftp_service.handler_sftp_event(socket, req, RespEventSftp::OnRenamed(RespSftpBase::new(payload.data.path.clone(), client_transfer_id)));
        Ok(())
    }

    pub(in super::super) async fn handle_chmod(&self, socket: SocketRef, req: Arc<ReqSocketContext>) -> anyhow::Result<()> {
        let payload: ChmodEvent = req.as_payload()?;
        let client_transfer_id = payload.data.client_transfer_id.clone();
        let client = self.resolve_client(&socket.id.to_string(), &client_transfer_id)?;
        let mut paths = Vec::new();
        for name in &payload.data.names {
            paths.push(path_util::join_frontend_path(&payload.data.path, name));
        }
        client.chmod(&paths, payload.data.mode).await?;
        self.sftp_service.handler_sftp_event(socket, req, RespEventSftp::OnChmodded(RespSftpBase::new(payload.data.path.clone(), client_transfer_id)));
        Ok(())
    }

    pub(in super::super) async fn handle_create(&self, socket: SocketRef, req: Arc<ReqSocketContext>) -> anyhow::Result<()> {
        let payload: CreateEvent = req.as_payload()?;
        let client_transfer_id = payload.data.client_transfer_id.clone();
        let client = self.resolve_client(&socket.id.to_string(), &client_transfer_id)?;
        let full_path = path_util::join_frontend_path(&payload.data.path, &payload.data.name);
        client.create(&full_path, payload.data.kind, payload.data.link_target.as_deref()).await?;
        self.sftp_service.handler_sftp_event(socket, req, RespEventSftp::OnCreated(RespSftpBase::new(payload.data.path.clone(), client_transfer_id)));
        Ok(())
    }

    pub(in super::super) async fn handle_delete_cancel(&self, socket: SocketRef, req: Arc<ReqSocketContext>) -> anyhow::Result<()> {
        let socket_id = socket.id.to_string();
        let Some(socket_entry) = self.delete_sessions.get(&socket_id) else {
            return Ok(());
        };
        let session_id = req.session_id.clone();
        let Some(session_entry) = socket_entry.get(&session_id) else {
            return Ok(());
        };
        let payload: DeleteCancelEvent = req.as_payload()?;
        let resp_id = payload.data.resp_id;
        let Some(session) = session_entry.get(&resp_id) else {
            return Ok(());
        };
        session.value().cancel();
        Ok(())
    }

    pub(in super::super) async fn handle_delete(&self, socket: SocketRef, req: Arc<ReqSocketContext>) -> anyhow::Result<()> {
        let resp_id = uuid();
        let socket_id = socket.id.to_string();
        let session_id = req.session_id.clone();
        let data = Self::parse_delete_data(req.as_ref())?;
        info!("[sftp] sftp delete session_id: {}, path: {}, names: {}", session_id, data.path, data.names.len());
        let session = Arc::new(CancelSession::new());

        self.delete_sessions
            .entry(socket_id.clone())
            .or_insert_with(|| Arc::new(DashMap::new()))
            .entry(session_id.clone())
            .or_insert_with(|| Arc::new(DashMap::new()))
            .insert(resp_id.clone(), Arc::clone(&session));

        let result = self.do_execute_delete(socket, req, data, resp_id.clone(), session).await;

        if let Some(socket_map) = self.delete_sessions.get(&socket_id) {
            if let Some(resp_map) = socket_map.get(&session_id) {
                resp_map.remove(&resp_id);
            }
        }

        match result {
            Err(err) if is_cancelled(&err) => {
                info!("[sftp] 用户取消了 resp_id:[{}] 删除.", resp_id);
                Ok(())
            }
            other => other,
        }
    }

    async fn do_execute_delete(&self, socket: SocketRef, req: Arc<ReqSocketContext>, data: SftpDeleteData, resp_id: String, session: Arc<CancelSession>) -> anyhow::Result<()> {
        let session_id = req.session_id.clone();
        let client_transfer_id = data.client_transfer_id.clone();
        let client = self.resolve_client(&socket.id.to_string(), &client_transfer_id)?;

        let early_progress =
            RespSftpDeleteProgress { resp_id: resp_id.clone(), file: String::new(), finish: false, percent: 0.0, files_done: 0, files_total: 0, client_transfer_id: client_transfer_id.clone() };
        self.sftp_delete_reply_if_active(socket.clone(), Arc::clone(&req), RespEventSftp::OnDeleteProgress(early_progress), session.as_ref());

        let cancel = session.cancel_notify();
        let plan = self.prepare_delete_plan(socket.clone(), &data, &client, session.as_ref(), Arc::clone(&req), Some(&cancel)).await?;
        let delete_handler = Self::build_delete_handler(resp_id.clone(), socket.clone(), session_id.clone(), self.sftp_service.clone(), client_transfer_id.clone(), Arc::clone(&session));
        client.delete(&plan, Some(&cancel), delete_handler).await?;

        let resp_sftp_base = RespSftpBase::with_resp_id(data.path.clone(), client_transfer_id, resp_id);
        self.sftp_delete_reply_if_active(socket, Arc::clone(&req), RespEventSftp::OnDeleted(resp_sftp_base), session.as_ref());
        Ok(())
    }

    fn sftp_delete_reply_if_active(&self, socket: SocketRef, req: Arc<ReqSocketContext>, event: RespEventSftp, session: &CancelSession) {
        if session.is_cancelled() {
            return;
        }
        self.sftp_service.handler_sftp_event(socket, req, event);
    }

    fn parse_delete_data(req: &ReqSocketContext) -> anyhow::Result<SftpDeleteData> {
        let payload: DeleteEvent = req.as_payload()?;
        Ok(payload.data)
    }

    async fn prepare_delete_plan(
        &self,
        socket: SocketRef,
        data: &SftpDeleteData,
        client: &SessionClient,
        session: &CancelSession,
        req: Arc<ReqSocketContext>,
        cancel: Option<&CancelNotify<'_>>,
    ) -> anyhow::Result<Vec<PlannedDelete>> {
        let event_start = RespEventSftp::OnDeleteCount(RespSftpDeleteCount { client_transfer_id: data.client_transfer_id.clone(), status: DELETE_COUNT_STATUS_COUNTING, total: 0 });
        self.sftp_delete_reply_if_active(socket.clone(), Arc::clone(&req), event_start, session);

        let roots = Self::resolve_delete_roots(client, &data.path, &data.names, cancel).await?;
        let plan = Self::collect_delete_plan(client, roots, cancel).await?;
        let files_total = plan.len() as u32;

        let event_end = RespEventSftp::OnDeleteCount(RespSftpDeleteCount { client_transfer_id: data.client_transfer_id.clone(), status: DELETE_COUNT_STATUS_DONE, total: files_total });
        self.sftp_delete_reply_if_active(socket, Arc::clone(&req), event_end, session);
        Ok(plan)
    }

    pub async fn resolve_delete_roots(client: &SessionClient, root_path: &str, names: &[String], cancel: Option<&CancelNotify<'_>>) -> anyhow::Result<Vec<FileEntry>> {
        let root_entries = client.list_dir_for_transfer_plan(root_path, cancel).await?;
        let mut roots = Vec::with_capacity(names.len());
        for name in names {
            let entry = root_entries.iter().find(|entry| &entry.name == name).cloned().ok_or_else(|| anyhow!("sftp delete source entry not found: {}", name))?;
            roots.push(entry);
        }
        Ok(roots)
    }

    pub async fn collect_delete_plan(client: &SessionClient, roots: Vec<FileEntry>, cancel: Option<&CancelNotify<'_>>) -> anyhow::Result<Vec<PlannedDelete>> {
        let mut result: Vec<PlannedDelete> = Vec::new();
        let mut stack: Vec<(FileEntry, bool)> = Vec::with_capacity(roots.len());
        for root in roots.into_iter().rev() {
            stack.push((root, false));
        }
        while let Some((entry, expanded)) = stack.pop() {
            if let Some(notify) = cancel {
                if notify.is_cancelled() {
                    return Err(cancelled());
                }
            }
            if entry.kind != FileKind::Directory {
                result.push(PlannedDelete { path: entry.path, kind: entry.kind });
                continue;
            }
            if expanded {
                result.push(PlannedDelete { path: entry.path, kind: FileKind::Directory });
                continue;
            }
            let children = client.list_dir_for_transfer_plan(&entry.path, cancel).await?;
            stack.push((entry, true));
            for child in children.into_iter().rev() {
                stack.push((child, false));
            }
        }
        Ok(result)
    }

    fn build_delete_handler(
        resp_id: String,
        socket: SocketRef,
        session_id: String,
        sftp_service: SftpService,
        client_transfer_id: String,
        session: Arc<CancelSession>,
    ) -> RespSftpDeleteProgressHandler {
        Arc::new(move |mut progress: RespSftpDeleteProgress| {
            let socket = socket.clone();
            let resp_id = resp_id.clone();
            let session = Arc::clone(&session);
            let session_id = session_id.clone();
            let sftp_service = sftp_service.clone();
            progress.resp_id = resp_id;
            progress.client_transfer_id = client_transfer_id.clone();
            Box::pin(async move {
                if session.is_cancelled() {
                    return Ok(());
                }
                let req = Arc::new(ReqSocketContext { r#type: RespEventSftp::OnDeleteProgress(progress.clone()).to_type(), session_id: session_id.clone(), payload: None });
                sftp_service.handler_sftp_event(socket, req, RespEventSftp::OnDeleteProgress(progress));
                Ok(())
            })
        })
    }
}
