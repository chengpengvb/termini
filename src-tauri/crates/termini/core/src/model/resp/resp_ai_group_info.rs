use serde::{Deserialize, Serialize};
use crate::model::resp::resp_ai_devices_info::RespAiDevicesInfo;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RespAiGroupInfo {
    pub id: String,
    pub name: String,
    pub devices: Vec<RespAiDevicesInfo>,
}
