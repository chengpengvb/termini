use serde::{Deserialize, Serialize};
use crate::model::job_indicator::JobIndicator;

#[derive(Debug, Serialize, Deserialize)]
pub struct ReqJobIndicator {
    pub interval_secs: i64,
    pub job_device_id: String,
    pub job_indicator: JobIndicator,
}
