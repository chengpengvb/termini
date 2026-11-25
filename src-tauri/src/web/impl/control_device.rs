use summer::plugin::Service;
use crate::model::device::Device;
use crate::web::control_base::BaseControl;
use crate::model::resp::resp_tree::RespTree;
use crate::model::req::req_device::ReqDevice;
use crate::model::resp::resp_device::RespDevice;
use crate::model::resp::resp_model::ResultModel;
use crate::service::service_device::DeviceService;
use termini_macros::{base_control, control_routes};
#[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
use crate::service::service_job_device::JobDeviceService;
use crate::module::ssh_forward::forward_manager::ForwardManager;

#[base_control(Device, prefix = "/device")]
#[derive(Clone, Service)]
pub struct DeviceController {
    #[inject(component)]
    pub svc: DeviceService,

    #[inject(component)]
    pub forward_manager: ForwardManager,

    #[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
    #[inject(component)]
    pub job_device_service: JobDeviceService,
}

impl BaseControl for DeviceController {
    type Service = DeviceService;

    fn get_service(&self) -> &Self::Service {
        &self.svc
    }
}

#[control_routes(prefix = "/device")]
impl DeviceController {
    #[summer_web::get("/local_device")]
    async fn local_device(&self) -> ResultModel<RespDevice> {
        ResultModel::ok(self.svc.local_device().read().clone())
    }

    #[summer_web::get("/{id}/with_config")]
    async fn select_with_config(&self, summer_web::axum::extract::Path(id): summer_web::axum::extract::Path<String>) -> ResultModel<Option<RespDevice>> {
        ResultModel::from_result(self.svc.select_with_config(id).await)
    }

    #[summer_web::get("/select_all_with_config")]
    async fn select_all_with_config(&self) -> ResultModel<Vec<RespDevice>> {
        ResultModel::from_result(self.svc.select_all_with_config().await)
    }

    #[summer_web::get("/select_device_tree")]
    async fn select_device_tree(&self) -> ResultModel<Vec<RespTree>> {
        ResultModel::from_result(self.svc.select_device_tree().await)
    }

    #[summer_web::get("/{id}/delete_with_config")]
    async fn delete_with_config(&self, summer_web::axum::extract::Path(id): summer_web::axum::extract::Path<String>) -> ResultModel<()> {
        self.forward_manager.stop_by_device_id(id.clone()).await;
        if let Err(error) = self.svc.delete_with_config(id.clone()).await {
            return ResultModel::from_result(Err(error));
        }
        #[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
        {
            return ResultModel::from_result(self.job_device_service.delete_by_device_id(id).await.map(|_| ()));
        }
        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        ResultModel::ok(())
    }

    #[summer_web::post("/save_with_config")]
    async fn save_with_config(&self, summer_web::axum::Json(body): summer_web::axum::Json<ReqDevice>) -> ResultModel<RespDevice> {
        ResultModel::from_result(self.svc.save_with_config(body).await)
    }

    #[summer_web::post("/update_with_config")]
    async fn update_with_config(&self, summer_web::axum::Json(body): summer_web::axum::Json<ReqDevice>) -> ResultModel<RespDevice> {
        ResultModel::from_result(self.svc.update_with_config(body).await)
    }
}
