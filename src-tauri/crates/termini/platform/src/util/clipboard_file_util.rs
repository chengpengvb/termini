use std::path::Path;
use crate::util::path_util;
use std::sync::atomic::{AtomicBool, Ordering};
use crate::model::resp::resp_sftp_transfer::{RespSftpFileCopy, RespSftpFilePaste, SFTP_FILE_COPY_SOURCE_KIND_OS};

static PASTE_STOP: AtomicBool = AtomicBool::new(false);
#[cfg(any(target_os = "windows", target_os = "linux"))]
static PASTE_STARTED: AtomicBool = AtomicBool::new(false);
static INPUT_INJECTION: AtomicBool = AtomicBool::new(false);

pub fn set_input_injection(active: bool) {
    INPUT_INJECTION.store(active, Ordering::SeqCst);
}

pub const APP_COPY_PLACEHOLDER_PREFIX: &str = "tmx-clip-";

#[cfg(any(target_os = "windows", target_os = "linux"))]
type BoxedOnPaste = Box<dyn Fn(RespSftpFilePaste) + Send + 'static>;

pub fn start_file_paste_watch<F>(on_paste: F)
where
    F: Fn(RespSftpFilePaste) + Send + 'static,
{
    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        let _ = on_paste;
    }
    #[cfg(any(target_os = "windows", target_os = "linux"))]
    {
        if PASTE_STARTED.swap(true, Ordering::SeqCst) {
            return;
        }
        #[cfg(windows)]
        win_paste_target::ensure_mapping();
        let on_paste: BoxedOnPaste = Box::new(on_paste);
        PASTE_STOP.store(false, Ordering::SeqCst);
        if let Ok(handle) = std::thread::Builder::new().name("file-paste-watch".into()).spawn(move || {
            run_paste_watch(on_paste);
        }) {
            crate::module::common::util::task_hub::TaskGroup::global().adopt_os(handle);
        }
    }
}

pub fn stop_file_paste_watch() {
    PASTE_STOP.store(true, Ordering::SeqCst);
}

#[cfg(any(target_os = "windows", target_os = "linux"))]
fn run_paste_watch(on_paste: BoxedOnPaste) {
    use std::sync::Arc;
    use rdev::{listen, EventType, Key};
    use summer::tracing::{error, info};

    info!("[file] starting global keyboard listener (rdev::listen)");

    let ctrl_down = Arc::new(AtomicBool::new(false));
    let meta_down = Arc::new(AtomicBool::new(false));
    let ctrl = Arc::clone(&ctrl_down);
    let meta = Arc::clone(&meta_down);
    let callback = on_paste;

    let result = listen(move |event| {
        if PASTE_STOP.load(Ordering::SeqCst) {
            return;
        }
        match event.event_type {
            EventType::KeyPress(Key::ControlLeft) | EventType::KeyPress(Key::ControlRight) => {
                ctrl.store(true, Ordering::SeqCst);
            }
            EventType::KeyRelease(Key::ControlLeft) | EventType::KeyRelease(Key::ControlRight) => {
                ctrl.store(false, Ordering::SeqCst);
            }
            EventType::KeyPress(Key::MetaLeft) | EventType::KeyPress(Key::MetaRight) => {
                info!("[file] meta key down observed by rdev listener");
                meta.store(true, Ordering::SeqCst);
            }
            EventType::KeyRelease(Key::MetaLeft) | EventType::KeyRelease(Key::MetaRight) => {
                meta.store(false, Ordering::SeqCst);
            }
            EventType::KeyPress(Key::KeyV) => {
                if INPUT_INJECTION.load(Ordering::SeqCst) {
                    return;
                }
                if !ctrl.load(Ordering::SeqCst) && !meta.load(Ordering::SeqCst) {
                    return;
                }
                if PASTE_STOP.load(Ordering::SeqCst) {
                    return;
                }
                let Some(paste) = read_clipboard_file_paste() else {
                    return;
                };
                callback(paste);
            }
            _ => {}
        }
    });
    if let Err(error) = result {
        error!("[file] rdev::listen exited with error: {:?}", error);
        PASTE_STARTED.store(false, Ordering::SeqCst);
    }
}

#[cfg(any(target_os = "windows", target_os = "linux"))]
fn read_clipboard_file_paste() -> Option<RespSftpFilePaste> {
    use clipboard_rs::{Clipboard, ClipboardContext};
    let ctx = ClipboardContext::new().ok()?;
    let files = ctx.get_files().ok()?;
    if files.is_empty() {
        return None;
    }
    #[cfg(windows)]
    {
        if let Some(target_path) = win_paste_target::wait_for_real_target() {
            return Some(RespSftpFilePaste { target_path, side_session_id: None, source_kind: SFTP_FILE_COPY_SOURCE_KIND_OS, target_client_transfer_id: String::new() });
        }
    }
    resolve_file_paste(files)
}

