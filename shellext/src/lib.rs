#![cfg(windows)]
#![allow(non_snake_case)]
use std::ffi::c_void;
use std::io::Write;
use windows::{
    core::{implement, IUnknown, Interface, Result, BOOL, GUID, HRESULT, PCSTR, PCWSTR},
    Win32::{
        Foundation::{CloseHandle, CLASS_E_NOAGGREGATION, E_NOINTERFACE, HWND, S_OK},
        System::{
            Com::{IClassFactory, IClassFactory_Impl},
            Memory::{
                MapViewOfFile, OpenFileMappingA, UnmapViewOfFile, FILE_MAP_ALL_ACCESS,
                MEMORY_MAPPED_VIEW_ADDRESS,
            },
        },
        UI::Shell::{ICopyHookW, ICopyHookW_Impl},
    },
};

const SHM_SIZE: usize = 8192;
const STATUS_WAITING: u8 = 1;
const STATUS_GOT_TARGET: u8 = 2;
const DRAG_PREFIX: &str = "termini-";
const CLIP_PREFIX: &str = "tmx-clip-";
const SHM_NAME: &[u8] = b"TerminiOsDragMapping\0";
const PASTE_SHM_NAME: &[u8] = b"TerminiOsPasteMapping\0";

fn log_line(msg: &str) {
    let path = std::env::temp_dir().join("termini-shellext.log");
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(f, "[{:?}] {}", std::time::SystemTime::now(), msg);
    }
}

const CLSID_TERMINI_COPY_HOOK: GUID = GUID {
    data1: 0xA8B3F421,
    data2: 0x9C3E,
    data3: 0x4D2E,
    data4: [0xB5, 0xF1, 0x01, 0x23, 0x45, 0x67, 0x89, 0xAB],
};

#[implement(ICopyHookW)]
struct TerminiCopyHook;

impl ICopyHookW_Impl for TerminiCopyHook_Impl {
    fn CopyCallback(
        &self,
        _hwnd: HWND,
        wfunc: u32,
        _wflags: u32,
        pszsrcfile: &PCWSTR,
        _dwsrcattribs: u32,
        pszdestfile: &PCWSTR,
        _dwdestattribs: u32,
    ) -> u32 {
        const FO_MOVE: u32 = 0x0001;
        const FO_COPY: u32 = 0x0002;
        const IDYES: u32 = 1;
        const IDNO: u32 = 2;

        let src = unsafe { pszsrcfile.to_string().unwrap_or_default() };
        let dst = unsafe { pszdestfile.to_string().unwrap_or_default() };
        log_line(&format!("CopyCallback fired wfunc={:#x} src={} dst={}", wfunc, src, dst));

        if wfunc != FO_COPY && wfunc != FO_MOVE {
            log_line("skip: wfunc is not FO_COPY/FO_MOVE");
            return IDYES;
        }

        let src_name = src
            .replace('\\', "/")
            .rsplit('/')
            .next()
            .unwrap_or_default()
            .to_lowercase();
        if src_name.starts_with(DRAG_PREFIX) {
            let written = unsafe { try_write_drag_target(*pszdestfile) };
            log_line(&format!("try_write_drag_target result={}", written));
            return if written { IDNO } else { IDYES };
        }

        if src_name.starts_with(CLIP_PREFIX) {
            // app 内复制→系统粘贴时塞进剪贴板的占位目录：只借它拿到真实目标目录，
            // 不能让这个空占位目录真的被复制到目标位置。
            let written = unsafe { try_write_paste_target(*pszdestfile) };
            log_line(&format!("[clip] try_write_paste_target result={}", written));
            return if written { IDNO } else { IDYES };
        }

        // 常规 OS 文件操作（含 Ctrl+V 粘贴）：不拦截，只把真实目标目录广播出去，
        // 供 app 端替换掉"读剪贴板文件父目录"的不可靠猜测。
        let written = unsafe { try_write_paste_target(*pszdestfile) };
        log_line(&format!("try_write_paste_target result={}", written));
        IDYES
    }
}

