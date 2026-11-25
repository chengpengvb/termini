use summer_web::axum::Json;
use summer::plugin::Service;
use summer_web::axum::extract::Path;
use crate::model::dto::shared_ptr::Shared;
use crate::web::control_base::BaseControl;
use crate::model::port_forward::PortForward;
use crate::service::service_base::BaseService;
use crate::model::resp::resp_model::ResultModel;
use termini_macros::{base_control, control_routes};
use crate::service::service_port_forward::PortForwardService;
use crate::module::ssh_forward::forward_manager::ForwardManager;

#[base_control(PortForward, prefix = "/port_forward", skip = "select_all, insert, update, delete")]
#[derive(Clone, Service)]
pub struct PortForwardController {
    #[inject(component)]
    pub svc: PortForwardService,

    #[inject(component)]
    pub forward_manager: ForwardManager,
}

impl BaseControl for PortForwardController {
    type Service = PortForwardService;

    fn get_service(&self) -> &Self::Service {
        &self.svc
    }
}

#[control_routes(prefix = "/port_forward")]
impl PortForwardController {
    #[summer_web::get("/select_all")]
    async fn select_all(&self) -> ResultModel<Vec<PortForward>> {
        ResultModel::from_result(self.svc.select_all_with_status().await)
    }

    #[summer_web::get("/select_by_device/{device_id}")]
    async fn select_by_device(&self, Path(device_id): Path<String>) -> ResultModel<Vec<PortForward>> {
        ResultModel::ok(self.svc.select_by_device(device_id))
    }

    #[summer_web::post("/insert")]
    async fn insert(&self, Json(body): Json<PortForward>) -> ResultModel<Shared<PortForward>> {
        match self.svc.insert(body).await {
            Ok(saved) => {
                self.spawn_sync(saved.read().clone());
                ResultModel::ok(saved)
            }
            Err(error) => ResultModel::from_result(Err(error)),
        }
    }

    #[summer_web::post("/update")]
    async fn update(&self, Json(body): Json<PortForward>) -> ResultModel<Shared<PortForward>> {
        match self.svc.update(body).await {
            Ok(saved) => {
                self.spawn_sync(saved.read().clone());
                ResultModel::ok(saved)
            }
            Err(error) => ResultModel::from_result(Err(error)),
        }
    }

    #[summer_web::delete("/delete/{id}")]
    async fn delete(&self, Path(id): Path<String>) -> ResultModel<()> {
        self.forward_manager.stop_rule(id.clone()).await;
        ResultModel::from_result(self.svc.delete_by_id(id).await.map(|_| ()))
    }
}

impl PortForwardController {
    fn spawn_sync(&self, rule: PortForward) {
        let manager = self.forward_manager.clone();
        tokio::spawn(async move {
            manager.sync_rule(rule).await;
        });
    }
}
