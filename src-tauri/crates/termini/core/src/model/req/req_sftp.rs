use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct ReqSftpStartDrag {
    pub paths: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReqSftpStartRemoteDrag {
    #[cfg(target_os = "macos")]
    pub screen_x: f64,
    #[cfg(target_os = "macos")]
    pub screen_y: f64,
    pub names: Vec<String>,
    pub session_id: String,
    pub source_path: String,
    pub client_transfer_id: String,
}
