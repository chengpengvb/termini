use summer::plugin::Service;
use summer_web::axum::extract::Path;
use crate::web::control_base::BaseControl;
use crate::model::ai_chat::{AiChat, AiChatType};
use crate::service::service_ai_chat::AiChatService;
use termini_macros::{base_control, control_routes};
use crate::model::req::req_ai_chat::ReqAiChatRevert;
use crate::model::req::req_ai_project::ReqAiChatRename;
use crate::model::resp::resp_model::{ResultCode, ResultModel};

#[base_control(AiChat, prefix = "/ai_chat")]
#[derive(Clone, Service)]
pub struct AiChatController {
    #[inject(component)]
    pub svc: AiChatService,
}

impl BaseControl for AiChatController {
    type Service = AiChatService;

    fn get_service(&self) -> &Self::Service {
        &self.svc
    }
}

#[control_routes(prefix = "/ai_chat")]
impl AiChatController {
    #[summer_web::post("/touch/{id}")]
    async fn touch(&self, Path(id): Path<String>) -> ResultModel<()> {
        ResultModel::from_result(self.svc.touch(id).await)
    }

    #[summer_web::get("/select_by_type/{type}")]
    async fn select_by_type(&self, Path(r#type): Path<i16>) -> ResultModel<Vec<AiChat>> {
        ResultModel::from_result(Self::chat_type(r#type).map(|chat_type| self.svc.list_by_type(chat_type)))
    }

    #[summer_web::post("/revert")]
    async fn revert(&self, summer_web::axum::Json(body): summer_web::axum::Json<ReqAiChatRevert>) -> ResultModel<()> {
        ResultModel::from_result(self.svc.revert_from_item(body).await)
    }

    #[summer_web::post("/rename")]
    async fn rename(&self, summer_web::axum::Json(body): summer_web::axum::Json<ReqAiChatRename>) -> ResultModel<()> {
        ResultModel::from_result(self.svc.rename_by_user(body.id, body.title).await)
    }
}

impl AiChatController {
    fn chat_type(value: i16) -> anyhow::Result<AiChatType> {
        let Some(chat_type) = AiChatType::from_i16(value) else {
            return Err(ResultCode::ChatTypeUnknown.into());
        };
        Ok(chat_type)
    }
}
