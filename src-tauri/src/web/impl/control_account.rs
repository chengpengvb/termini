use summer::plugin::Service;
use async_trait::async_trait;
use crate::base::base_app::BaseApp;
use summer_web::axum::extract::Query;
use termini_macros::{base_app, control_routes};
use crate::model::resp::resp_model::ResultModel;
use crate::model::resp::resp_login_url::RespLoginUrl;
use crate::model::resp::resp_app_version::RespAboutInfo;
use crate::model::resp::resp_user_account::RespUserAccount;
use crate::service::service_account_token::AccountTokenService;
use crate::model::req::req_account::{ReqAccountCallback, ReqAccountCode, ReqAccountEmail, ReqAccountLogin, ReqAccountRegister, ReqFeedback, ReqOpenSite, ReqReportError};

#[base_app(sort = 420)]
#[derive(Clone, Service)]
pub struct AccountController {
    #[inject(component)]
    account_token_service: AccountTokenService,
}

#[async_trait]
impl BaseApp for AccountController {
    fn phase(&self) -> i16 {
        40
    }

    fn name(&self) -> &'static str {
        "AccountController"
    }
}

#[control_routes(prefix = "/account")]
impl AccountController {
    #[summer_web::get("/token")]
    async fn token(&self) -> ResultModel<Option<String>> {
        ResultModel::from_result(self.account_token_service.current_token().await)
    }

    #[summer_web::get("/login_url")]
    async fn login_url(&self) -> ResultModel<RespLoginUrl> {
        ResultModel::from_result(self.account_token_service.login_url().await)
    }

    #[summer_web::post("/client_login")]
    async fn client_login(&self, summer_web::axum::Json(body): summer_web::axum::Json<ReqAccountLogin>) -> ResultModel<()> {
        ResultModel::from_result(self.account_token_service.client_login(body.email, body.password).await)
    }

    #[summer_web::post("/register_code")]
    async fn register_code(&self, summer_web::axum::Json(body): summer_web::axum::Json<ReqAccountEmail>) -> ResultModel<()> {
        ResultModel::from_result(self.account_token_service.send_register_code(body.email).await)
    }

    #[summer_web::post("/register")]
    async fn register_user(&self, summer_web::axum::Json(body): summer_web::axum::Json<ReqAccountRegister>) -> ResultModel<()> {
        ResultModel::from_result(self.account_token_service.register_user(body.email, body.password, body.code).await)
    }

    #[summer_web::post("/reset_code")]
    async fn reset_code(&self, summer_web::axum::Json(body): summer_web::axum::Json<ReqAccountEmail>) -> ResultModel<()> {
        ResultModel::from_result(self.account_token_service.send_reset_code(body.email).await)
    }

    #[summer_web::post("/reset_password")]
    async fn reset_password(&self, summer_web::axum::Json(body): summer_web::axum::Json<ReqAccountRegister>) -> ResultModel<()> {
        ResultModel::from_result(self.account_token_service.reset_password(body.email, body.password, body.code).await)
    }

    #[summer_web::get("/portal_url")]
    async fn portal_url(&self) -> ResultModel<RespLoginUrl> {
        ResultModel::from_result(self.account_token_service.portal_url().await)
    }

    #[summer_web::get("/callback")]
    async fn callback(&self, Query(query): Query<ReqAccountCallback>) -> ResultModel<()> {
        ResultModel::from_result(self.account_token_service.save_token(query.token).await)
    }

    #[summer_web::get("/profile")]
    async fn profile(&self) -> ResultModel<RespUserAccount> {
        ResultModel::from_result(self.account_token_service.profile().await)
    }

    #[summer_web::delete("/logout")]
    async fn logout(&self) -> ResultModel<()> {
        ResultModel::from_result(self.account_token_service.delete_token().await)
    }

    #[summer_web::post("/delete_code")]
    async fn delete_code(&self) -> ResultModel<()> {
        ResultModel::from_result(self.account_token_service.send_delete_code().await)
    }

    #[summer_web::post("/delete_account")]
    async fn delete_account(&self, summer_web::axum::Json(body): summer_web::axum::Json<ReqAccountCode>) -> ResultModel<()> {
        ResultModel::from_result(self.account_token_service.delete_account(body.code).await)
    }

    #[summer_web::get("/about")]
    async fn about(&self) -> ResultModel<RespAboutInfo> {
        ResultModel::from_result(self.account_token_service.about_info().await)
    }

    #[summer_web::get("/open_site")]
    async fn open_site(&self, Query(query): Query<ReqOpenSite>) -> ResultModel<()> {
        ResultModel::from_result(self.account_token_service.open_site(query.kind).await)
    }

    #[summer_web::post("/report_error")]
    async fn report_error(&self, summer_web::axum::Json(body): summer_web::axum::Json<ReqReportError>) -> ResultModel<()> {
        ResultModel::from_result(self.account_token_service.report_error(body.source, body.message, body.stack).await)
    }

    #[summer_web::post("/feedback")]
    async fn feedback(&self, summer_web::axum::Json(body): summer_web::axum::Json<ReqFeedback>) -> ResultModel<()> {
        ResultModel::from_result(self.account_token_service.submit_feedback(body.content).await)
    }
}
