use summer_web::axum::Json;
use summer::plugin::Service;
use summer_web::axum::extract::Path;
use crate::web::control_base::BaseControl;
use crate::model::job_discover::JobDiscover;
use crate::module::ai::flow::FlowJobDiscover;
use crate::service::service_base::BaseService;
use crate::model::resp::resp_model::ResultModel;
use crate::model::job_discover::JobDiscoverStatus;
use termini_macros::{base_control, control_routes};
use crate::model::job_discover_item::JobDiscoverItem;
use crate::service::service_job_discover::JobDiscoverService;

#[base_control(JobDiscover, prefix = "/job_discover")]
#[derive(Clone, Service)]
pub struct JobDiscoverController {
    #[inject(component)]
    pub svc: JobDiscoverService,

    #[inject(component)]
    pub flow_job_discover: FlowJobDiscover,
}

impl BaseControl for JobDiscoverController {
    type Service = JobDiscoverService;

    fn get_service(&self) -> &Self::Service {
        &self.svc
    }
}

#[control_routes(prefix = "/job_discover")]
impl JobDiscoverController {
    #[summer_web::post("/start/{job_device_id}")]
    async fn start(&self, Path(job_device_id): Path<String>) -> ResultModel<JobDiscover> {
        ResultModel::from_result(self.flow_job_discover.start_discover(job_device_id).await)
    }

    #[summer_web::post("/cancel/{chat_id}")]
    async fn cancel(&self, Path(chat_id): Path<String>) -> ResultModel<()> {
        ResultModel::from_result(self.flow_job_discover.cancel(&chat_id))
    }

    #[summer_web::get("/select_item/{discover_id}")]
    async fn select_item(&self, Path(discover_id): Path<String>) -> ResultModel<Vec<JobDiscoverItem>> {
        ResultModel::ok(self.svc.list_items(&discover_id))
    }

    #[summer_web::post("/accept")]
    async fn accept(&self, Json(ids): Json<Vec<String>>) -> ResultModel<()> {
        ResultModel::from_result(self.svc.accept(ids).await)
    }

    #[summer_web::post("/delete_batch")]
    async fn delete_batch(&self, Json(ids): Json<Vec<String>>) -> ResultModel<u64> {
        for id in &ids {
            let Ok(Some(shared)) = self.svc.find_by_id(id.clone()).await else {
                continue;
            };
            let run = shared.read().clone();
            let running = run.status == JobDiscoverStatus::Queued as i16 || run.status == JobDiscoverStatus::Running as i16;
            if running {
                let _ = self.flow_job_discover.cancel(&run.chat_id);
            }
        }
        let removed = match self.svc.delete_by_ids(ids).await {
            Ok(removed) => removed,
            Err(error) => return ResultModel::from_result(Err(error)),
        };
        ResultModel::ok(removed.len() as u64)
    }
}
