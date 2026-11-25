use serde::{Deserialize, Serialize};
use crate::model::job_indicator_alert::JobIndicatorAlert;
use crate::model::job_indicator_value::JobIndicatorValue;

pub enum RespMetricStatus {
    Pending = 0, //未采集
    Ok = 1,      //正常
    Error = 2,   //采集失败
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RespMetricItem {
    pub status: i16,
    pub job_device_id: String,
    pub updated_at: Option<i64>,
    pub job_indicator_id: String,
    pub values: Vec<JobIndicatorValue>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alert: Option<JobIndicatorAlert>,
}
