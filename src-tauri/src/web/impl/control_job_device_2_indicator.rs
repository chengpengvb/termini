use summer::plugin::Service;
use summer_web::axum::extract::Path;
use crate::collect::job::JobTargetService;
use crate::web::control_base::BaseControl;
use crate::model::resp::resp_model::ResultModel;
use termini_macros::{base_control, control_routes};
use crate::model::job_indicator_value::JobIndicatorValue;
use crate::model::job_device_2_indicator::JobDevice2Indicator;
use crate::service::service_job_device_2_indicator::JobDevice2IndicatorService;

#[base_control(JobDevice2Indicator, prefix = "/job_device_2_indicator")]
#[derive(Clone, Service)]
pub struct JobDevice2IndicatorController {
    #[inject(component)]
    pub svc: JobDevice2IndicatorService,

    #[inject(component)]
    pub job_target_service: JobTargetService,
}

impl BaseControl for JobDevice2IndicatorController {
    type Service = JobDevice2IndicatorService;

    fn get_service(&self) -> &Self::Service {
        &self.svc
    }
}

#[control_routes(prefix = "/job_device_2_indicator")]
impl JobDevice2IndicatorController {
    #[summer_web::post("/run_once/{job_id}")]
    async fn run_once(&self, Path(job_id): Path<String>) -> ResultModel<Vec<JobIndicatorValue>> {
        ResultModel::from_result(self.job_target_service.run_once(&job_id).await)
    }
}
