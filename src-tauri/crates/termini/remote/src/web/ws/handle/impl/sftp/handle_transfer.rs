use std::pin::Pin;
use anyhow::anyhow;
use std::sync::Arc;
use dashmap::DashMap;
use std::time::Instant;
use std::future::Future;
use tokio::sync::Notify;
use super::SessionClient;
use summer::tracing::info;
use tokio::time::Duration;
use super::RemoteSessionClient;
use crate::model::resp::resp_sftp;
use crate::util::id_factory::uuid;
use super::super::handle_sftp::HandleSftp;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use summer_web::socketioxide::extract::SocketRef;
use crate::model::req::req_ws_context::ReqSocketContext;
use crate::model::resp::event::resp_event_sftp::RespEventSftp;
use crate::module::common::dto::file_entry::{FileEntry, FileKind};
use crate::model::dto::cancel_session::{cancelled, CancelNotify, CancelSession};
use crate::model::req::event::ws_sftp::{SftpTransferData, TransferAnswerEvent, TransferCancelEvent, TransferEvent};
use crate::module::common::dto::sftp_transfer_answer::{SftpTransferAnswer, SftpTransferAnswerEntry, SftpTransferInbox};

const TRANSFER_CHUNK_BYTES: usize = 64 * 1024;
const TRANSFER_PROGRESS_INTERVAL_MS: u64 = 200;
const TRANSFER_CONFLICT_TIMEOUT_MS: u64 = 180_000;
const TRANSFER_PROGRESS_MIN_BYTES: u64 = 256 * 1024;

struct PlannedFile {
    size: Option<u64>,
    mode: Option<u32>,
    source_path: String,
    target_path: String,
}

struct PlannedDir {
    mode: Option<u32>,
    target_path: String,
}

struct TransferPlan {
    files: Vec<PlannedFile>,
    target_dirs: Vec<PlannedDir>,
}

pub struct TransferCoreInput {
    pub names: Vec<String>,
    pub resp_id: String,
    pub source_path: String,
    pub target_path: String,
    pub client_transfer_id: String,
    pub force_overwrite: Option<bool>,
}

pub type TransferEventCallback = Arc<dyn Fn(RespEventSftp) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;
pub type TransferConflictCallback = Arc<dyn Fn(String) -> Pin<Box<dyn Future<Output = anyhow::Result<SftpTransferAnswer>> + Send>> + Send + Sync>;

pub struct TransferCoreHooks {
    pub on_event: TransferEventCallback,
    pub on_conflict: Option<TransferConflictCallback>,
}

fn compute_transfer_percent(done_bytes: u64, total_bytes: u64) -> f32 {
    if total_bytes == 0 {
        return 0.0;
    }
    (done_bytes as f32 / total_bytes as f32 * 100.0).min(100.0)
}

impl HandleSftp {
    fn sftp_transfer_reply_if_active(&self, socket: SocketRef, req: Arc<ReqSocketContext>, event: RespEventSftp, session: &CancelSession) {
        if session.is_cancelled() {
            return;
        }
        self.sftp_service.handler_sftp_event(socket, req, event);
    }

    pub(in super::super) async fn handle_transfer_cancel(&self, socket: SocketRef, req: Arc<ReqSocketContext>) -> anyhow::Result<()> {
        let socket_id = socket.id.to_string();
        let Some(socket_entry) = self.transfer_sessions.get(&socket_id) else {
            return Ok(());
        };
        let session_id = req.session_id.clone();
        let Some(session_entry) = socket_entry.get(&session_id) else {
            return Ok(());
        };
        let payload: TransferCancelEvent = req.as_payload()?;
        let resp_id = payload.data.resp_id;
        let Some(session) = session_entry.get(&resp_id) else {
            return Ok(());
        };
        session.value().cancel();
        Ok(())
    }

    pub(in super::super) async fn handle_transfer(&self, socket: SocketRef, req: Arc<ReqSocketContext>) -> anyhow::Result<()> {
        let data = Self::parse_transfer_data(req.as_ref())?;
        info!("[sftp] sftp transfer session_id: {}, {} -> {}, names: {}", req.session_id, data.source_path, data.target_path, data.names.len());
        self.do_execute_transfer(socket, req, data, None).await
    }

