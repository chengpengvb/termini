use summer::plugin::Service;
use async_trait::async_trait;
use crate::base::base_app::BaseApp;
use summer_web::axum::extract::Path;
use termini_macros::{base_app, control_routes};
use crate::model::resp::resp_model::ResultModel;
use crate::model::job_indicator_value::JobIndicatorValue;
use crate::service::service_job_indicator_value::JobIndicatorValueService;

#[base_app(sort = 455)]
#[derive(Clone, Service)]
pub struct JobIndicatorValueController {
    #[inject(component)]
    svc: JobIndicatorValueService,
}

#[async_trait]
impl BaseApp for JobIndicatorValueController {
    fn phase(&self) -> i16 {
        40
    }

    fn name(&self) -> &'static str {
        "JobIndicatorValueController"
    }
}

#[control_routes(prefix = "/job_indicator_value")]
impl JobIndicatorValueController {
    #[summer_web::get("/select/{id}")]
    async fn select(&self, Path(id): Path<String>) -> ResultModel<Option<JobIndicatorValue>> {
        ResultModel::ok(self.svc.find_by_id(&id))
    }

    #[summer_web::get("/select_by_job_device_id/{job_device_id}/job_indicator_id/{job_indicator_id}/range/{duration_ms}")]
    async fn select_by_device_indicator_range(&self, Path((job_device_id, job_indicator_id, duration_ms)): Path<(String, String, i64)>) -> ResultModel<Vec<JobIndicatorValue>> {
        ResultModel::ok(self.svc.list_by_range(&job_device_id, &job_indicator_id, duration_ms))
    }
}
