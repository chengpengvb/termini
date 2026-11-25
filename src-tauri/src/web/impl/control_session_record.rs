use summer::plugin::Service;
use async_trait::async_trait;
use crate::base::base_app::BaseApp;
use summer_web::axum::extract::Path;
use termini_macros::{base_app, control_routes};
use crate::model::resp::resp_model::ResultModel;
use crate::model::session_record::SessionRecord;
use crate::service::service_session_record::SessionRecordService;
use crate::model::req::req_session_record::{ReqSessionRecordDeleteBatch, ReqSessionRecordDeleteBefore};

#[base_app(sort = 422)]
#[derive(Clone, Service)]
pub struct SessionRecordController {
    #[inject(component)]
    session_record_service: SessionRecordService,
}

#[async_trait]
impl BaseApp for SessionRecordController {
    fn phase(&self) -> i16 {
        40
    }

    fn name(&self) -> &'static str {
        "SessionRecordController"
    }
}

#[control_routes(prefix = "/session_record")]
impl SessionRecordController {
    #[summer_web::get("/list")]
    async fn list(&self) -> ResultModel<Vec<SessionRecord>> {
        ResultModel::from_result(self.session_record_service.list().await)
    }

    #[summer_web::get("/content/{id}")]
    async fn content(&self, Path(id): Path<String>) -> ResultModel<String> {
        ResultModel::from_result(self.session_record_service.content(id).await)
    }

    #[summer_web::delete("/delete/{id}")]
    async fn delete(&self, Path(id): Path<String>) -> ResultModel<bool> {
        ResultModel::from_result(self.session_record_service.delete(id).await.map(|_| true))
    }

    #[summer_web::post("/delete_batch")]
    async fn delete_batch(&self, summer_web::axum::Json(body): summer_web::axum::Json<ReqSessionRecordDeleteBatch>) -> ResultModel<usize> {
        ResultModel::from_result(self.session_record_service.delete_batch(body.ids).await)
    }

    #[summer_web::post("/delete_before")]
    async fn delete_before(&self, summer_web::axum::Json(body): summer_web::axum::Json<ReqSessionRecordDeleteBefore>) -> ResultModel<usize> {
        ResultModel::from_result(self.session_record_service.delete_before(body.started_before_millis).await)
    }
}
