use summer_web::axum::Json;
use summer::plugin::Service;
use summer_web::axum::extract::Path;
use crate::model::job_device::JobDevice;
use crate::web::control_base::BaseControl;
use crate::model::resp::resp_model::ResultModel;
use termini_macros::{base_control, control_routes};
use crate::model::resp::resp_metric_item::RespMetricItem;
use crate::service::service_job_device::JobDeviceService;

#[base_control(JobDevice, prefix = "/job_device", skip = "delete")]
#[derive(Clone, Service)]
pub struct JobDeviceController {
    #[inject(component)]
    pub svc: JobDeviceService,
}

impl BaseControl for JobDeviceController {
    type Service = JobDeviceService;

    fn get_service(&self) -> &Self::Service {
        &self.svc
    }
}

#[control_routes(prefix = "/job_device")]
impl JobDeviceController {
    #[summer_web::get("/select_all_detail")]
    async fn select_all_detail(&self) -> ResultModel<Vec<JobDevice>> {
        ResultModel::from_result(self.svc.list_detail().await)
    }

    #[summer_web::get("/take_collect_gap")]
    async fn take_collect_gap(&self) -> ResultModel<i64> {
        ResultModel::ok(self.svc.take_collect_gap())
    }

    #[summer_web::post("/select_metric/{job_device_id}")]
    async fn select_metric(&self, Path(job_device_id): Path<String>, Json(job_indicator_ids): Json<Vec<String>>) -> ResultModel<Vec<RespMetricItem>> {
        ResultModel::from_result(self.svc.list_metric(job_device_id, job_indicator_ids).await)
    }

    #[summer_web::post("/set_enabled/{id}")]
    async fn set_enabled(&self, Path(id): Path<String>, Json(enabled): Json<bool>) -> ResultModel<bool> {
        ResultModel::from_result(self.svc.set_enabled(id, enabled).await)
    }

    #[summer_web::post("/save_batch")]
    async fn save_batch(&self, Json(device_ids): Json<Vec<String>>) -> ResultModel<u64> {
        ResultModel::from_result(self.svc.save_batch(device_ids).await)
    }

    #[summer_web::delete("/delete/{id}")]
    async fn delete(&self, Path(id): Path<String>) -> ResultModel<u64> {
        ResultModel::from_result(self.svc.delete_detail(id).await)
    }
}
