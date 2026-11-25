use termini_macros::base_target;
use crate::model::device::Device;
use serde::{Deserialize, Serialize};
use crate::model::job_device_2_indicator::JobDevice2Indicator;

#[base_target(table = "job_device", entity_type = 9)]
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Model {
    pub device_id: String, // 设备 ID
    pub enabled: bool,     // 任务设备开启 / 关闭
    pub updated_at: i64,   // 更新时间（Unix 毫秒）
    #[sea_orm(ignore)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device: Option<Device>, // 查询填充，不落库
    #[sea_orm(ignore)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub indicators: Option<Vec<JobDevice2Indicator>>, // 查询填充，不落库
}

pub type JobDevice = Model;
