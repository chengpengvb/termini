use summer::plugin::Service;
use crate::base::base_app::BaseApp;
use termini_macros::{base_app, control_routes};
use crate::model::resp::resp_model::ResultModel;
use crate::service::service_config::ConfigService;
use crate::module::ai::code_index::index_guard::MAX_INDEX_DIRS;
use crate::model::req::req_config::{ReqRebuildIndexDir, ReqSaveIndexDirs};
use crate::service::service_code_index::{CodeIndexService, IndexDirStatus};

#[base_app(sort = 900)]
#[derive(Clone, Service)]
pub struct CodeIndexController {
    #[inject(component)]
    pub config_service: ConfigService,

    #[inject(component)]
    pub code_index_service: CodeIndexService,
}

impl BaseApp for CodeIndexController {
    fn name(&self) -> &'static str {
        "CodeIndexController"
    }
}

#[control_routes(prefix = "/code_index")]
impl CodeIndexController {
    #[summer_web::get("/status")]
    async fn status(&self) -> ResultModel<Vec<IndexDirStatus>> {
        ResultModel::ok(self.code_index_service.status().await)
    }

    #[summer_web::post("/save_dirs")]
    async fn save_dirs(&self, summer_web::axum::Json(body): summer_web::axum::Json<ReqSaveIndexDirs>) -> ResultModel<Vec<String>> {
        if body.dirs.len() > MAX_INDEX_DIRS {
            return ResultModel::fail(format!("at most {} index directories can be configured", MAX_INDEX_DIRS));
        }
        let (valid, rejected) = self.code_index_service.screen_dirs(&body.dirs).await;
        if let Err(error) = self.config_service.save_index_dirs(valid).await {
            return ResultModel::fail(format!("{:#}", error));
        }
        if let Err(error) = self.code_index_service.reload().await {
            return ResultModel::fail(format!("{:#}", error));
        }
        ResultModel::ok(rejected)
    }

    #[summer_web::post("/rebuild")]
    async fn rebuild(&self, summer_web::axum::Json(body): summer_web::axum::Json<ReqRebuildIndexDir>) -> ResultModel<()> {
        ResultModel::from_result(self.code_index_service.rebuild(&body.dir).await)
    }
}
