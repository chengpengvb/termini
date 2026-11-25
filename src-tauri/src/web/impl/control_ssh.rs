use summer::plugin::Service;
use async_trait::async_trait;
use crate::base::base_app::BaseApp;
use summer_web::axum::extract::Path;
use crate::service::service_ssh::SshService;
use termini_macros::{base_app, control_routes};
use crate::model::resp::resp_model::ResultModel;

#[base_app(sort = 422)]
#[derive(Clone, Service)]
pub struct SshController {
    #[inject(component)]
    ssh_service: SshService,
}

#[async_trait]
impl BaseApp for SshController {
    fn phase(&self) -> i16 {
        40
    }

    fn name(&self) -> &'static str {
        "SshController"
    }
}

#[control_routes(prefix = "/ssh")]
impl SshController {
    #[summer_web::get("/cmd/history/{device_id}")]
    async fn cmd_history(&self, Path(device_id): Path<String>) -> ResultModel<Vec<String>> {
        match self.ssh_service.get_history(&device_id) {
            Some(history) => ResultModel::ok(history),
            None => ResultModel::ok(Vec::new()),
        }
    }
}
