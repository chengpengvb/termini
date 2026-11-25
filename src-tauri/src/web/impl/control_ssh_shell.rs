use std::time::Duration;
use summer::plugin::Service;
use async_trait::async_trait;
use crate::base::base_app::BaseApp;
use termini_macros::{base_app, control_routes};
use crate::service::service_ssh_shell::SshShellService;
use crate::model::resp::resp_ssh_shell::RespSshShellExec;
use crate::model::resp::resp_model::{ResultCode, ResultModel};
use crate::model::req::req_ssh_shell::{ReqSshShellClose, ReqSshShellExec, ReqSshShellOpen};

#[base_app(sort = 420)]
#[derive(Clone, Service)]
pub struct SshShellController {
    #[inject(component)]
    ssh_shell_service: SshShellService,
}

#[async_trait]
impl BaseApp for SshShellController {
    fn phase(&self) -> i16 {
        40
    }

    fn name(&self) -> &'static str {
        "SshShellController"
    }
}

#[control_routes(prefix = "/ssh/shell")]
impl SshShellController {
    #[summer_web::post("/open")]
    async fn open(&self, summer_web::axum::Json(body): summer_web::axum::Json<ReqSshShellOpen>) -> ResultModel<()> {
        ResultModel::from_result(self.ssh_shell_service.open(body.session_id, body.device_id).await)
    }

    #[summer_web::post("/close")]
    async fn close(&self, summer_web::axum::Json(body): summer_web::axum::Json<ReqSshShellClose>) -> ResultModel<()> {
        ResultModel::from_result(self.ssh_shell_service.close(&body.session_id).await)
    }

    #[summer_web::post("/exec")]
    async fn exec(&self, summer_web::axum::Json(body): summer_web::axum::Json<ReqSshShellExec>) -> ResultModel<RespSshShellExec> {
        if body.exec_timeout_seconds == 0 {
            return ResultModel::fail_code(ResultCode::ExecTimeoutInvalid);
        }
        let exec_timeout = Duration::from_secs(body.exec_timeout_seconds);
        let result = self.ssh_shell_service.exec(&body.session_id, body.command, exec_timeout).await.map(|(exit_code, output)| RespSshShellExec { exit_code, output });
        ResultModel::from_result(result)
    }
}
