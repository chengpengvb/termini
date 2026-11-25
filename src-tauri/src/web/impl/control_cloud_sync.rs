use summer::plugin::Service;
use async_trait::async_trait;
use crate::base::base_app::BaseApp;
use termini_macros::{base_app, control_routes};
use crate::model::resp::resp_model::ResultModel;
use crate::service::service_cloud_sync::CloudSyncService;
use crate::model::req::req_cloud_sync::{ReqCloudSyncApply, ReqCloudSyncPrepare};
use crate::model::resp::resp_cloud_sync::{RespCloudSyncPrepare, RespCloudSyncStatus};

#[base_app(sort = 420)]
#[derive(Clone, Service)]
pub struct CloudSyncController {
    #[inject(component)]
    cloud_sync_service: CloudSyncService,
}

#[async_trait]
impl BaseApp for CloudSyncController {
    fn phase(&self) -> i16 {
        40
    }

    fn name(&self) -> &'static str {
        "CloudSyncController"
    }
}

#[control_routes(prefix = "/cloud_sync")]
impl CloudSyncController {
    #[summer_web::get("/status")]
    async fn status(&self) -> ResultModel<RespCloudSyncStatus> {
        ResultModel::from_result(self.cloud_sync_service.status().await)
    }

    #[summer_web::post("/prepare")]
    async fn prepare(&self, summer_web::axum::Json(body): summer_web::axum::Json<ReqCloudSyncPrepare>) -> ResultModel<RespCloudSyncPrepare> {
        ResultModel::from_result(self.cloud_sync_service.prepare(body.op, body.password).await)
    }

    #[summer_web::post("/apply")]
    async fn apply(&self, summer_web::axum::Json(body): summer_web::axum::Json<ReqCloudSyncApply>) -> ResultModel<()> {
        ResultModel::from_result(self.cloud_sync_service.apply(body.op, body.password, body.choices).await)
    }
}
