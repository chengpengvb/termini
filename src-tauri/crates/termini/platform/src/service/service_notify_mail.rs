use anyhow::Result;
use serde_json::Value;
use summer::plugin::Service;
use crate::model::resp::resp_model::ResultCode;
use crate::service::service_base_api::BaseApiService;
use crate::service::service_account_token::AccountTokenService;

#[derive(Clone, Service)]
pub struct NotifyMailService {
    #[inject(component)]
    account_token_service: AccountTokenService,
}

impl BaseApiService for NotifyMailService {}

impl NotifyMailService {
    pub async fn send_mail(&self, to: &[String], subject: &str, body: &str) -> Result<()> {
        if self.account_token_service.get_token().await?.is_none() {
            return Err(ResultCode::NotLoggedIn.into());
        }
        let payload = serde_json::json!({"to": to, "subject": subject, "body": body});
        Self::api_result::<Value>(Self::api_post("/api/notifyApi/sendMail").json(&payload)).await?;
        Ok(())
    }
}
