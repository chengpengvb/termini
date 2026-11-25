use summer::plugin::Service;
use crate::web::control_base::BaseControl;
use crate::model::command_rule::CommandRule;
use crate::model::resp::resp_model::ResultModel;
use termini_macros::{base_control, control_routes};
use crate::service::service_command_rule::CommandRuleService;
use crate::model::req::req_command_rule::ReqCreateCommandRule;

#[base_control(CommandRule, prefix = "/command_rule")]
#[derive(Clone, Service)]
pub struct CommandRuleController {
    #[inject(component)]
    svc: CommandRuleService,
}

impl BaseControl for CommandRuleController {
    type Service = CommandRuleService;

    fn get_service(&self) -> &Self::Service {
        &self.svc
    }
}

#[control_routes(prefix = "/command_rule")]
impl CommandRuleController {
    #[summer_web::post("/create")]
    async fn create(&self, summer_web::axum::Json(body): summer_web::axum::Json<ReqCreateCommandRule>) -> ResultModel<CommandRule> {
        let tokens: Vec<String> = body.pattern.split_whitespace().map(|token| token.to_string()).collect();
        let device_id = body.device_id.unwrap_or_default();
        let justification = body.justification.unwrap_or_default();
        ResultModel::from_result(self.svc.create(tokens, body.decision, device_id, justification).await)
    }
}