    pub async fn do_execute_transfer(&self, socket: SocketRef, req: Arc<ReqSocketContext>, data: SftpTransferData, force_overwrite: Option<bool>) -> anyhow::Result<()> {
        let resp_id = uuid();
        let socket_id = socket.id.to_string();
        let session = Arc::new(CancelSession::new());
        let side_session_id = data.side_session_id.clone();
        self.transfer_sessions
            .entry(socket_id.clone())
            .or_insert_with(|| Arc::new(DashMap::new()))
            .entry(side_session_id.clone())
            .or_insert_with(|| Arc::new(DashMap::new()))
            .insert(resp_id.clone(), Arc::clone(&session));

        let result = async {
            let source = self.resolve_client(&socket_id, &data.source_client_transfer_id)?;
            let target = self.resolve_client(&socket_id, &data.target_client_transfer_id)?;
            let hooks = self.build_ws_transfer_hooks(socket, Arc::clone(&req), &data, Arc::clone(&session));
            let input = TransferCoreInput {
                force_overwrite,
                names: data.names.clone(),
                resp_id: resp_id.clone(),
                source_path: data.source_path.clone(),
                target_path: data.target_path.clone(),
                client_transfer_id: data.target_client_transfer_id.clone(),
            };
            self.execute_transfer_core(&source, &target, input, session, &hooks).await
        }
        .await;

        if let Some(socket_map) = self.transfer_sessions.get(&socket_id) {
            if let Some(resp_map) = socket_map.get(&side_session_id) {
                resp_map.remove(&resp_id);
            }
        }
        result
    }

    pub async fn execute_transfer_core(&self, source: &SessionClient, target: &SessionClient, input: TransferCoreInput, session: Arc<CancelSession>, hooks: &TransferCoreHooks) -> anyhow::Result<()> {
        let cancel = session.cancel_notify();
        (hooks.on_event)(RespEventSftp::OnTransferCount(resp_sftp::RespSftpTransferCount {
            total: 0,
            client_transfer_id: input.client_transfer_id.clone(),
            status: resp_sftp::TRANSFER_COUNT_STATUS_COUNTING,
        }))
        .await;

        let roots = Self::resolve_transfer_roots(&input.source_path, &input.target_path, &input.names, source, Some(&cancel)).await?;
        let plan = Self::collect_transfer_plan(source, roots, Some(&cancel)).await?;
        let files_total = plan.files.len() as u32;

        (hooks.on_event)(RespEventSftp::OnTransferCount(resp_sftp::RespSftpTransferCount {
            total: files_total,
            client_transfer_id: input.client_transfer_id.clone(),
            status: resp_sftp::TRANSFER_COUNT_STATUS_DONE,
        }))
        .await;

        self.create_transfer_target_dirs(target, &plan).await?;
        self.run_transfer_files(&input.resp_id, files_total, &plan, &input, source, target, Arc::clone(&session), hooks).await?;
        self.apply_transfer_dir_modes(target, &plan).await?;

        let last_file_path = plan.files.last().map(|file| file.target_path.clone()).unwrap_or_default();
        let progress_finish = resp_sftp::RespSftpTfProgress {
            files_total,
            finish: true,
            percent: 100.0,
            file: last_file_path,
            files_done: files_total,
            resp_id: input.resp_id.clone(),
            client_transfer_id: input.client_transfer_id.clone(),
            ..Default::default()
        };
        (hooks.on_event)(RespEventSftp::OnTransferProgress(progress_finish)).await;
        let resp_sftp_base = resp_sftp::RespSftpBase::with_resp_id(input.target_path.clone(), input.client_transfer_id.clone(), input.resp_id.clone());
        (hooks.on_event)(RespEventSftp::OnTransfer(resp_sftp_base)).await;
        Ok(())
    }

    fn build_ws_transfer_hooks(&self, socket: SocketRef, req: Arc<ReqSocketContext>, data: &SftpTransferData, session: Arc<CancelSession>) -> TransferCoreHooks {
        let handler = self.clone();
        let socket_for_event = socket.clone();
        let reply_req = Arc::new(Self::transfer_reply_req(req, data));
        let req_for_event = Arc::clone(&reply_req);
        let session_for_event = Arc::clone(&session);
        let on_event: TransferEventCallback = Arc::new(move |event| {
            let handler = handler.clone();
            let socket = socket_for_event.clone();
            let req = Arc::clone(&req_for_event);
            let session = Arc::clone(&session_for_event);
            Box::pin(async move {
                handler.sftp_transfer_reply_if_active(socket, req, event, session.as_ref());
            })
        });

        let handler = self.clone();
        let socket_for_conflict = socket;
        let req_for_conflict = reply_req;
        let data_for_conflict = data.clone();
        let session_for_conflict = session;
        let client_transfer_id = data.target_client_transfer_id.clone();
        let on_conflict: TransferConflictCallback = Arc::new(move |path| {
            let handler = handler.clone();
            let socket = socket_for_conflict.clone();
            let req = Arc::clone(&req_for_conflict);
            let data = data_for_conflict.clone();
            let session = Arc::clone(&session_for_conflict);
            let client_transfer_id = client_transfer_id.clone();
            Box::pin(async move { handler.confirm_transfer_overwrite(&path, socket, &data, &client_transfer_id, req, session).await })
        });

        TransferCoreHooks { on_event, on_conflict: Some(on_conflict) }
    }

