use anyhow::Result;
use std::sync::Arc;
use tauri::Manager;
use std::ffi::c_void;
use std::sync::atomic::{AtomicUsize, Ordering};
use crate::model::req::req_ws_event::ReqWsEventSftp;
use crate::model::resp::resp_sftp::RespSftpRemoteDrag;
use crate::model::req::req_ws_context::ReqSocketContext;
use crate::web::ws::handle::r#impl::handle_sftp::HandleSftp;
use crate::model::req::event::ws_sftp::{SftpTransferData, TransferEvent};

const SHM_SIZE: usize = 8192;
const STATUS_WAITING: u8 = 1;
const STATUS_GOT_TARGET: u8 = 2;
const SHM_NAME: &[u8] = b"TerminiOsDragMapping\0";

static MAPPING_HANDLE: AtomicUsize = AtomicUsize::new(0);

type Handle = *mut c_void;
const PAGE_READWRITE: u32 = 0x04;
const FILE_MAP_READ: u32 = 0x0004;
const FILE_MAP_WRITE: u32 = 0x0002;

fn invalid_handle() -> Handle {
    (-1_isize) as usize as Handle
}

extern "system" {
    fn CloseHandle(hObject: Handle) -> i32;

    fn FreeLibrary(hLibModule: Handle) -> i32;

    fn LoadLibraryW(lpLibFileName: *const u16) -> Handle;

    fn UnmapViewOfFile(lpBaseAddress: *const c_void) -> i32;

    fn GetProcAddress(hModule: Handle, lpProcName: *const u8) -> *mut c_void;

    fn CreateFileMappingA(hFile: Handle, lpFileMappingAttributes: *const c_void, flProtect: u32, dwMaximumSizeHigh: u32, dwMaximumSizeLow: u32, lpName: *const u8) -> Handle;

    fn MapViewOfFile(hFileMappingObject: Handle, dwDesiredAccess: u32, dwFileOffsetHigh: u32, dwFileOffsetLow: u32, dwNumberOfBytesToMap: usize) -> *mut c_void;

}

