use serde::{Deserialize, Serialize};
use crate::model::resp::resp_device_import::DeviceImportItem;

pub const IMPORT_SOURCE_SSH_CONFIG: i16 = 1;
pub const IMPORT_SOURCE_PUTTY: i16 = 2;
pub const IMPORT_SOURCE_XSHELL: i16 = 3;
pub const IMPORT_SOURCE_TERMIUS: i16 = 4;

#[derive(Debug, Serialize, Deserialize)]
pub struct ReqDeviceImportPreview {
    pub source: i16,

    #[serde(default)]
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReqDeviceImportApply {
    pub items: Vec<DeviceImportItem>,
}
