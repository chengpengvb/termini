use summer_web::axum::Json;
use summer::plugin::Service;
use summer_web::axum::extract::Path;
use crate::web::control_base::BaseControl;
use crate::model::resp::resp_model::ResultModel;
use termini_macros::{base_control, control_routes};
use crate::model::ai_chat_2_project::AiChat2Project;
use crate::model::req::req_ai_project::ReqAiChat2ProjectSet;
use crate::service::service_ai_chat_2_project::AiChat2ProjectService;

#[base_control(AiChat2Project, prefix = "/ai_chat_2_project", skip = "select, insert, insert_batch, update, delete")]
#[derive(Clone, Service)]
pub struct AiChat2ProjectController {
    #[inject(component)]
    pub svc: AiChat2ProjectService,
}

impl BaseControl for AiChat2ProjectController {
    type Service = AiChat2ProjectService;

    fn get_service(&self) -> &Self::Service {
        &self.svc
    }
}

#[control_routes(prefix = "/ai_chat_2_project")]
impl AiChat2ProjectController {
    #[summer_web::get("/list")]
    async fn list(&self) -> ResultModel<Vec<AiChat2Project>> {
        ResultModel::ok(self.svc.list())
    }

    #[summer_web::post("/set")]
    async fn set(&self, Json(body): Json<ReqAiChat2ProjectSet>) -> ResultModel<()> {
        ResultModel::from_result(self.svc.set_project(body.chat_id, body.project_id).await)
    }

    #[summer_web::post("/unset/{chat_id}")]
    async fn unset(&self, Path(chat_id): Path<String>) -> ResultModel<()> {
        ResultModel::from_result(self.svc.unset(chat_id).await)
    }
}
