use summer_web::axum::Json;
use summer::plugin::Service;
use async_trait::async_trait;
use crate::base::base_app::BaseApp;
use termini_macros::{base_app, control_routes};
use crate::model::resp::resp_model::ResultModel;
use crate::service::service_device_import::DeviceImportService;
use crate::model::resp::resp_device_import::{DeviceImportItem, RespDeviceImportApply};
use crate::model::req::req_device_import::{ReqDeviceImportApply, ReqDeviceImportPreview};

#[base_app(sort = 421)]
#[derive(Clone, Service)]
pub struct DeviceImportController {
    #[inject(component)]
    device_import_service: DeviceImportService,
}

#[async_trait]
impl BaseApp for DeviceImportController {
    fn phase(&self) -> i16 {
        40
    }

    fn name(&self) -> &'static str {
        "DeviceImportController"
    }
}

#[control_routes(prefix = "/device_import")]
impl DeviceImportController {
    #[summer_web::post("/preview")]
    async fn preview(&self, Json(body): Json<ReqDeviceImportPreview>) -> ResultModel<Vec<DeviceImportItem>> {
        ResultModel::from_result(self.device_import_service.preview(body).await)
    }

    #[summer_web::post("/apply")]
    async fn apply(&self, Json(body): Json<ReqDeviceImportApply>) -> ResultModel<RespDeviceImportApply> {
        ResultModel::from_result(self.device_import_service.apply(body).await)
    }
}
