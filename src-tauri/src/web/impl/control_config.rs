use crate::model::config::Config;
use crate::model::req::req_config::{
    ReqSaveAuditPolicy, ReqSaveCloseAction, ReqSaveCodeIndex, ReqSaveJobAlert, ReqSaveJobCollect, ReqSaveJobDiscover, ReqSaveJobSshPool, ReqSaveNotify, ReqSaveRoleModel, ReqSaveSuggest, ReqSaveSystem,
};
use summer::plugin::Service;
use crate::web::control_base::BaseControl;
use crate::model::resp::resp_model::ResultModel;
use crate::service::service_config::ConfigService;
use termini_macros::{base_control, control_routes};

#[base_control(Config, prefix = "/config")]
#[derive(Clone, Service)]
pub struct ConfigController {
    #[inject(component)]
    pub svc: ConfigService,
}

impl BaseControl for ConfigController {
    type Service = ConfigService;

    fn get_service(&self) -> &Self::Service {
        &self.svc
    }
}

#[control_routes(prefix = "/config")]
impl ConfigController {
    #[summer_web::post("/save_suggest")]
    async fn save_suggest(&self, summer_web::axum::Json(body): summer_web::axum::Json<ReqSaveSuggest>) -> ResultModel<()> {
        ResultModel::from_result(self.svc.save_suggest(body.enabled, body.count).await)
    }

    #[summer_web::post("/save_code_index")]
    async fn save_code_index(&self, summer_web::axum::Json(body): summer_web::axum::Json<ReqSaveCodeIndex>) -> ResultModel<()> {
        ResultModel::from_result(self.svc.save_code_index(body.ignore, body.include_tests).await)
    }

    #[summer_web::post("/save_system")]
    async fn save_system(&self, summer_web::axum::Json(body): summer_web::axum::Json<ReqSaveSystem>) -> ResultModel<()> {
        ResultModel::from_result(self.svc.save_system(body.system_language, body.local_shell_id, body.shell_cpu_percent).await)
    }

    #[summer_web::post("/save_job_collect")]
    async fn save_job_collect(&self, summer_web::axum::Json(body): summer_web::axum::Json<ReqSaveJobCollect>) -> ResultModel<()> {
        ResultModel::from_result(self.svc.save_job_collect(body.exec_timeout_secs, body.worker_parallelism, body.indicator_value_cache_size).await)
    }

    #[summer_web::post("/save_job_ssh_pool")]
    async fn save_job_ssh_pool(&self, summer_web::axum::Json(body): summer_web::axum::Json<ReqSaveJobSshPool>) -> ResultModel<()> {
        ResultModel::from_result(self.svc.save_job_ssh_pool(body.max_size, body.idle_secs).await)
    }

    #[summer_web::post("/save_audit_policy")]
    async fn save_audit_policy(&self, summer_web::axum::Json(body): summer_web::axum::Json<ReqSaveAuditPolicy>) -> ResultModel<()> {
        ResultModel::from_result(self.svc.save_audit_policy(body.audit_policy).await)
    }

    #[summer_web::post("/save_job_alert")]
    async fn save_job_alert(&self, summer_web::axum::Json(body): summer_web::axum::Json<ReqSaveJobAlert>) -> ResultModel<()> {
        ResultModel::from_result(self.svc.save_job_alert(body.ai_provide_id).await)
    }

    #[summer_web::post("/save_job_discover")]
    async fn save_job_discover(&self, summer_web::axum::Json(body): summer_web::axum::Json<ReqSaveJobDiscover>) -> ResultModel<()> {
        ResultModel::from_result(self.svc.save_job_discover(body.ai_provide_id, body.parallelism).await)
    }

    #[summer_web::post("/save_role_model")]
    async fn save_role_model(&self, summer_web::axum::Json(body): summer_web::axum::Json<ReqSaveRoleModel>) -> ResultModel<()> {
        ResultModel::from_result(self.svc.save_role_model(body).await)
    }

    #[summer_web::post("/save_notify")]
    async fn save_notify(&self, summer_web::axum::Json(body): summer_web::axum::Json<ReqSaveNotify>) -> ResultModel<()> {
        ResultModel::from_result(self.svc.save_notify(body).await)
    }

    #[summer_web::post("/save_close_action")]
    async fn save_close_action(&self, summer_web::axum::Json(body): summer_web::axum::Json<ReqSaveCloseAction>) -> ResultModel<()> {
        ResultModel::from_result(self.svc.save_close_action(body.close_action).await)
    }
}
