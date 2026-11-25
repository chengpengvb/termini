use anyhow::Result;
use std::sync::Arc;
#[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
use tauri::Manager;
use tauri::AppHandle;
use parking_lot::Mutex;
#[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
use std::path::PathBuf;
use std::sync::OnceLock;
use summer::plugin::Service;
use async_trait::async_trait;
use crate::util::drop_target;
use termini_macros::base_app;
use summer::tracing::{info, warn};
use crate::util::clipboard_file_util;
use tokio::runtime::Handle as TokioHandle;
use crate::model::resp::resp_sftp_transfer;
use crate::model::resp::resp_model::ResultCode;
use crate::base::base_app::{AppContext, BaseApp};
use crate::model::req::req_ws_event::ReqWsEventSftp;
use crate::model::resp::resp_sftp::RespSftpRemoteDrag;
use crate::model::req::req_ws_context::ReqSocketContext;
use crate::service::service_clipboard::ClipboardService;
use crate::web::ws::handle::r#impl::handle_sftp::HandleSftp;
use crate::util::clipboard_file_util::APP_COPY_PLACEHOLDER_PREFIX;
use crate::model::req::event::ws_sftp::{SftpTransferData, TransferEvent};
use crate::model::dto::clipboard::{ClipboardEvent, ClipboardSubscription};
use crate::model::resp::resp_sftp_transfer::{RespSftpFileCopy, RespSftpFilePaste};

fn create_file_copy() -> Arc<Mutex<RespSftpFileCopy>> {
    Arc::new(Mutex::new(RespSftpFileCopy::default()))
}

fn create_file_paste() -> Arc<Mutex<RespSftpFilePaste>> {
    Arc::new(Mutex::new(RespSftpFilePaste::default()))
}

fn create_tokio_runtime() -> Arc<OnceLock<TokioHandle>> {
    Arc::new(OnceLock::new())
}

fn create_copy_subscription() -> Arc<Mutex<Option<ClipboardSubscription>>> {
    Arc::new(Mutex::new(None))
}

#[cfg(windows)]
fn create_clip_placeholder() -> Arc<Mutex<Option<PathBuf>>> {
    Arc::new(Mutex::new(None))
}

fn is_own_clip_placeholder(copy: &RespSftpFileCopy) -> bool {
    match copy.names.first() {
        Some(name) => copy.names.len() == 1 && name.trim_start_matches('.').starts_with(APP_COPY_PLACEHOLDER_PREFIX),
        None => false,
    }
}

#[base_app(sort = 903)]
#[derive(Clone, Service)]
pub struct SftpDragService {
    #[inject(component)]
    handle: AppHandle,

    #[inject(component)]
    sftp_handler: HandleSftp,

    #[inject(component)]
    clipboard_service: ClipboardService,

    #[inject(func = create_file_copy())]
    file_copy: Arc<Mutex<RespSftpFileCopy>>,

    #[inject(func = create_file_paste())]
    file_paste: Arc<Mutex<RespSftpFilePaste>>,

    #[inject(func = create_tokio_runtime())]
    tokio_runtime: Arc<OnceLock<TokioHandle>>,

    #[cfg(windows)]
    #[inject(func = create_clip_placeholder())]
    clip_placeholder: Arc<Mutex<Option<PathBuf>>>,

    #[inject(func = create_copy_subscription())]
    copy_subscription: Arc<Mutex<Option<ClipboardSubscription>>>,
}

#[base_app(sort = 1001)]
#[derive(Clone, Service)]
pub struct SftpDragShellService;

#[async_trait]
impl BaseApp for SftpDragService {
    fn name(&self) -> &'static str {
        "SftpDragService"
    }

    async fn start(&self, _ctx: &AppContext) -> Result<()> {
        let _ = self.tokio_runtime.set(TokioHandle::current());
        #[cfg(any(windows, target_os = "macos"))]
        Self::cleanup_stale_placeholders();
        self.start_file_copy_listener();
        self.start_file_paste_listener();
        Ok(())
    }

    async fn on_exit(&self, _ctx: &AppContext) -> Result<()> {
        clipboard_file_util::stop_file_paste_watch();
        Ok(())
    }
}

#[async_trait]
impl BaseApp for SftpDragShellService {
    fn name(&self) -> &'static str {
        "SftpDragShellService"
    }

    async fn start(&self, _ctx: &AppContext) -> Result<()> {
        #[cfg(windows)]
        {
            use crate::util::drop_target::windows::auto_register_shellext;
            let Ok(exe) = std::env::current_exe() else {
                return Ok(());
            };
            let Some(exe_dir) = exe.parent() else {
                return Ok(());
            };
            auto_register_shellext(exe_dir);
        }
        Ok(())
    }
}