unsafe fn try_write_paste_target(dest_pcwstr: PCWSTR) -> bool {
    let h = match OpenFileMappingA(FILE_MAP_ALL_ACCESS.0, false, PCSTR(PASTE_SHM_NAME.as_ptr())) {
        Ok(h) => h,
        Err(e) => {
            log_line(&format!("[paste] OpenFileMappingA failed: {:?}", e));
            return false;
        }
    };

    let view = MapViewOfFile(h, FILE_MAP_ALL_ACCESS, 0, 0, SHM_SIZE);
    let ptr = view.Value as *mut u8;
    if ptr.is_null() {
        log_line("[paste] MapViewOfFile returned null");
        let _ = CloseHandle(h);
        return false;
    }

    let dest = dest_pcwstr.to_string().unwrap_or_default();
    let target_dir = parent_dir(&dest);
    let target_bytes = target_dir.as_bytes();
    let tgt_len = target_bytes.len().min(SHM_SIZE - 2);

    let data_ptr = ptr.add(1);
    std::ptr::copy_nonoverlapping(target_bytes.as_ptr(), data_ptr, tgt_len);
    std::ptr::write(data_ptr.add(tgt_len), 0u8);
    let next_seq = std::ptr::read(ptr).wrapping_add(1);
    std::ptr::write(ptr, next_seq);

    let _ = UnmapViewOfFile(MEMORY_MAPPED_VIEW_ADDRESS { Value: ptr as _ });
    let _ = CloseHandle(h);
    true
}

unsafe fn try_write_drag_target(dest_pcwstr: PCWSTR) -> bool {
    let h = match OpenFileMappingA(FILE_MAP_ALL_ACCESS.0, false, PCSTR(SHM_NAME.as_ptr())) {
        Ok(h) => h,
        Err(e) => {
            log_line(&format!("OpenFileMappingA failed: {:?}", e));
            return false;
        }
    };

    let view = MapViewOfFile(h, FILE_MAP_ALL_ACCESS, 0, 0, SHM_SIZE);
    let ptr = view.Value as *mut u8;
    if ptr.is_null() {
        log_line("MapViewOfFile returned null");
        let _ = CloseHandle(h);
        return false;
    }

    let status = std::ptr::read(ptr);
    if status != STATUS_WAITING {
        log_line(&format!("status mismatch: expected WAITING(1), got {}", status));
        let _ = UnmapViewOfFile(MEMORY_MAPPED_VIEW_ADDRESS { Value: ptr as _ });
        let _ = CloseHandle(h);
        return false;
    }

    let dest = dest_pcwstr.to_string().unwrap_or_default();
    let target_dir = parent_dir(&dest);
    let target_bytes = target_dir.as_bytes();
    let tgt_len = target_bytes.len().min(SHM_SIZE - 4097);

    let tgt_ptr = ptr.add(4096);
    std::ptr::copy_nonoverlapping(target_bytes.as_ptr(), tgt_ptr, tgt_len);
    std::ptr::write(tgt_ptr.add(tgt_len), 0u8);
    std::ptr::write(ptr, STATUS_GOT_TARGET);

    let _ = UnmapViewOfFile(MEMORY_MAPPED_VIEW_ADDRESS { Value: ptr as _ });
    let _ = CloseHandle(h);
    true
}

fn parent_dir(path: &str) -> String {
    let p = path.trim_end_matches(['\\', '/']);
    match p.rfind(['\\', '/']) {
        Some(pos) => p[..pos].to_string(),
        None => path.to_string(),
    }
}

#[implement(IClassFactory)]
struct TerminiClassFactory;

impl IClassFactory_Impl for TerminiClassFactory_Impl {
    fn CreateInstance(
        &self,
        punkouter: windows::core::Ref<'_, IUnknown>,
        riid: *const GUID,
        ppvobject: *mut *mut c_void,
    ) -> Result<()> {
        if !punkouter.is_null() {
            return Err(windows::core::Error::from(CLASS_E_NOAGGREGATION));
        }
        let hook: ICopyHookW = TerminiCopyHook.into();
        unsafe { hook.query(riid, ppvobject).ok() }
    }

