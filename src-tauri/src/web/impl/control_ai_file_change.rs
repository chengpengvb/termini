use summer_web::axum::Json;
use summer::plugin::Service;
use summer_web::axum::extract::Path;
use crate::web::control_base::BaseControl;
use crate::model::ai_file_change::AiFileChange;
use crate::model::resp::resp_model::ResultModel;
use termini_macros::{base_control, control_routes};
use crate::service::service_ai_file_change::AiFileChangeService;
use crate::model::req::req_ai_file_change::ReqAiFileChangeAction;
use crate::model::resp::resp_ai_file_change::RespAiFileChangeView;

#[base_control(AiFileChange, prefix = "/ai_file_change", skip = "insert, insert_batch, update, delete")]
#[derive(Clone, Service)]
pub struct AiFileChangeController {
    #[inject(component)]
    pub svc: AiFileChangeService,
}

impl BaseControl for AiFileChangeController {
    type Service = AiFileChangeService;

    fn get_service(&self) -> &Self::Service {
        &self.svc
    }
}

#[control_routes(prefix = "/ai_file_change")]
impl AiFileChangeController {
    #[summer_web::get("/select_view_by_chat_id/{chat_id}")]
    async fn select_view_by_chat_id(&self, Path(chat_id): Path<String>) -> ResultModel<Vec<RespAiFileChangeView>> {
        ResultModel::from_result(self.svc.list_view(chat_id).await)
    }

    #[summer_web::post("/accept")]
    async fn accept(&self, Json(body): Json<ReqAiFileChangeAction>) -> ResultModel<()> {
        ResultModel::from_result(self.svc.accept(body.chat_id, body.device_id, body.path).await)
    }

    #[summer_web::post("/revert")]
    async fn revert(&self, Json(body): Json<ReqAiFileChangeAction>) -> ResultModel<()> {
        ResultModel::from_result(self.svc.revert(body.chat_id, body.device_id, body.path).await)
    }
}
