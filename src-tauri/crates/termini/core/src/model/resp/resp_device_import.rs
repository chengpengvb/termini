use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceImportItem {
    pub name: String,
    pub host: String,
    pub port: u16,

    #[serde(default)]
    pub username: Option<String>,

    #[serde(default)]
    pub jump_name: Option<String>,

    #[serde(default)]
    pub exists: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RespDeviceImportApply {
    pub imported: i32,
    pub skipped: i32,
    pub jump_linked: i32,
}
