use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RespAiDevicesInfo {
    pub id: String,
    pub name: String,
}
