use summer_web::axum::Json;
use summer::plugin::Service;
use summer_web::axum::extract::Path;
use crate::web::control_base::BaseControl;
use crate::collect::collector::AgentCollector;
use crate::model::job_indicator::JobIndicator;
use crate::model::resp::resp_model::ResultModel;
use termini_macros::{base_control, control_routes};
use crate::model::req::req_job_indicator::ReqJobIndicator;
use crate::model::job_device_2_indicator::JobDevice2Indicator;
use crate::service::service_job_indicator::JobIndicatorService;
use crate::model::dto::job::indicator_arg_template::IndicatorArgTemplate;

#[base_control(JobIndicator, prefix = "/job_indicator")]
#[derive(Clone, Service)]
pub struct JobIndicatorController {
    #[inject(component)]
    pub svc: JobIndicatorService,

    #[inject(component)]
    pub agent_collector: AgentCollector,
}

impl BaseControl for JobIndicatorController {
    type Service = JobIndicatorService;

    fn get_service(&self) -> &Self::Service {
        &self.svc
    }
}

#[control_routes(prefix = "/job_indicator")]
impl JobIndicatorController {
    #[summer_web::get("/select_arg_template")]
    async fn select_arg_template(&self) -> ResultModel<Vec<IndicatorArgTemplate>> {
        ResultModel::ok(self.agent_collector.list_arg_template())
    }

    #[summer_web::get("/select_default")]
    async fn select_default(&self) -> ResultModel<Vec<JobIndicator>> {
        ResultModel::from_result(self.svc.select_default().await)
    }

    #[summer_web::post("/save_with_job")]
    async fn save_with_job(&self, Json(req): Json<ReqJobIndicator>) -> ResultModel<JobDevice2Indicator> {
        ResultModel::from_result(self.svc.save_with_job(req).await)
    }

    #[summer_web::delete("/delete_with_job/{job_id}")]
    async fn delete_with_job(&self, Path(job_id): Path<String>) -> ResultModel<()> {
        ResultModel::from_result(self.svc.delete_with_job(job_id).await)
    }
}
