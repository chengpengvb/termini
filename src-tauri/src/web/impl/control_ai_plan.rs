use summer_web::axum::Json;
use summer::plugin::Service;
use summer_web::axum::extract::Path;
use crate::model::ai_plan_run::AiPlanRun;
use crate::model::dto::shared_ptr::Shared;
use crate::module::ai::flow::FlowPlanTask;
use crate::web::control_base::BaseControl;
use crate::model::ai_plan_task::AiPlanTask;
use crate::model::resp::resp_model::ResultModel;
use termini_macros::{base_control, control_routes};
use crate::service::service_ai_plan_task::AiPlanTaskService;
use crate::service::service_ai_plan_run::{AiPlanRunService, RespPlanRun};

#[base_control(AiPlanTask, prefix = "/ai_plan_task")]
#[derive(Clone, Service)]
pub struct AiPlanTaskController {
    #[inject(component)]
    pub svc: AiPlanTaskService,

    #[inject(component)]
    pub flow_plan_task: FlowPlanTask,

    #[inject(component)]
    pub ai_plan_run_service: AiPlanRunService,
}

impl BaseControl for AiPlanTaskController {
    type Service = AiPlanTaskService;

    fn get_service(&self) -> &Self::Service {
        &self.svc
    }
}

#[control_routes(prefix = "/ai_plan_task")]
impl AiPlanTaskController {
    #[summer_web::post("/save")]
    async fn save(&self, Json(task): Json<AiPlanTask>) -> ResultModel<Shared<AiPlanTask>> {
        ResultModel::from_result(self.svc.save_task(task).await)
    }

    #[summer_web::post("/run_now/{id}")]
    async fn run_now(&self, Path(id): Path<String>) -> ResultModel<AiPlanRun> {
        ResultModel::from_result(self.flow_plan_task.run_task(id).await)
    }

    #[summer_web::delete("/delete_with_runs/{id}")]
    async fn delete_with_runs(&self, Path(id): Path<String>) -> ResultModel<()> {
        ResultModel::from_result(self.svc.delete_task(id).await)
    }

    #[summer_web::get("/runs")]
    async fn runs(&self) -> ResultModel<Vec<RespPlanRun>> {
        ResultModel::from_result(self.ai_plan_run_service.select_recent().await)
    }

    #[summer_web::get("/runs_by_task/{task_id}")]
    async fn runs_by_task(&self, Path(task_id): Path<String>) -> ResultModel<Vec<RespPlanRun>> {
        ResultModel::from_result(self.ai_plan_run_service.select_by_task_id(task_id).await)
    }

    #[summer_web::post("/cancel_run/{run_id}")]
    async fn cancel_run(&self, Path(run_id): Path<String>) -> ResultModel<()> {
        ResultModel::from_result(self.flow_plan_task.cancel_run(&run_id))
    }

    #[summer_web::delete("/delete_run/{run_id}")]
    async fn delete_run(&self, Path(run_id): Path<String>) -> ResultModel<()> {
        ResultModel::from_result(self.ai_plan_run_service.delete_run(run_id).await)
    }
}
