use tauri::Emitter;
use tauri::AppHandle;
use parking_lot::Mutex;
use std::sync::OnceLock;
use termini_core::util::crypto_util;
use termini_core::model::resp::resp_fatal_error::{RespFatalError, FATAL_KIND_CRYPTO, FATAL_KIND_PANIC};

pub const FATAL_EVENT: &str = "termini://fatal";

fn fatal_slot() -> &'static Mutex<Option<RespFatalError>> {
    static SLOT: OnceLock<Mutex<Option<RespFatalError>>> = OnceLock::new();
    SLOT.get_or_init(|| Mutex::new(None))
}

fn panic_slot() -> &'static Mutex<Option<String>> {
    static SLOT: OnceLock<Mutex<Option<String>>> = OnceLock::new();
    SLOT.get_or_init(|| Mutex::new(None))
}

pub fn install_panic_hook() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        *panic_slot().lock() = Some(info.to_string());
        previous(info);
    }));
}

pub fn raise(handle: &AppHandle) {
    let kind = if crypto_util::key_broken() { FATAL_KIND_CRYPTO } else { FATAL_KIND_PANIC };
    let message = match panic_slot().lock().take() {
        Some(text) => text,
        None => "startup thread panicked, original panic message not captured".to_string(),
    };
    let fatal = RespFatalError { kind, message };
    *fatal_slot().lock() = Some(fatal.clone());
    let _ = handle.emit(FATAL_EVENT, fatal);
}

#[tauri::command]
pub fn fatal_error() -> Option<RespFatalError> {
    fatal_slot().lock().clone()
}

#[tauri::command]
pub fn recover_master_key(handle: AppHandle, phrase: String) -> Result<(), String> {
    crypto_util::import_phrase(&phrase).map_err(|error| format!("{error:#}"))?;
    handle.restart()
}

#[tauri::command]
pub fn restart_app(handle: AppHandle) {
    handle.restart()
}
