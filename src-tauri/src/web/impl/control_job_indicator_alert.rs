use summer_web::axum::Json;
use summer::plugin::Service;
use summer_web::axum::extract::Path;
use crate::web::control_base::BaseControl;
use crate::model::resp::resp_model::ResultModel;
use termini_macros::{base_control, control_routes};
use crate::model::job_indicator_alert::JobIndicatorAlert;
#[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
use crate::collect::alert::alert_evaluator::AlertEvaluator;
use crate::service::service_job_indicator_alert::JobIndicatorAlertService;

#[base_control(JobIndicatorAlert, prefix = "/job_indicator_alert")]
#[derive(Clone, Service)]
pub struct JobIndicatorAlertController {
    #[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
    #[inject(component)]
    alert_evaluator: AlertEvaluator,

    #[inject(component)]
    pub svc: JobIndicatorAlertService,
}

impl BaseControl for JobIndicatorAlertController {
    type Service = JobIndicatorAlertService;

    fn get_service(&self) -> &Self::Service {
        &self.svc
    }
}

#[control_routes(prefix = "/job_indicator_alert")]
impl JobIndicatorAlertController {
    #[summer_web::get("/select_by_job_device_id/{job_device_id}/job_indicator_id/{job_indicator_id}")]
    async fn select_list(&self, Path((job_device_id, job_indicator_id)): Path<(String, String)>) -> ResultModel<Vec<JobIndicatorAlert>> {
        ResultModel::from_result(self.svc.list_by_device_indicator(&job_device_id, &job_indicator_id).await)
    }

    #[summer_web::get("/select_recent/{limit}")]
    async fn select_recent(&self, Path(limit): Path<usize>) -> ResultModel<Vec<JobIndicatorAlert>> {
        ResultModel::from_result(self.svc.list_recent(limit).await)
    }

    #[summer_web::post("/delete_batch")]
    async fn delete_batch(&self, Json(ids): Json<Vec<String>>) -> ResultModel<u64> {
        let removed = match self.svc.delete_by_ids(ids).await {
            Ok(removed) => removed,
            Err(error) => return ResultModel::from_result(Err(error)),
        };
        #[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
        {
            for alert in &removed {
                self.alert_evaluator.forget(&alert.job_device_id, &alert.job_indicator_id);
            }
        }
        ResultModel::ok(removed.len() as u64)
    }
}
