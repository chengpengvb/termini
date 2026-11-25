use summer_web::axum::Json;
use summer::plugin::Service;
use async_trait::async_trait;
use crate::base::base_app::BaseApp;
use termini_macros::{base_app, control_routes};
use crate::model::resp::resp_model::ResultModel;
use crate::service::service_file_edit::FileEditService;
use crate::model::resp::resp_file_edit::RespFileEditRead;
use crate::model::req::req_file_edit::{ReqFileEditRead, ReqFileEditWrite};

#[base_app(sort = 423)]
#[derive(Clone, Service)]
pub struct FileEditController {
    #[inject(component)]
    file_edit_service: FileEditService,
}

#[async_trait]
impl BaseApp for FileEditController {
    fn phase(&self) -> i16 {
        40
    }

    fn name(&self) -> &'static str {
        "FileEditController"
    }
}

#[control_routes(prefix = "/file_edit")]
impl FileEditController {
    #[summer_web::post("/read")]
    async fn read(&self, Json(body): Json<ReqFileEditRead>) -> ResultModel<RespFileEditRead> {
        ResultModel::from_result(self.file_edit_service.read(body).await)
    }

    #[summer_web::post("/write")]
    async fn write(&self, Json(body): Json<ReqFileEditWrite>) -> ResultModel<()> {
        ResultModel::from_result(self.file_edit_service.write(body).await)
    }
}
