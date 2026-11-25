use serde::{Deserialize, Serialize};

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct JobIndicatorValue {
    pub id: String,
    pub success: bool,            // 本次采集是否成功
    pub updated_at: i64,          // 更新时间（Unix 毫秒）
    pub value: Option<String>,    // 指标值，采集失败时为空
    pub error: Option<String>,    // 失败原因，可空
    pub job_device_id: String,    // 设备 ID
    pub job_indicator_id: String, // 指标 ID
}
