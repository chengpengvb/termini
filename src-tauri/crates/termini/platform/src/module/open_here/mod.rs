use anyhow::Result;
use parking_lot::Mutex;
use std::sync::OnceLock;
use crate::util::path_util;
use std::path::{Path, PathBuf};
use summer::tracing::{info, warn};
use crate::model::config::SystemLanguage;
use crate::web::ws::ws_manager::WsManager;
use std::sync::atomic::{AtomicU64, Ordering};
use crate::model::resp::event::resp_event_window::RespEventWindow;

mod open_here_os;

#[derive(Clone)]
pub struct OpenHereItem {
    pub seq: u64,
    pub path: String,
}

struct Queue {
    items: Vec<OpenHereItem>,
}

static QUEUE: OnceLock<Mutex<Queue>> = OnceLock::new();
static SEQ: AtomicU64 = AtomicU64::new(0);

fn queue() -> &'static Mutex<Queue> {
    QUEUE.get_or_init(|| Mutex::new(Queue { items: Vec::new() }))
}

pub struct OpenHere;

impl OpenHere {
    pub fn ingest_open_args(args: impl IntoIterator<Item = impl AsRef<str>>) {
        let mut pending_flag = false;
        for arg in args {
            let raw = arg.as_ref();
            if pending_flag {
                pending_flag = false;
                Self::ingest_explicit(raw);
                continue;
            }
            if raw == "--open-here" {
                pending_flag = true;
                continue;
            }
            if let Some(rest) = raw.strip_prefix("--open-here=") {
                Self::ingest_explicit(rest);
                continue;
            }
            if raw.starts_with("termini://") || raw.starts_with('-') {
                continue;
            }
            if let Some(path) = Self::resolve_path(raw) {
                Self::push(path);
            }
        }
        if pending_flag {
            warn!("[open-here] --open-here missing path");
        }
    }

    pub fn snapshot() -> Vec<OpenHereItem> {
        queue().lock().items.clone()
    }

    pub fn sync_menu(enabled: bool, language: SystemLanguage) -> Result<()> {
        if enabled {
            open_here_os::register(menu_label(language))
        } else {
            open_here_os::unregister()
        }
    }

    fn ingest_explicit(raw: &str) {
        match Self::resolve_path(raw) {
            Some(path) => Self::push(path),
            None => warn!("[open-here] path is not a directory: {raw}"),
        }
    }

    fn push(path: String) {
        let seq = SEQ.fetch_add(1, Ordering::Relaxed) + 1;
        info!("[open-here] queued path={path} seq={seq}");
        queue().lock().items.push(OpenHereItem { seq, path: path.clone() });
        if let Some(manager) = WsManager::current() {
            manager.broadcast_all(&RespEventWindow::OnOpenHere { path, seq }.to_resp());
        }
    }

    fn resolve_path(raw: &str) -> Option<String> {
        let path = if raw.starts_with("file:") {
            let url = url::Url::parse(raw).ok()?;
            url.to_file_path().ok()?
        } else {
            PathBuf::from(raw)
        };
        let canonical = path.canonicalize().ok()?;
        if is_current_exe(&canonical) {
            return None;
        }
        let dir = if canonical.is_dir() {
            canonical
        } else if canonical.is_file() {
            canonical.parent()?.to_path_buf()
        } else {
            return None;
        };
        Some(path_util::strip_unc_prefix(dir.to_str()?.to_string()))
    }
}

fn is_current_exe(path: &Path) -> bool {
    let Ok(exe) = std::env::current_exe() else {
        return false;
    };
    let Ok(exe) = exe.canonicalize() else {
        return false;
    };
    path == exe
}

fn menu_label(language: SystemLanguage) -> &'static str {
    match language {
        SystemLanguage::ZhCn => "在 Termini 中打开",
        SystemLanguage::ZhTw => "在 Termini 中開啟",
        SystemLanguage::EnUs => "Open in Termini",
        SystemLanguage::JaJp => "Termini で開く",
        SystemLanguage::KoKr => "Termini에서 열기",
        SystemLanguage::DeDe => "In Termini öffnen",
        SystemLanguage::FrFr => "Ouvrir dans Termini",
        SystemLanguage::EsEs => "Abrir en Termini",
        SystemLanguage::ItIt => "Apri in Termini",
        SystemLanguage::PtPt => "Abrir no Termini",
        SystemLanguage::NlNl => "Openen in Termini",
        SystemLanguage::HiIn => "Termini में खोलें",
    }
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn all_menu_labels() -> &'static [&'static str] {
    &[
        "Open in Termini",
        "在 Termini 中打开",
        "在 Termini 中開啟",
        "Termini で開く",
        "Termini에서 열기",
        "In Termini öffnen",
        "Ouvrir dans Termini",
        "Abrir en Termini",
        "Apri in Termini",
        "Abrir no Termini",
        "Openen in Termini",
        "Termini में खोलें",
    ]
}