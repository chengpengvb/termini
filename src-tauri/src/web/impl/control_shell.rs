use summer::plugin::Service;
use async_trait::async_trait;
use crate::base::base_app::BaseApp;
use summer_web::axum::extract::Path;
use termini_macros::{base_app, control_routes};
use crate::model::resp::resp_model::ResultModel;
use crate::service::service_shell::ShellService;
use crate::module::common::context::context_shell::ShellDescr;
use crate::service::service_shell_history::ShellHistoryService;

#[base_app(sort = 420)]
#[derive(Clone, Service)]
pub struct ShellController {
    #[inject(component)]
    shell_service: ShellService,

    #[inject(component)]
    shell_history_service: ShellHistoryService,
}

#[async_trait]
impl BaseApp for ShellController {
    fn phase(&self) -> i16 {
        40
    }

    fn name(&self) -> &'static str {
        "ShellController"
    }
}

#[control_routes(prefix = "/shell")]
impl ShellController {
    #[summer_web::get("/discover")]
    async fn discover(&self) -> ResultModel<Vec<ShellDescr>> {
        ResultModel::from_result(Ok(self.shell_service.discover()))
    }

    #[summer_web::get("/cmd/history/{shell_id}")]
    async fn cmd_history(&self, Path(shell_id): Path<String>) -> ResultModel<Vec<String>> {
        ResultModel::ok(self.shell_history_service.get_cmd_history(&shell_id))
    }
}