    fn transfer_reply_req(req: Arc<ReqSocketContext>, data: &SftpTransferData) -> ReqSocketContext {
        let mut reply = (*req).clone();
        if !data.side_session_id.is_empty() {
            reply.session_id = data.side_session_id.clone();
        }
        reply
    }

    fn parse_transfer_data(req: &ReqSocketContext) -> anyhow::Result<SftpTransferData> {
        let payload: TransferEvent = req.as_payload()?;
        Ok(payload.data)
    }

    async fn create_transfer_target_dirs(&self, target: &SessionClient, plan: &TransferPlan) -> anyhow::Result<()> {
        for dir in &plan.target_dirs {
            target.create(&dir.target_path, FileKind::Directory, None).await?;
        }
        Ok(())
    }

    async fn apply_transfer_dir_modes(&self, target: &SessionClient, plan: &TransferPlan) -> anyhow::Result<()> {
        if matches!(target, SessionClient::Remote(RemoteSessionClient::Ftp(_))) {
            return Ok(());
        }
        for dir in &plan.target_dirs {
            if let Some(mode) = dir.mode {
                target.chmod(std::slice::from_ref(&dir.target_path), mode & 0o7777).await?;
            }
        }
        Ok(())
    }

    async fn run_transfer_files(
        &self,
        resp_id: &str,
        files_total: u32,
        plan: &TransferPlan,
        input: &TransferCoreInput,
        source: &SessionClient,
        target: &SessionClient,
        session: Arc<CancelSession>,
        hooks: &TransferCoreHooks,
    ) -> anyhow::Result<()> {
        let mut files_done = 0u32;
        let mut skip_all = false;
        let mut overwrite_all = matches!(input.force_overwrite, Some(true));
        let total_bytes: u64 = plan.files.iter().map(|file| file.size.unwrap_or(0)).sum();
        let mut done_bytes: u64 = 0;
        let transfer_start = Instant::now();
        for file in &plan.files {
            if session.is_cancelled() {
                return Err(cancelled());
            }
            if !overwrite_all && target.exists(&file.target_path).await? {
                if skip_all {
                    files_done += 1;
                    done_bytes += file.size.unwrap_or(0);
                    continue;
                }
                match input.force_overwrite {
                    Some(true) => {
                        overwrite_all = true;
                    }
                    Some(false) => {
                        return Err(anyhow!("remote file already exists: {}. Set force_overwrite=true to overwrite.", file.target_path));
                    }
                    None => {
                        let Some(on_conflict) = hooks.on_conflict.as_ref() else {
                            return Err(anyhow!("remote file already exists: {}. conflict resolver is missing.", file.target_path));
                        };
                        let answer = on_conflict(file.target_path.clone()).await?;
                        match answer {
                            SftpTransferAnswer::Skip => {
                                files_done += 1;
                                done_bytes += file.size.unwrap_or(0);
                                continue;
                            }
                            SftpTransferAnswer::SkipAll => {
                                skip_all = true;
                                files_done += 1;
                                done_bytes += file.size.unwrap_or(0);
                                continue;
                            }
                            SftpTransferAnswer::OverwriteAll => {
                                overwrite_all = true;
                            }
                            SftpTransferAnswer::Overwrite => {}
                        }
                    }
                }
            }
            let written = self
                .copy_planned_file(resp_id, files_total, file, files_done, done_bytes, total_bytes, transfer_start, source, target, &input.client_transfer_id, Arc::clone(&session), &hooks.on_event)
                .await?;
            done_bytes += written;
            files_done += 1;
        }
        Ok(())
    }

