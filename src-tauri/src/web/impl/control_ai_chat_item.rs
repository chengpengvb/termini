use summer::plugin::Service;
use crate::model::dto::shared_ptr::Shared;
use crate::web::control_base::BaseControl;
use crate::model::ai_chat_item::AiChatItem;
use summer_web::axum::extract::{Path, Query};
use crate::model::resp::resp_model::ResultModel;
use termini_macros::{base_control, control_routes};
use crate::model::req::req_ai_chat::ReqAiChatSearch;
use crate::service::service_ai_chat_item::AiChatItemService;
use crate::module::ai::shared::{ChatSessionCacheManager, ContextUsage, ContextUsageService};

#[base_control(AiChatItem, prefix = "/ai_chat_item")]
#[derive(Clone, Service)]
pub struct AiChatItemController {
    #[inject(component)]
    pub svc: AiChatItemService,

    #[inject(component)]
    pub context_usage_service: ContextUsageService,

    #[inject(component)]
    pub chat_session_cache_manager: ChatSessionCacheManager,
}

impl BaseControl for AiChatItemController {
    type Service = AiChatItemService;

    fn get_service(&self) -> &Self::Service {
        &self.svc
    }
}

#[control_routes(prefix = "/ai_chat_item")]
impl AiChatItemController {
    #[summer_web::get("/select_by_chat_id/{chat_id}")]
    async fn select_by_chat_id(&self, Path(chat_id): Path<String>) -> ResultModel<Vec<Shared<AiChatItem>>> {
        ResultModel::from_result(self.svc.select_by_chat_id(chat_id).await)
    }

    #[summer_web::get("/search_chat_ids")]
    async fn search_chat_ids(&self, Query(query): Query<ReqAiChatSearch>) -> ResultModel<Vec<String>> {
        ResultModel::from_result(self.svc.search_chat_ids(&query.q).await)
    }

    #[summer_web::get("/is_running/{chat_id}")]
    async fn is_running(&self, Path(chat_id): Path<String>) -> ResultModel<Option<String>> {
        ResultModel::ok(self.chat_session_cache_manager.get_req_id_by_chat_id(&chat_id))
    }

    #[summer_web::get("/context_usage/{chat_id}/{provide_id}")]
    async fn context_usage(&self, Path((chat_id, provide_id)): Path<(String, String)>) -> ResultModel<ContextUsage> {
        let provide = Some(provide_id.as_str()).filter(|id| !id.is_empty() && *id != "-");
        ResultModel::from_result(self.context_usage_service.usage(&chat_id, provide).await)
    }
}
