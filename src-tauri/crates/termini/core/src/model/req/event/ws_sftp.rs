use serde::{Deserialize, Serialize};
use crate::common::dto::file_entry::FileKind;
use crate::model::req::event::ws_base::WsBase;

pub type OpenEvent = WsBase<SftpOpen>;
pub type StatEvent = WsBase<SftpStatData>;
pub type ChmodEvent = WsBase<SftpChmodData>;
pub type ListDirEvent = WsBase<SftpPathData>;
pub type FileOpenEvent = WsBase<SftpFileOpen>;
pub type HomeDirEvent = WsBase<SftpClientRef>;
pub type RenameEvent = WsBase<SftpRenameData>;
pub type CreateEvent = WsBase<SftpCreateData>;
pub type DeleteEvent = WsBase<SftpDeleteData>;
pub type TransferEvent = WsBase<SftpTransferData>;
pub type ExportToTempEvent = WsBase<SftpExportData>;
pub type DeleteCancelEvent = WsBase<SftpDeleteCancel>;
pub type TransferCancelEvent = WsBase<SftpTransferCancel>;
pub type TransferAnswerEvent = WsBase<SftpTransferAnswerData>;

fn default_open_protocol() -> i16 {
    crate::model::config_ftp::PROTOCOL_SFTP
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SftpOpen {
    pub device_id: String,
    #[serde(default = "default_open_protocol")]
    pub protocol: i16,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SftpClientRef {
    #[serde(default)]
    pub client_transfer_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SftpPathData {
    pub client_transfer_id: String,
    pub path: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SftpStatData {
    pub client_transfer_id: String,
    pub path: String,
    pub kind: FileKind,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SftpRenameData {
    pub path: String,
    pub to_name: String,
    pub from_name: String,
    pub client_transfer_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SftpChmodData {
    pub mode: u32,
    pub path: String,
    pub names: Vec<String>,
    pub client_transfer_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SftpCreateData {
    pub path: String,
    pub name: String,
    pub kind: FileKind,
    pub client_transfer_id: String,
    pub link_target: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SftpDeleteData {
    pub path: String,
    pub names: Vec<String>,
    pub client_transfer_id: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SftpTransferData {
    pub names: Vec<String>,
    pub source_path: String,
    pub target_path: String,
    pub side_session_id: String,
    pub source_client_transfer_id: String,
    pub target_client_transfer_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SftpTransferAnswerData {
    pub code: i16,
    pub request_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SftpFileOpen {
    pub path: String,
    pub open_with: bool,
    pub client_transfer_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SftpExportData {
    pub names: Vec<String>,
    pub source_path: String,
    pub source_client_transfer_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SftpTransferCancel {
    pub resp_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SftpDeleteCancel {
    pub resp_id: String,
}