pub fn start_remote_drag(handle: tauri::AppHandle, sftp_handler: HandleSftp, req: RespSftpRemoteDrag) -> Result<()> {
    let uid = uuid::Uuid::new_v4().to_string().replace('-', "");
    let temp_dir = std::env::temp_dir().join(format!("termini-{}", uid));
    std::fs::create_dir_all(&temp_dir)?;
    let temp_dir_str = temp_dir.to_str().ok_or_else(|| anyhow::anyhow!("non-utf8 temp path"))?.to_string();

    shm_set_waiting(&temp_dir_str)?;

    let rt = tokio::runtime::Handle::current();
    let fake_dir_item = vec![temp_dir.clone()];

    let handle_for_closure = handle.clone();
    std::thread::spawn(move || {
        let _ = handle.run_on_main_thread(move || {
            let Some(window) = handle_for_closure.get_webview_window("main") else {
                return;
            };
            let image = drag::Image::Raw(vec![]);
            let item = drag::DragItem::Files(fake_dir_item);
            let _ = drag::start_drag(
                &window,
                item,
                image,
                move |_result, cursor| {
                    let target = shm_read_target();
                    summer::tracing::info!("[file] callback fired, cursor={:?}, shm_target={:?}", cursor, target);
                    let _ = std::fs::remove_dir_all(&temp_dir);
                    shm_close();

                    let Some(target_path) = target else {
                        summer::tracing::warn!("[file] no target — shellext not registered?");
                        return;
                    };
                    let data = SftpTransferData {
                        target_path,
                        names: req.names.clone(),
                        source_path: req.source_path.clone(),
                        side_session_id: req.session_id.clone(),
                        source_client_transfer_id: req.client_transfer_id.clone(),
                        target_client_transfer_id: req.local_client_transfer_id.clone(),
                    };
                    let sftp = sftp_handler.clone();
                    let session_id = req.session_id.clone();
                    rt.spawn(async move {
                        let payload = serde_json::to_value(TransferEvent { data: data.clone() }).ok();
                        let request = Arc::new(ReqSocketContext { r#type: ReqWsEventSftp::Transfer as i16, session_id: session_id.clone(), payload });
                        let result = match sftp.resolve_socket_by_session(&session_id) {
                            Ok(socket) => sftp.do_execute_transfer(socket, request, data, None).await,
                            Err(error) => Err(error),
                        };
                        if let Err(e) = result {
                            summer::tracing::error!("[file] drag transfer failed: {e}");
                        }
                    });
                },
                drag::Options::default(),
            );
        });
    });

    Ok(())
}

fn shm_set_waiting(source_dir: &str) -> Result<()> {
    unsafe {
        let h = CreateFileMappingA(invalid_handle(), std::ptr::null(), PAGE_READWRITE, 0, SHM_SIZE as u32, SHM_NAME.as_ptr());
        if h.is_null() || h == invalid_handle() {
            return Err(anyhow::anyhow!("CreateFileMappingA failed"));
        }
        let ptr = MapViewOfFile(h, FILE_MAP_WRITE, 0, 0, SHM_SIZE) as *mut u8;
        if ptr.is_null() {
            CloseHandle(h);
            return Err(anyhow::anyhow!("MapViewOfFile failed"));
        }
        std::ptr::write(ptr, STATUS_WAITING);
        let src = source_dir.as_bytes();
        let len = src.len().min(SHM_SIZE - 2);
        std::ptr::copy_nonoverlapping(src.as_ptr(), ptr.add(1), len);
        std::ptr::write(ptr.add(1 + len), 0u8);
        UnmapViewOfFile(ptr as _);
        MAPPING_HANDLE.store(h as usize, Ordering::SeqCst);
    }
    Ok(())
}

fn shm_read_target() -> Option<String> {
    unsafe {
        let h = MAPPING_HANDLE.load(Ordering::SeqCst) as Handle;
        if h.is_null() {
            return None;
        }
        let ptr = MapViewOfFile(h, FILE_MAP_READ, 0, 0, SHM_SIZE) as *const u8;
        if ptr.is_null() {
            return None;
        }
        let status = std::ptr::read(ptr);
        let result = if status == STATUS_GOT_TARGET {
            let tgt = ptr.add(4096);
            let slice = std::slice::from_raw_parts(tgt, SHM_SIZE - 4096);
            let end = slice.iter().position(|&b| b == 0).unwrap_or(slice.len());
            let s = String::from_utf8_lossy(&slice[..end]).into_owned();
            if s.is_empty() {
                None
            } else {
                Some(s)
            }
        } else {
            None
        };
        UnmapViewOfFile(ptr as _);
        result
    }
}

fn shm_close() {
    unsafe {
        let h = MAPPING_HANDLE.swap(0, Ordering::SeqCst) as Handle;
        if !h.is_null() {
            CloseHandle(h);
        }
    }
}

pub fn auto_register_shellext(exe_dir: &std::path::Path) {
    use winreg::RegKey;
    use winreg::enums::{HKEY_CURRENT_USER, KEY_READ};

    let dll_path = exe_dir.join("shellext.dll");
    if !dll_path.exists() {
        summer::tracing::debug!("[shellext] DLL not found at {:?}, skip", dll_path);
        return;
    }
    let dll_path_str = match dll_path.to_str() {
        Some(s) => s.to_string(),
        None => return,
    };

    const CLSID: &str = "{A8B3F421-9C3E-4D2E-B5F1-0123456789AB}";
    let reg_key = format!(r"SOFTWARE\Classes\CLSID\{}\InProcServer32", CLSID);
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);

    if let Ok(key) = hkcu.open_subkey_with_flags(&reg_key, KEY_READ) {
        if let Ok(registered) = key.get_value::<String, _>("") {
            if registered.eq_ignore_ascii_case(&dll_path_str) {
                summer::tracing::debug!("[shellext] already registered, skip");
                return;
            }
        }
    }

    summer::tracing::info!("[shellext] registering {:?}", dll_path_str);
    unsafe { call_dll_register_server(&dll_path_str) };
}

unsafe fn call_dll_register_server(dll_path: &str) {
    let wide: Vec<u16> = dll_path.encode_utf16().chain([0u16]).collect();
    let h = LoadLibraryW(wide.as_ptr());
    if h.is_null() {
        summer::tracing::error!("[shellext] LoadLibraryW failed");
        return;
    }
    let fn_ptr = GetProcAddress(h, b"DllRegisterServer\0".as_ptr());
    if fn_ptr.is_null() {
        summer::tracing::error!("[shellext] DllRegisterServer not found in DLL");
        FreeLibrary(h);
        return;
    }
    type RegFn = unsafe extern "system" fn() -> i32;
    let register: RegFn = std::mem::transmute(fn_ptr);
    let hr = register();
    if hr == 0 {
        summer::tracing::info!("[shellext] DllRegisterServer succeeded");
    } else {
        summer::tracing::error!("[shellext] DllRegisterServer failed: HRESULT={:#010x}", hr);
    }
    FreeLibrary(h);
}
