use std::path::PathBuf;
use crate::module::ai::tools::ai_tool_error::AiToolError;
use crate::model::resp::event::resp_event_sftp::RespEventSftp;

pub const SFTP_TRANSFER_DIRECTION_UPLOAD: i16 = 0;
pub const SFTP_TRANSFER_DIRECTION_DOWNLOAD: i16 = 1;

pub fn resolve_direction(direction: i16) -> Result<i16, AiToolError> {
    if direction == SFTP_TRANSFER_DIRECTION_UPLOAD || direction == SFTP_TRANSFER_DIRECTION_DOWNLOAD {
        return Ok(direction);
    }
    Err(AiToolError::Execution(format!("invalid direction: {} (0=upload, 1=download)", direction)))
}

pub fn resolve_local_upload(local_path: &str) -> Result<(String, String), AiToolError> {
    let trimmed = local_path.trim();
    if trimmed.is_empty() {
        return Err(AiToolError::Execution("local_path is empty".to_string()));
    }
    let path = PathBuf::from(trimmed);
    if !path.is_absolute() {
        return Err(AiToolError::Execution(format!("local_path must be absolute: {}", trimmed)));
    }
    if !path.exists() {
        return Err(AiToolError::Execution(format!("local path not found: {}", trimmed)));
    }
    let name = path.file_name().and_then(|value| value.to_str()).ok_or_else(|| AiToolError::Execution(format!("invalid local_path file name: {}", trimmed)))?.to_string();
    let parent = path.parent().ok_or_else(|| AiToolError::Execution(format!("local_path has no parent: {}", trimmed)))?.to_string_lossy().to_string();
    Ok((parent, name))
}

pub fn resolve_remote_source(remote_path: &str) -> Result<(String, String), AiToolError> {
    let trimmed = remote_path.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return Err(AiToolError::Execution("remote_path is empty".to_string()));
    }
    if !trimmed.starts_with('/') {
        return Err(AiToolError::Execution(format!("remote_path must be absolute: {}", remote_path)));
    }
    let name = match trimmed.rsplit('/').next() {
        Some(value) if !value.is_empty() => value.to_string(),
        _ => return Err(AiToolError::Execution(format!("invalid remote_path file name: {}", remote_path))),
    };
    let Some(index) = trimmed.rfind('/') else {
        return Err(AiToolError::Execution(format!("remote_path has no parent: {}", remote_path)));
    };
    if index == 0 {
        return Ok(("/".to_string(), name));
    }
    Ok((trimmed[..index].to_string(), name))
}

pub fn resolve_local_download_target(local_path: &str) -> Result<String, AiToolError> {
    let trimmed = local_path.trim();
    if trimmed.is_empty() {
        return Err(AiToolError::Execution("local_path is empty".to_string()));
    }
    let path = PathBuf::from(trimmed);
    if !path.is_absolute() {
        return Err(AiToolError::Execution(format!("local_path must be absolute: {}", trimmed)));
    }
    if !path.exists() {
        return Err(AiToolError::Execution(format!("local target directory not found: {}", trimmed)));
    }
    if !path.is_dir() {
        return Err(AiToolError::Execution(format!("local_path must be a directory for download: {}", trimmed)));
    }
    Ok(trimmed.to_string())
}

pub fn format_progress_event(event: &RespEventSftp) -> Option<String> {
    match event {
        RespEventSftp::OnTransferCount(count) if count.status == crate::model::resp::resp_sftp::TRANSFER_COUNT_STATUS_COUNTING => Some("counting files…\n".to_string()),
        RespEventSftp::OnTransferCount(count) => Some(format!("files_total={}\n", count.total)),
        RespEventSftp::OnTransferProgress(progress) => {
            if progress.finish {
                return Some(format!("done {:.1}% files={}/{}\n", progress.percent, progress.files_done, progress.files_total));
            }
            Some(format!("[{:.1}%] {} files={}/{} speed={} B/s\n", progress.percent, progress.file, progress.files_done, progress.files_total, progress.speed))
        }
        RespEventSftp::OnTransfer(_) => Some("transfer finished\n".to_string()),
        _ => None,
    }
}
