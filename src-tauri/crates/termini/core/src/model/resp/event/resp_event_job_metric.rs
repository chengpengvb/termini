use crate::model::resp::resp_metric_item::RespMetricItem;
use crate::model::resp::resp_ws_context::RespSocketContext;

pub enum RespEventJobMetric {
    OnChanged(RespMetricItem),
}

impl RespEventJobMetric {
    pub const ON_CHANGED: i16 = 74;

    pub fn to_type(&self) -> i16 {
        match self {
            RespEventJobMetric::OnChanged(_) => Self::ON_CHANGED,
        }
    }

    pub fn to_resp(&self) -> RespSocketContext {
        match self {
            RespEventJobMetric::OnChanged(metric) => RespSocketContext { r#type: self.to_type(), session_id: metric.job_device_id.clone(), payload: serde_json::to_value(metric).ok() },
        }
    }
}
