use crate::util::id_factory::uuid;
use crate::util::time_util::now_millis;
use crate::model::job_device::JobDevice;
use crate::model::dto::shared_ptr::Shared;
use crate::model::job_indicator::JobIndicator;
use crate::model::dto::job::indicator_arg::IndicatorArg;
use crate::model::job_indicator_value::JobIndicatorValue;
use crate::model::job_device_2_indicator::JobDevice2Indicator;

#[derive(Clone)]
pub struct CollectContext {
    pub job_device: Shared<JobDevice>,
    pub job_indicator: Shared<JobIndicator>,
    pub job_device_2_indicator: Shared<JobDevice2Indicator>,
}

impl CollectContext {
    pub fn job_id(&self) -> String {
        self.job_device_2_indicator.read().id.clone()
    }

    pub fn enabled(&self) -> bool {
        self.job_device.read().enabled && self.job_indicator.read().enabled
    }

    pub fn device_id(&self) -> String {
        self.job_device.read().device_id.clone()
    }

    pub fn job_device_id(&self) -> String {
        self.job_device.read().id.clone()
    }

    pub fn job_indicator_id(&self) -> String {
        self.job_indicator.read().id.clone()
    }

    pub fn protocol(&self) -> i16 {
        self.job_indicator.read().protocol
    }

    pub fn interval_secs(&self) -> i64 {
        self.job_device_2_indicator.read().interval_secs
    }

    pub fn parser_type(&self) -> i16 {
        self.job_indicator.read().parser_type
    }

    pub fn parse_expr(&self) -> String {
        self.job_indicator.read().parse_expr.clone()
    }

    pub fn transform_expr(&self) -> String {
        self.job_indicator.read().transform_expr.clone()
    }

    pub fn arg(&self) -> Option<IndicatorArg> {
        self.job_indicator.read().get_arg().ok().flatten()
    }

    pub fn script(&self) -> Option<String> {
        self.arg().map(|arg| arg.value).filter(|value| !value.trim().is_empty())
    }

    pub fn new_value(&self, value: Option<String>, success: bool, error: Option<String>) -> JobIndicatorValue {
        JobIndicatorValue { id: uuid(), value, success, error, job_device_id: self.job_device.read().id.clone(), updated_at: now_millis(), job_indicator_id: self.job_indicator.read().id.clone() }
    }
}
