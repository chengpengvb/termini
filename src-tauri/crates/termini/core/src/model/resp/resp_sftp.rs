use std::pin::Pin;
use anyhow::Result;
use std::sync::Arc;
use serde::Serialize;
use std::future::Future;
use crate::common::dto::file_entry::FileEntry;
use crate::model::resp::event::resp_event_ssh_auth::OpenSshStep;
use crate::common::dto::file_stat_properties::FileStatProperties;

pub type RespSftpTransferProgressHandler = Arc<dyn Fn(RespSftpTfProgress) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> + Send + Sync>;
pub type RespSftpDeleteProgressHandler = Arc<dyn Fn(RespSftpDeleteProgress) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> + Send + Sync>;

pub const TRANSFER_COUNT_STATUS_COUNTING: i16 = 0;
pub const TRANSFER_COUNT_STATUS_DONE: i16 = 1;

pub const DELETE_COUNT_STATUS_COUNTING: i16 = 0;
pub const DELETE_COUNT_STATUS_DONE: i16 = 1;

#[derive(Debug, Default, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RespSftpBase {
    pub path: String,
    pub client_transfer_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resp_id: Option<String>,
}

impl RespSftpBase {
    pub fn new(path: impl Into<String>, client_transfer_id: impl Into<String>) -> Self {
        Self { path: path.into(), client_transfer_id: client_transfer_id.into(), resp_id: None }
    }

    pub fn with_resp_id(path: impl Into<String>, client_transfer_id: impl Into<String>, resp_id: impl Into<String>) -> Self {
        Self { path: path.into(), client_transfer_id: client_transfer_id.into(), resp_id: Some(resp_id.into()) }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RespSftpLocalId {
    pub client_transfer_id: String,
}

#[derive(Debug, Clone)]
pub struct RespSftpRemoteDrag {
    #[cfg(target_os = "macos")]
    pub screen_x: f64,
    #[cfg(target_os = "macos")]
    pub screen_y: f64,
    pub session_id: String,
    pub names: Vec<String>,
    pub source_path: String,
    pub client_transfer_id: String,
    pub local_client_transfer_id: String,
}

#[derive(Debug, Default, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RespSftpError {
    pub message: String,
    pub client_transfer_id: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RespSftpOpenStep {
    pub step: OpenSshStep,
    pub client_transfer_id: String,
}

#[derive(Default, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RespSftpEntries {
    pub has_parent: bool,
    pub entries: Vec<FileEntry>,
    pub client_transfer_id: String,
    pub parent_path: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RespSftpEntry {
    pub entry: FileEntry,
    pub client_transfer_id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RespSftpStatProperties {
    pub client_transfer_id: String,
    pub properties: FileStatProperties,
}

#[derive(Debug, Default, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RespSftpTfProgress {
    pub speed: u64,
    pub finish: bool, //传输是否全部完成
    pub file: String,
    pub percent: f32,
    pub files_done: u32,
    pub files_total: u32,
    pub resp_id: String,     //请求ID
    pub transfer_total: u64, //当前文件大小
    pub transfer_done: u64,  //当前文件已传输大小
    pub client_transfer_id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RespSftpTransferCount {
    pub status: i16,
    pub total: u32,
    pub client_transfer_id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RespSftpTransferConflict {
    pub path: String,
    pub request_id: String,
    pub client_transfer_id: String,
}

#[derive(Debug, Default, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RespSftpDeleteProgress {
    pub file: String,
    pub finish: bool, //删除是否全部完成
    pub percent: f32,
    pub resp_id: String,
    pub files_done: u32,
    pub files_total: u32,
    pub client_transfer_id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RespSftpDeleteCount {
    pub status: i16,
    pub total: u32,
    pub client_transfer_id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RespSftpFileOpened {
    pub path: String,
    pub edit_id: Option<String>,
    pub client_transfer_id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RespSftpExported {
    pub paths: Vec<String>,
    pub client_transfer_id: String,
}
