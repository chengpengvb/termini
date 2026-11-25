use std::path::{Path, PathBuf};
use summer::tracing::{info, warn};
use termini_core::model::resp::resp_run_marker::RespRunMarker;

const MARKER_FILE: &str = "running.marker";
const LAST_CRASH_FILE: &str = "last-crash.json";

fn marker_path(data_dir: &Path) -> PathBuf {
    data_dir.join(MARKER_FILE)
}

fn last_crash_path(data_dir: &Path) -> PathBuf {
    data_dir.join(LAST_CRASH_FILE)
}

fn now_millis() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

pub fn arm(data_dir: &Path, version: &str) {
    let path = marker_path(data_dir);
    if let Ok(raw) = std::fs::read_to_string(&path) {
        match serde_json::from_str::<RespRunMarker>(&raw) {
            Ok(previous) => {
                warn!("[crash-marker] 上次运行未正常退出 pid={} version={} started_at={}", previous.pid, previous.version, previous.started_at);
                if let Err(error) = std::fs::write(last_crash_path(data_dir), raw) {
                    warn!("[crash-marker] 写 last-crash.json 失败: {}", error);
                }
            }
            Err(error) => warn!("[crash-marker] 残留标记无法解析: {}", error),
        }
    }
    let marker = RespRunMarker { pid: std::process::id(), version: version.to_string(), started_at: now_millis() };
    match serde_json::to_string(&marker) {
        Ok(json) => {
            if let Err(error) = std::fs::create_dir_all(data_dir).and_then(|_| std::fs::write(&path, json)) {
                warn!("[crash-marker] 写运行标记失败: {}", error);
            }
        }
        Err(error) => warn!("[crash-marker] 序列化运行标记失败: {}", error),
    }
}

pub fn disarm(data_dir: &Path) {
    if let Err(error) = std::fs::remove_file(marker_path(data_dir)) {
        if error.kind() != std::io::ErrorKind::NotFound {
            warn!("[crash-marker] 删除运行标记失败: {}", error);
        }
        return;
    }
    info!("[crash-marker] 正常退出, 运行标记已清除");
}

#[tauri::command]
pub fn take_last_crash() -> Option<RespRunMarker> {
    let data_dir = PathBuf::from(std::env::var("TERMINI_DATA_DIR").ok()?);
    let path = last_crash_path(&data_dir);
    let raw = std::fs::read_to_string(&path).ok()?;
    let _ = std::fs::remove_file(&path);
    serde_json::from_str::<RespRunMarker>(&raw).ok()
}
