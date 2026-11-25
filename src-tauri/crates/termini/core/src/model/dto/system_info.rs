use serde::{Deserialize, Serialize};
use crate::model::resp::resp_platform_info::RespPlatformInfo;

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemInfo {
    pub cpu_info: String,
    pub disk_info: String,
    pub memory_info: String,
    pub terminal: Option<String>,
    pub platform_info: RespPlatformInfo,
}