    fn LockServer(&self, _flock: BOOL) -> Result<()> {
        Ok(())
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "system" fn DllGetClassObject(
    rclsid: *const GUID,
    riid: *const GUID,
    ppv: *mut *mut c_void,
) -> HRESULT {
    if *rclsid != CLSID_TERMINI_COPY_HOOK {
        return E_NOINTERFACE;
    }
    let factory: IClassFactory = TerminiClassFactory.into();
    factory.query(riid, ppv)
}

#[unsafe(no_mangle)]
pub unsafe extern "system" fn DllRegisterServer() -> HRESULT {
    reg_write()
}

#[unsafe(no_mangle)]
pub unsafe extern "system" fn DllUnregisterServer() -> HRESULT {
    reg_delete()
}

fn clsid_str() -> String {
    format!(
        "{{{:08X}-{:04X}-{:04X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}}}",
        CLSID_TERMINI_COPY_HOOK.data1,
        CLSID_TERMINI_COPY_HOOK.data2,
        CLSID_TERMINI_COPY_HOOK.data3,
        CLSID_TERMINI_COPY_HOOK.data4[0],
        CLSID_TERMINI_COPY_HOOK.data4[1],
        CLSID_TERMINI_COPY_HOOK.data4[2],
        CLSID_TERMINI_COPY_HOOK.data4[3],
        CLSID_TERMINI_COPY_HOOK.data4[4],
        CLSID_TERMINI_COPY_HOOK.data4[5],
        CLSID_TERMINI_COPY_HOOK.data4[6],
        CLSID_TERMINI_COPY_HOOK.data4[7],
    )
}

fn dll_path() -> String {
    use windows::Win32::Foundation::HMODULE;
    use windows::Win32::System::LibraryLoader::{
        GetModuleFileNameW, GetModuleHandleExW, GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS,
        GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
    };

    let mut hmodule = HMODULE::default();
    unsafe {
        let _ = GetModuleHandleExW(
            GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
            windows::core::PCWSTR(dll_path as *const () as *const u16),
            &mut hmodule,
        );
    }
    let mut buf = vec![0u16; 260];
    let len = unsafe { GetModuleFileNameW(Some(hmodule), &mut buf) };
    buf.truncate(len as usize);
    String::from_utf16_lossy(&buf)
}

unsafe fn reg_write() -> HRESULT {
    use windows::Win32::System::Registry::*;
    let clsid = clsid_str();
    let path = dll_path();
    let inproc_key = format!("SOFTWARE\\Classes\\CLSID\\{}\\InProcServer32", clsid);
    let hook_key =
        "SOFTWARE\\Classes\\Directory\\shellex\\CopyHookHandlers\\TerminiDrag".to_string();
    if reg_set_str(HKEY_CURRENT_USER, &inproc_key, "", &path).is_err() {
        return windows::core::HRESULT(-2147024774i32); // ERROR_REGISTRY_IO_FAILED
    }
    if reg_set_str(
        HKEY_CURRENT_USER,
        &inproc_key,
        "ThreadingModel",
        "Apartment",
    )
    .is_err()
    {
        return windows::core::HRESULT(-2147024774i32);
    }
    if reg_set_str(HKEY_CURRENT_USER, &hook_key, "", &clsid).is_err() {
        return windows::core::HRESULT(-2147024774i32);
    }
    S_OK
}

unsafe fn reg_delete() -> HRESULT {
    use windows::Win32::System::Registry::*;
    let clsid = clsid_str();
    let inproc_key = format!("SOFTWARE\\Classes\\CLSID\\{}\\InProcServer32", clsid);
    let clsid_key = format!("SOFTWARE\\Classes\\CLSID\\{}", clsid);
    let hook_key =
        "SOFTWARE\\Classes\\Directory\\shellex\\CopyHookHandlers\\TerminiDrag".to_string();
    let _ = reg_delete_key(HKEY_CURRENT_USER, &hook_key);
    let _ = reg_delete_key(HKEY_CURRENT_USER, &inproc_key);
    let _ = reg_delete_key(HKEY_CURRENT_USER, &clsid_key);
    S_OK
}

fn reg_set_str(
    root: windows::Win32::System::Registry::HKEY,
    key_path: &str,
    value_name: &str,
    data: &str,
) -> Result<()> {
    use windows::Win32::System::Registry::*;
    let key_wide: Vec<u16> = key_path.encode_utf16().chain([0]).collect();
    let val_wide: Vec<u16> = value_name.encode_utf16().chain([0]).collect();
    let data_wide: Vec<u16> = data.encode_utf16().chain([0]).collect();
    unsafe {
        let mut hkey = HKEY::default();
        RegCreateKeyExW(
            root,
            windows::core::PCWSTR(key_wide.as_ptr()),
            Some(0),
            None,
            REG_OPTION_NON_VOLATILE,
            KEY_WRITE,
            None,
            &mut hkey,
            None,
        )
        .ok()?;
        RegSetValueExW(
            hkey,
            windows::core::PCWSTR(val_wide.as_ptr()),
            Some(0),
            REG_SZ,
            Some(std::slice::from_raw_parts(
                data_wide.as_ptr() as *const u8,
                data_wide.len() * 2,
            )),
        )
        .ok()?;
        RegCloseKey(hkey).ok()?;
    }
    Ok(())
}

fn reg_delete_key(root: windows::Win32::System::Registry::HKEY, key_path: &str) -> Result<()> {
    use windows::Win32::System::Registry::*;
    let key_wide: Vec<u16> = key_path.encode_utf16().chain([0]).collect();
    unsafe {
        RegDeleteKeyW(root, windows::core::PCWSTR(key_wide.as_ptr())).ok()?;
    }
    Ok(())
}
