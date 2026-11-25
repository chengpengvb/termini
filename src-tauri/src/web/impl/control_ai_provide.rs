use summer::plugin::Service;
use summer_web::axum::extract::Path;
use crate::model::ai_provide::AiProvide;
use crate::model::dto::shared_ptr::Shared;
use crate::web::control_base::BaseControl;
use termini_macros::{base_control, control_routes};
use crate::service::service_ai_provide::AiProvideService;
use crate::model::resp::resp_model::{ResultCode, ResultModel};
use crate::model::resp::resp_ai_provider_info::RespAiProviderInfo;

#[base_control(AiProvide, prefix = "/ai_provide")]
#[derive(Clone, Service)]
pub struct AiProvideController {
    #[inject(component)]
    pub svc: AiProvideService,
}

impl BaseControl for AiProvideController {
    type Service = AiProvideService;

    fn get_service(&self) -> &Self::Service {
        &self.svc
    }
}

#[control_routes(prefix = "/ai_provide")]
impl AiProvideController {
    #[summer_web::get("/list_template_provider_info")]
    async fn list_template_provider_info(&self) -> ResultModel<Vec<RespAiProviderInfo>> {
        ResultModel::ok(self.svc.list_template_provider_info().to_vec())
    }

    #[summer_web::get("/refresh")]
    async fn refresh(&self) -> ResultModel<Vec<Shared<AiProvide>>> {
        ResultModel::from_result(self.svc.refresh_cloud().await)
    }

    #[summer_web::get("/get_template_provider_info/{provide_id}")]
    async fn get_template_provider_info(&self, Path(provide_id): Path<i16>) -> ResultModel<RespAiProviderInfo> {
        if let Some(item) = self.svc.get_template_provider_info(provide_id) {
            return ResultModel::ok(item.clone());
        }
        ResultModel::fail_code(ResultCode::ProviderNotFound)
    }
}