    pub(in super::super) async fn handle_transfer_answer(&self, socket: SocketRef, req: Arc<ReqSocketContext>) -> anyhow::Result<()> {
        let socket_id = socket.id.to_string();
        let Some(socket_entry) = self.transfer_answer_requests.get(&socket_id) else {
            anyhow::bail!("sftp transfer answer socket not found socket_id={}", socket_id);
        };
        let session_id = req.session_id.clone();
        let Some(session_entry) = socket_entry.get(&session_id) else {
            anyhow::bail!("sftp transfer answer session not found session_id={}", session_id);
        };
        let payload: TransferAnswerEvent = req.as_payload()?;
        let data = payload.data;
        let answer = SftpTransferAnswer::from_code(data.code).ok_or_else(|| anyhow!("sftp transfer answer code invalid: {}", data.code))?;
        let Some(inbox) = session_entry.get(&data.request_id).map(|entry| entry.value().clone()) else {
            anyhow::bail!("sftp transfer answer request not found request_id={}", data.request_id);
        };
        inbox.queue.lock().await.push_back(SftpTransferAnswerEntry { request_id: data.request_id, answer });
        inbox.notify.notify_waiters();
        Ok(())
    }

    async fn resolve_transfer_roots(source_path: &str, target_path: &str, names: &[String], source: &SessionClient, cancel: Option<&CancelNotify<'_>>) -> anyhow::Result<Vec<(FileEntry, String)>> {
        let root_entries = source.list_dir_for_transfer_plan(source_path, cancel).await?;
        let mut roots = Vec::with_capacity(names.len());
        for name in names {
            let entry = root_entries.iter().find(|entry| &entry.name == name).cloned().ok_or_else(|| anyhow!("sftp transfer source entry not found: {}", name))?;
            let target_full = Self::join_path(target_path, name);
            roots.push((entry, target_full));
        }
        Ok(roots)
    }

    async fn collect_transfer_plan(source: &SessionClient, roots: Vec<(FileEntry, String)>, cancel: Option<&CancelNotify<'_>>) -> anyhow::Result<TransferPlan> {
        let mut stack = roots;
        let mut files = Vec::new();
        let mut target_dirs = Vec::new();
        while let Some((entry, target_full)) = stack.pop() {
            if let Some(notify) = cancel {
                if notify.is_cancelled() {
                    return Err(cancelled());
                }
            }
            if entry.kind == FileKind::Directory {
                target_dirs.push(PlannedDir { mode: entry.permissions, target_path: target_full.clone() });
                let children = source.list_dir_for_transfer_plan(&entry.path, cancel).await?;
                for child in children {
                    let child_target = Self::join_path(&target_full, &child.name);
                    stack.push((child, child_target));
                }
                continue;
            }
            files.push(PlannedFile { size: entry.size, mode: entry.permissions, source_path: entry.path.clone(), target_path: target_full });
        }
        Ok(TransferPlan { files, target_dirs })
    }

    #[allow(clippy::too_many_arguments)]
    async fn copy_planned_file(
        &self,
        resp_id: &str,
        files_total: u32,
        file: &PlannedFile,
        files_done_before: u32,
        done_before_bytes: u64,
        total_bytes: u64,
        transfer_start: Instant,
        source: &SessionClient,
        target: &SessionClient,
        client_transfer_id: &str,
        session: Arc<CancelSession>,
        on_event: &TransferEventCallback,
    ) -> anyhow::Result<u64> {
        if session.is_cancelled() {
            return Err(cancelled());
        }
        let file_path = file.source_path.clone();
        let transfer_total = file.size.unwrap_or(0);

        let mut reader = source.open_read(&file.source_path).await?;
        let total_for_writer = if transfer_total > 0 { Some(transfer_total) } else { None };
        let mut writer = target.open_write(&file.target_path, None, true, total_for_writer, Self::noop_transfer_handler()).await?;

        let mut transfer_done: u64 = 0;
        let mut last_report_bytes: u64 = 0;
        let mut last_report = Instant::now();
        let mut buf = vec![0u8; TRANSFER_CHUNK_BYTES];

        loop {
            if session.is_cancelled() {
                let _ = writer.shutdown().await;
                return Err(cancelled());
            }
            let cancel_notified = session.cancel.notified();
            tokio::pin!(cancel_notified);
            cancel_notified.as_mut().enable();

            let read_result = tokio::select! {
                biased;
                _ = &mut cancel_notified => {
                    let _ = writer.shutdown().await;
                    return Err(cancelled());
                }
                read_result = reader.read(&mut buf) => read_result,
            };
            let n = read_result?;
            if n == 0 {
                break;
            }
            writer.write_all(&buf[..n]).await?;
            transfer_done += n as u64;

            let now = Instant::now();
            let elapsed_ms = now.duration_since(last_report).as_millis() as u64;
            let bytes_since = transfer_done - last_report_bytes;
            if elapsed_ms < TRANSFER_PROGRESS_INTERVAL_MS && bytes_since < TRANSFER_PROGRESS_MIN_BYTES {
                continue;
            }
            let total_done = done_before_bytes + transfer_done;
            let elapsed_total_ms = transfer_start.elapsed().as_millis() as u64;
            let speed = if elapsed_total_ms > 0 { total_done * 1000 / elapsed_total_ms } else { 0 };
            let percent = compute_transfer_percent(total_done, total_bytes);
            let progress_going = resp_sftp::RespSftpTfProgress {
                speed,
                percent,
                files_total,
                finish: false,
                transfer_done,
                transfer_total,
                resp_id: resp_id.to_string(),
                file: file_path.clone(),
                files_done: files_done_before,
                client_transfer_id: client_transfer_id.to_string(),
            };
            on_event(RespEventSftp::OnTransferProgress(progress_going)).await;
            last_report = now;
            last_report_bytes = transfer_done;
        }
        writer.flush().await?;
        writer.shutdown().await?;
        if let Some(mode) = file.mode {
            if !matches!(target, SessionClient::Remote(RemoteSessionClient::Ftp(_))) {
                target.chmod(std::slice::from_ref(&file.target_path), mode & 0o7777).await?;
            }
        }
        Ok(transfer_done)
    }

