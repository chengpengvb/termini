use termini_macros::base_target;
use serde::{Deserialize, Serialize};
use crate::model::job_indicator::JobIndicator;

#[base_target(table = "job_device_2_indicator", entity_type = 11, notify = 9)]
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Model {
    pub interval_secs: i64,       // 轮询周期（秒）
    pub job_device_id: String,    // 任务设备 ID
    pub job_indicator_id: String, // 指标 ID
    #[sea_orm(ignore)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub job_indicator: Option<JobIndicator>, // 查询填充，不落库
}

pub type JobDevice2Indicator = Model;
