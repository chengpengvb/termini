use summer_web::axum::Json;
use summer::plugin::Service;
use summer_web::axum::extract::Path;
use crate::model::ai_project::AiProject;
use crate::model::dto::shared_ptr::Shared;
use crate::web::control_base::BaseControl;
use crate::model::resp::resp_model::ResultModel;
use termini_macros::{base_control, control_routes};
use crate::service::service_ai_project::AiProjectService;
use crate::model::req::req_ai_project::{ReqAiProjectCreate, ReqAiProjectRename};

#[base_control(AiProject, prefix = "/ai_project", skip = "insert, insert_batch, update, delete")]
#[derive(Clone, Service)]
pub struct AiProjectController {
    #[inject(component)]
    pub svc: AiProjectService,
}

impl BaseControl for AiProjectController {
    type Service = AiProjectService;

    fn get_service(&self) -> &Self::Service {
        &self.svc
    }
}

#[control_routes(prefix = "/ai_project")]
impl AiProjectController {
    #[summer_web::get("/list")]
    async fn list(&self) -> ResultModel<Vec<AiProject>> {
        ResultModel::ok(self.svc.list())
    }

    #[summer_web::post("/create")]
    async fn create(&self, Json(body): Json<ReqAiProjectCreate>) -> ResultModel<Shared<AiProject>> {
        ResultModel::from_result(self.svc.create(body.name).await)
    }

    #[summer_web::post("/rename")]
    async fn rename(&self, Json(body): Json<ReqAiProjectRename>) -> ResultModel<()> {
        ResultModel::from_result(self.svc.rename(body.id, body.name).await)
    }

    #[summer_web::delete("/delete/{id}")]
    async fn delete(&self, Path(id): Path<String>) -> ResultModel<()> {
        ResultModel::from_result(self.svc.remove(id).await)
    }
}