impl SftpDragService {
    pub fn start_drag(&self, paths: Vec<String>) -> Result<()> {
        let handle = self.handle.clone();
        self.handle
            .run_on_main_thread(move || {
                let _ = start_local_drag_on_main(handle, paths);
            })
            .map_err(|e| anyhow::anyhow!("{}", e))
    }

    pub fn start_remote_drag(&self, req: RespSftpRemoteDrag) -> Result<()> {
        drop_target::start_remote_drag(self.handle.clone(), self.sftp_handler.clone(), req)
    }

    pub fn set_file_copy(&self, mut copy: RespSftpFileCopy, source_kind: i8) {
        copy.source_kind = source_kind;
        info!(
            "[sftp] set names={:?} source_path={} source_kind={} source_client_transfer_id={} side_session_id={:?}",
            copy.names, copy.source_path, copy.source_kind, copy.source_client_transfer_id, copy.side_session_id
        );
        *self.file_copy.lock() = copy;
        if source_kind == resp_sftp_transfer::SFTP_FILE_COPY_SOURCE_KIND_APP {
            self.publish_app_copy_placeholder();
        }
    }

    #[cfg(any(windows, target_os = "macos"))]
    fn cleanup_stale_placeholders() {
        let Ok(entries) = std::fs::read_dir(std::env::temp_dir()) else {
            return;
        };
        for entry in entries.flatten() {
            let name = entry.file_name();
            let Some(name) = name.to_str() else {
                continue;
            };
            if name.trim_start_matches('.').starts_with(APP_COPY_PLACEHOLDER_PREFIX) {
                let _ = std::fs::remove_dir_all(entry.path());
            }
        }
    }

    #[cfg(windows)]
    fn publish_app_copy_placeholder(&self) {
        use clipboard_rs::{Clipboard, ClipboardContext};
        self.clipboard_service.mark_self_write();
        let uid = uuid::Uuid::new_v4().to_string().replace('-', "");
        let dir_name = format!("{}{}", APP_COPY_PLACEHOLDER_PREFIX, uid);
        let placeholder = std::env::temp_dir().join(dir_name);
        if std::fs::create_dir_all(&placeholder).is_err() {
            return;
        }
        let Some(placeholder_str) = placeholder.to_str() else {
            let _ = std::fs::remove_dir_all(&placeholder);
            return;
        };
        let Ok(ctx) = ClipboardContext::new() else {
            let _ = std::fs::remove_dir_all(&placeholder);
            return;
        };
        if ctx.set_files(vec![placeholder_str.to_string()]).is_err() {
            let _ = std::fs::remove_dir_all(&placeholder);
            return;
        }
        info!("[sftp] published app copy placeholder {}", placeholder_str);
        let previous = self.clip_placeholder.lock().replace(placeholder);
        if let Some(old) = previous {
            let _ = std::fs::remove_dir_all(&old);
        }
    }

    #[cfg(not(windows))]
    fn publish_app_copy_placeholder(&self) {}

    pub fn set_file_paste(&self, mut paste: RespSftpFilePaste, source_kind: i8) {
        paste.source_kind = source_kind;
        info!(
            "[sftp] set target_path={} source_kind={} target_client_transfer_id={} side_session_id={:?}",
            paste.target_path, paste.source_kind, paste.target_client_transfer_id, paste.side_session_id
        );
        *self.file_paste.lock() = paste;
        self.try_execute_clipboard_transfer();
    }

