use std::sync::Arc;
use std::future::Future;
use tokio::sync::Semaphore;
use futures::future::join_all;
use std::collections::HashSet;
use tokio_util::sync::CancellationToken;
use crate::service::service_base::BaseService;
use crate::service::service_device::DeviceService;
use crate::module::ai::tools::ai_tool_error::AiToolError;
use crate::model::resp::resp_ai_ssh_batch::RespAiSshBatchItem;

pub const MAX_DEVICE_IDS: usize = 50;

pub fn normalize_device_ids(device_ids: Vec<String>) -> Result<Vec<String>, AiToolError> {
    let mut seen = HashSet::new();
    let mut normalized = Vec::new();
    for device_id in device_ids {
        let trimmed = device_id.trim();
        if trimmed.is_empty() || !seen.insert(trimmed.to_string()) {
            continue;
        }
        normalized.push(trimmed.to_string());
    }
    if normalized.is_empty() {
        return Err(AiToolError::Execution("device_ids is empty".to_string()));
    }
    if normalized.len() > MAX_DEVICE_IDS {
        return Err(AiToolError::Execution(format!("device_ids allows at most {} devices, got {}", MAX_DEVICE_IDS, normalized.len())));
    }
    Ok(normalized)
}

pub async fn device_label(device_service: &DeviceService, device_id: &str) -> Result<(String, String), String> {
    let device = device_service.find_by_id(device_id.to_string()).await.map_err(|error| error.to_string())?.ok_or_else(|| format!("device not found: {}", device_id))?;
    let guard = device.read();
    Ok((guard.name.clone(), guard.host.clone()))
}

pub fn cancelled_item(device_id: String, name: String, host: String) -> RespAiSshBatchItem {
    RespAiSshBatchItem { device_id, name, host, ok: false, exit_code: None, output: String::new(), error: Some("cancelled".to_string()) }
}

pub fn error_item(device_id: String, name: String, host: String, error: String) -> RespAiSshBatchItem {
    RespAiSshBatchItem { device_id, name, host, ok: false, exit_code: None, output: String::new(), error: Some(error) }
}

pub fn ok_item(device_id: String, name: String, host: String, exit_code: Option<i32>, output: String) -> RespAiSshBatchItem {
    RespAiSshBatchItem { device_id, name, host, ok: true, exit_code, output, error: None }
}

pub async fn run_device_batch<F, Fut>(device_ids: Vec<String>, concurrency: usize, tool_cancel: &CancellationToken, session_cancel: &CancellationToken, task: F) -> Vec<RespAiSshBatchItem>
where
    F: Fn(String) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = RespAiSshBatchItem> + Send,
{
    let semaphore = Arc::new(Semaphore::new(concurrency.max(1)));
    let task = Arc::new(task);
    let futures = device_ids.into_iter().map(|device_id| {
        let semaphore = semaphore.clone();
        let task = Arc::clone(&task);
        let tool_cancel = tool_cancel.clone();
        let session_cancel = session_cancel.clone();
        async move {
            if tool_cancel.is_cancelled() || session_cancel.is_cancelled() {
                return cancelled_item(device_id, String::new(), String::new());
            }
            let Ok(permit) = semaphore.acquire().await else {
                return error_item(device_id, String::new(), String::new(), "semaphore closed".to_string());
            };
            if tool_cancel.is_cancelled() || session_cancel.is_cancelled() {
                drop(permit);
                return cancelled_item(device_id, String::new(), String::new());
            }
            let item = task(device_id).await;
            drop(permit);
            item
        }
    });
    join_all(futures).await
}
