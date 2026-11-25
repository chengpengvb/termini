use summer_web::axum;
use summer::plugin::Service;
use crate::model::ai_skill::AiSkill;
use crate::model::dto::shared_ptr::Shared;
use crate::web::control_base::BaseControl;
use crate::model::resp::resp_model::ResultModel;
use termini_macros::{base_control, control_routes};
use crate::service::service_ai_skill::AiSkillService;

#[base_control(AiSkill, prefix = "/ai_skill", skip = "insert,update")]
#[derive(Clone, Service)]
pub struct AiSkillController {
    #[inject(component)]
    svc: AiSkillService,
}

impl BaseControl for AiSkillController {
    type Service = AiSkillService;

    fn get_service(&self) -> &Self::Service {
        &self.svc
    }
}

#[control_routes(prefix = "/ai_skill")]
impl AiSkillController {
    #[summer_web::post("/insert")]
    async fn insert(&self, axum::Json(body): axum::Json<AiSkill>) -> ResultModel<Shared<AiSkill>> {
        ResultModel::from_result(self.svc.create_skill(body).await)
    }

    #[summer_web::post("/update")]
    async fn update(&self, axum::Json(body): axum::Json<AiSkill>) -> ResultModel<Shared<AiSkill>> {
        ResultModel::from_result(self.svc.update_skill(body).await)
    }
}