#[cfg(any(target_os = "android", target_os = "ios"))]
pub fn read_clipboard_local_paths() -> Vec<String> {
    Vec::new()
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub fn read_clipboard_local_paths() -> Vec<String> {
    use clipboard_rs::{Clipboard, ClipboardContext};
    let Ok(ctx) = ClipboardContext::new() else {
        return Vec::new();
    };
    let Ok(files) = ctx.get_files() else {
        return Vec::new();
    };
    files.iter().map(|raw| strip_file_uri(raw.as_str())).collect()
}

pub fn resolve_file_copy(files: Vec<String>) -> Option<RespSftpFileCopy> {
    let (names, source_path) = resolve_names_and_parent(files)?;
    Some(RespSftpFileCopy { names, source_path, side_session_id: None, source_kind: SFTP_FILE_COPY_SOURCE_KIND_OS, source_client_transfer_id: String::new() })
}

pub fn resolve_file_paste(files: Vec<String>) -> Option<RespSftpFilePaste> {
    let (_names, target_path) = resolve_names_and_parent(files)?;
    Some(RespSftpFilePaste { target_path, side_session_id: None, source_kind: SFTP_FILE_COPY_SOURCE_KIND_OS, target_client_transfer_id: String::new() })
}

fn resolve_names_and_parent(files: Vec<String>) -> Option<(Vec<String>, String)> {
    if files.is_empty() {
        return None;
    }
    let mut names = Vec::with_capacity(files.len());
    let mut parent_path: Option<String> = None;
    for raw in files {
        let path = strip_file_uri(raw.as_str());
        let path_obj = Path::new(path.as_str());
        let name = path_obj.file_name()?.to_string_lossy().into_owned();
        if name.is_empty() {
            return None;
        }
        let parent = path_obj.parent()?;
        let parent_norm = path_util::normalize_pathbuf_lossy(parent);
        if parent_norm.is_empty() {
            return None;
        }
        match &parent_path {
            None => parent_path = Some(parent_norm),
            Some(existing) => {
                if existing != &parent_norm {
                    return None;
                }
            }
        }
        names.push(name);
    }
    Some((names, parent_path?))
}

#[cfg(windows)]
mod win_paste_target {
    use std::ffi::c_void;
    use std::time::Duration;
    use std::sync::atomic::{AtomicUsize, Ordering};

    const SHM_SIZE: usize = 8192;
    const SHM_NAME: &[u8] = b"TerminiOsPasteMapping\0";
    const POLL_INTERVAL: Duration = Duration::from_millis(25);
    const POLL_ATTEMPTS: u32 = 40;

    static MAPPING_HANDLE: AtomicUsize = AtomicUsize::new(0);

    type Handle = *mut c_void;
    const PAGE_READWRITE: u32 = 0x04;
    const FILE_MAP_READ: u32 = 0x0004;

    fn invalid_handle() -> Handle {
        (-1_isize) as usize as Handle
    }

    extern "system" {
        fn UnmapViewOfFile(lpBaseAddress: *const c_void) -> i32;
        fn CreateFileMappingA(hFile: Handle, lpFileMappingAttributes: *const c_void, flProtect: u32, dwMaximumSizeHigh: u32, dwMaximumSizeLow: u32, lpName: *const u8) -> Handle;
        fn MapViewOfFile(hFileMappingObject: Handle, dwDesiredAccess: u32, dwFileOffsetHigh: u32, dwFileOffsetLow: u32, dwNumberOfBytesToMap: usize) -> *mut c_void;
    }

    pub fn ensure_mapping() {
        if MAPPING_HANDLE.load(Ordering::SeqCst) != 0 {
            return;
        }
        unsafe {
            let h = CreateFileMappingA(invalid_handle(), std::ptr::null(), PAGE_READWRITE, 0, SHM_SIZE as u32, SHM_NAME.as_ptr());
            if h.is_null() || h == invalid_handle() {
                return;
            }
            MAPPING_HANDLE.store(h as usize, Ordering::SeqCst);
        }
    }

    fn read_seq_and_path() -> Option<(u8, String)> {
        let h = MAPPING_HANDLE.load(Ordering::SeqCst) as Handle;
        if h.is_null() {
            return None;
        }
        unsafe {
            let ptr = MapViewOfFile(h, FILE_MAP_READ, 0, 0, SHM_SIZE) as *const u8;
            if ptr.is_null() {
                return None;
            }
            let seq = std::ptr::read(ptr);
            let data = std::slice::from_raw_parts(ptr.add(1), SHM_SIZE - 1);
            let end = data.iter().position(|&b| b == 0).unwrap_or(data.len());
            let path = String::from_utf8_lossy(&data[..end]).into_owned();
            UnmapViewOfFile(ptr as _);
            Some((seq, path))
        }
    }

    pub fn wait_for_real_target() -> Option<String> {
        let (start_seq, _) = read_seq_and_path()?;
        for _ in 0..POLL_ATTEMPTS {
            std::thread::sleep(POLL_INTERVAL);
            let (seq, path) = read_seq_and_path()?;
            if seq != start_seq && !path.is_empty() {
                return Some(path);
            }
        }
        None
    }
}

pub fn strip_file_uri(raw: &str) -> String {
    let trimmed = raw.trim();
    if let Some(rest) = trimmed.strip_prefix("file://") {
        #[cfg(windows)]
        {
            let rest = rest.trim_start_matches('/');
            if rest.len() >= 2 && rest.as_bytes()[1] == b':' {
                return path_util::normalize_frontend_path(rest);
            }
        }
        return path_util::normalize_frontend_path(rest);
    }
    path_util::normalize_frontend_path(trimmed)
}
