use summer_web::axum;
use summer::plugin::Service;
use async_trait::async_trait;
use crate::base::base_app::BaseApp;
use crate::model::ai_skill::AiSkill;
use summer_web::axum::extract::Path;
use crate::model::dto::shared_ptr::Shared;
use termini_macros::{base_app, control_routes};
use crate::model::resp::resp_model::ResultModel;
use crate::service::service_agent::AgentService;
use crate::module::ai::shared::{ShellDaemonPush, WakeBus};
use crate::model::req::req_shell_daemon::ReqShellDaemonStop;
use crate::model::req::req_agent_image::{ReqAgentImageRead, ReqAgentImageSave};
use crate::model::resp::resp_agent::{RespAgentImageData, RespAgentShellDaemonRow};
use crate::model::req::req_generate_skill::{ReqCancelGenerateSkill, ReqGenerateSkill};
use crate::module::ai::tools::common::shell::local_shell_registry::LocalShellRegistry;

#[base_app(sort = 910)]
#[derive(Clone, Service)]
pub struct AiAgentController {
    #[inject(component)]
    wake_bus: WakeBus,

    #[inject(component)]
    agent_service: AgentService,

    #[inject(component)]
    shell_daemon_push: ShellDaemonPush,

    #[inject(component)]
    local_shell_registry: LocalShellRegistry,
}

#[async_trait]
impl BaseApp for AiAgentController {
    fn phase(&self) -> i16 {
        40
    }

    fn name(&self) -> &'static str {
        "AiAgentController"
    }
}

#[control_routes(prefix = "/ai_agent")]
impl AiAgentController {
    #[summer_web::post("/generate_skill")]
    async fn generate_skill(&self, axum::Json(body): axum::Json<ReqGenerateSkill>) -> ResultModel<Shared<AiSkill>> {
        ResultModel::from_result(self.agent_service.generate_skill(body).await)
    }

    #[summer_web::post("/cancel_generate_skill")]
    async fn cancel_generate_skill(&self, axum::Json(body): axum::Json<ReqCancelGenerateSkill>) -> ResultModel<()> {
        ResultModel::from_result(self.agent_service.cancel_generate_skill(body))
    }

    #[summer_web::post("/image/read")]
    async fn image_read(&self, axum::Json(body): axum::Json<ReqAgentImageRead>) -> ResultModel<RespAgentImageData> {
        ResultModel::from_result(self.agent_service.read_image(body).await)
    }

    #[summer_web::post("/image/save")]
    async fn image_save(&self, axum::Json(body): axum::Json<ReqAgentImageSave>) -> ResultModel<()> {
        ResultModel::from_result(self.agent_service.save_image(body).await)
    }

    #[summer_web::get("/shell_daemon/{chat_id}")]
    async fn shell_daemon_list(&self, Path(chat_id): Path<String>) -> ResultModel<Vec<RespAgentShellDaemonRow>> {
        ResultModel::ok(self.shell_daemon_push.rows(&chat_id))
    }

    #[summer_web::post("/shell_daemon/stop")]
    async fn shell_daemon_stop(&self, axum::Json(body): axum::Json<ReqShellDaemonStop>) -> ResultModel<()> {
        self.wake_bus.stop(&body.chat_id, &body.process_id);
        if let Some(entry) = self.local_shell_registry.get(&body.chat_id, &body.process_id) {
            entry.process.kill();
        }
        self.shell_daemon_push.push_daemon_list(&body.chat_id);
        ResultModel::ok(())
    }
}