    fn try_execute_clipboard_transfer(&self) {
        let copy = self.file_copy.lock().clone();
        let paste = self.file_paste.lock().clone();
        let Some(side_session_id) = Self::resolve_transfer_side_session_id(&copy, &paste) else {
            return;
        };
        if copy.names.is_empty() || copy.source_path.is_empty() || paste.target_path.is_empty() || copy.source_client_transfer_id.is_empty() || paste.target_client_transfer_id.is_empty() {
            return;
        }
        let data = SftpTransferData {
            names: copy.names,
            source_path: copy.source_path,
            target_path: paste.target_path,
            side_session_id: side_session_id.clone(),
            source_client_transfer_id: copy.source_client_transfer_id,
            target_client_transfer_id: paste.target_client_transfer_id,
        };
        info!("[sftp] 系统复制粘贴文件:{:?}", data);
        let payload = serde_json::to_value(TransferEvent { data: data.clone() }).ok();
        let req = Arc::new(ReqSocketContext { r#type: ReqWsEventSftp::Transfer as i16, session_id: side_session_id, payload });
        self.spawn_clipboard_transfer(req, data);
    }

    fn resolve_transfer_side_session_id(copy: &RespSftpFileCopy, paste: &RespSftpFilePaste) -> Option<String> {
        let copy_is_os = copy.source_kind == resp_sftp_transfer::SFTP_FILE_COPY_SOURCE_KIND_OS;
        let paste_is_os = paste.source_kind == resp_sftp_transfer::SFTP_FILE_COPY_SOURCE_KIND_OS;
        let copy_is_app = copy.source_kind == resp_sftp_transfer::SFTP_FILE_COPY_SOURCE_KIND_APP;
        let paste_is_app = paste.source_kind == resp_sftp_transfer::SFTP_FILE_COPY_SOURCE_KIND_APP;
        if copy_is_os && paste_is_os {
            return None;
        }
        if copy_is_os && paste_is_app {
            return Self::take_non_empty_session_id(&paste.side_session_id);
        }
        if copy_is_app && paste_is_os {
            return Self::take_non_empty_session_id(&copy.side_session_id);
        }
        if copy_is_app && paste_is_app {
            return Self::take_non_empty_session_id(&paste.side_session_id);
        }
        None
    }

    fn take_non_empty_session_id(side_session_id: &Option<String>) -> Option<String> {
        match side_session_id {
            Some(id) if !id.is_empty() => Some(id.clone()),
            _ => None,
        }
    }

    fn spawn_clipboard_transfer(&self, req: Arc<ReqSocketContext>, data: SftpTransferData) {
        let handler = self.sftp_handler.clone();
        let runtime = match TokioHandle::try_current() {
            Ok(rt) => rt,
            Err(_) => match self.tokio_runtime.get() {
                Some(rt) => rt.clone(),
                None => {
                    warn!("[sftp] tokio runtime missing, skip transfer");
                    return;
                }
            },
        };
        runtime.spawn(async move {
            let result = match handler.resolve_socket_by_session(&req.session_id) {
                Ok(socket) => handler.do_execute_transfer(socket, req, data, None).await,
                Err(error) => Err(error),
            };
            if let Err(error) = result {
                warn!("[sftp] execute_transfer failed: {:#}", error);
            }
        });
    }

    fn start_file_copy_listener(&self) {
        let service = self.clone();
        let subscription = self.clipboard_service.subscribe(move |event| match event {
            ClipboardEvent::Unavailable => {
                service.clear_file_copy();
            }
            ClipboardEvent::Snapshot(snapshot) => match clipboard_file_util::resolve_file_copy(snapshot.files.clone()) {
                Some(mut copy) => {
                    if is_own_clip_placeholder(&copy) {
                        return;
                    }
                    copy.side_session_id = None;
                    copy.source_client_transfer_id = service.sftp_handler.local_client_transfer_id();
                    service.set_file_copy(copy, resp_sftp_transfer::SFTP_FILE_COPY_SOURCE_KIND_OS);
                }
                None => service.clear_file_copy(),
            },
        });
        *self.copy_subscription.lock() = Some(subscription);
    }

    fn clear_file_copy(&self) {
        *self.file_copy.lock() = RespSftpFileCopy::default();
    }

    fn start_file_paste_listener(&self) {
        let service = self.clone();
        clipboard_file_util::start_file_paste_watch(move |mut paste| {
            paste.side_session_id = None;
            paste.target_client_transfer_id = service.sftp_handler.local_client_transfer_id();
            service.set_file_paste(paste, resp_sftp_transfer::SFTP_FILE_COPY_SOURCE_KIND_OS);
        });
    }
}

fn start_local_drag_on_main(handle: AppHandle, paths: Vec<String>) -> Result<()> {
    #[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
    {
        let image = drag::Image::Raw(vec![]);
        let item = drag::DragItem::Files(paths.iter().map(PathBuf::from).collect());
        let window = handle.get_webview_window("main").ok_or_else(|| anyhow::Error::from(ResultCode::MainWindowNotFound))?;
        #[cfg(not(target_os = "linux"))]
        {
            return drag::start_drag(&window, item, image, |_, _| {}, drag::Options::default()).map_err(|e| anyhow::anyhow!("{}", e));
        }
        #[cfg(target_os = "linux")]
        {
            let gtk_win = window.gtk_window().map_err(|e| anyhow::anyhow!("{}", e))?;
            return drag::start_drag(&gtk_win, item, image, |_, _| {}, drag::Options::default()).map_err(|e| anyhow::anyhow!("{}", e));
        }
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        let _ = (handle, paths);
        Err(ResultCode::DragUnsupported.into())
    }
}
