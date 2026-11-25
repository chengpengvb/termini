use serde::{Deserialize, Serialize};

pub const SFTP_FILE_COPY_SOURCE_KIND_OS: i8 = 0;
pub const SFTP_FILE_COPY_SOURCE_KIND_APP: i8 = 1;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RespSftpFileCopy {
    #[serde(default)]
    pub source_kind: i8,
    pub names: Vec<String>,
    pub source_path: String,
    #[serde(default)]
    pub side_session_id: Option<String>,
    pub source_client_transfer_id: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RespSftpFilePaste {
    #[serde(default)]
    pub source_kind: i8,
    pub target_path: String,
    #[serde(default)]
    pub side_session_id: Option<String>,
    pub target_client_transfer_id: String,
}