    fn noop_transfer_handler() -> resp_sftp::RespSftpTransferProgressHandler {
        Arc::new(|_progress: resp_sftp::RespSftpTfProgress| Box::pin(async { Ok(()) }))
    }

    async fn confirm_transfer_overwrite(
        &self,
        path: &str,
        socket: SocketRef,
        data: &SftpTransferData,
        client_transfer_id: &str,
        req: Arc<ReqSocketContext>,
        session: Arc<CancelSession>,
    ) -> anyhow::Result<SftpTransferAnswer> {
        let request_id = uuid();
        let socket_id = socket.id.to_string();
        let inbox = Arc::new(SftpTransferInbox::new());
        let side_session_id = data.side_session_id.clone();
        self.transfer_answer_requests
            .entry(socket_id.clone())
            .or_insert_with(|| Arc::new(DashMap::new()))
            .entry(side_session_id.clone())
            .or_insert_with(|| Arc::new(DashMap::new()))
            .insert(request_id.clone(), Arc::clone(&inbox));

        let resp = resp_sftp::RespSftpTransferConflict { client_transfer_id: client_transfer_id.to_string(), request_id: request_id.clone(), path: path.to_string() };
        let event = RespEventSftp::OnTransferConflict(resp);
        self.sftp_service.handler_sftp_event(socket, req, event);

        let result = Self::wait_transfer_answer(&session.cancel, &inbox, &request_id).await;

        if let Some(socket_map) = self.transfer_answer_requests.get(&socket_id) {
            if let Some(request_map) = socket_map.get(&side_session_id) {
                request_map.remove(&request_id);
            }
        }
        result
    }

    async fn wait_transfer_answer(cancel: &Notify, inbox: &SftpTransferInbox, request_id: &str) -> anyhow::Result<SftpTransferAnswer> {
        loop {
            let notified = inbox.notify.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();

            let found = {
                let mut guard = inbox.queue.lock().await;
                let pos = guard.iter().position(|entry| entry.request_id == request_id);
                pos.and_then(|index| guard.remove(index)).map(|entry| entry.answer)
            };
            if let Some(answer) = found {
                return Ok(answer);
            }

            let sleep = tokio::time::sleep(Duration::from_millis(TRANSFER_CONFLICT_TIMEOUT_MS));
            tokio::pin!(sleep);
            tokio::select! {
                _ = &mut notified => {}
                _ = &mut sleep => {
                    anyhow::bail!("sftp transfer overwrite confirm timed out: request_id={}", request_id);
                }
                _ = cancel.notified() => {
                    info!("[sftp] 用户取消了request_id:[{}] 传输.",request_id);
                    return Err(cancelled());
                }
            }
        }
    }

    fn join_path(base: &str, name: &str) -> String {
        format!("{}/{}", base.trim_end_matches('/'), name)
    }
}
