use termini_macros::base_target;
use serde::{Deserialize, Serialize};

#[base_target(table = "group_2_device", entity_type = 30, notify = 1)]
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Model {
    pub device_id: String, // 设备 ID
    pub group_id: String,  // 组 ID
}

pub type Group2Device = Model;
