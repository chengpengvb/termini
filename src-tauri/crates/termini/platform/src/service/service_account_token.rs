use reqwest::Method;
use tauri::AppHandle;
use tokio::sync::broadcast;
use summer::plugin::Service;
use async_trait::async_trait;
use crate::model::account_token;
use std::sync::{Mutex, OnceLock};
use crate::dao::dao_base::BaseDao;
use tauri_plugin_opener::OpenerExt;
use anyhow::{bail, Context, Result};
use tauri_plugin_deep_link::DeepLinkExt;
use crate::module::common::cloud_provide;
use crate::web::ws::ws_manager::WsManager;
use termini_macros::{base_app, base_service};
use crate::model::account_token::AccountToken;
use crate::base::base_app::{AppContext, BaseApp};
use crate::service::service_config::ConfigService;
use crate::dao::dao_account_token::AccountTokenDao;
use crate::model::resp::resp_login_url::RespLoginUrl;
use crate::model::resp::resp_app_version::RespAboutInfo;
use crate::model::resp::resp_model::{BizError, ResultCode};
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use crate::service::service_base_api::{ApiEvent, BaseApiService};
use crate::model::resp::event::resp_event_account::RespEventAccount;
use crate::model::resp::resp_user_account::{RespAccountToken, RespUserAccount};
use crate::model::server::resp::result_model::{ResultModel, CODE_OK, CODE_UNAUTHORIZED, CODE_UPGRADE_REQUIRED};

#[base_app(sort = 130)]
#[base_service(account_token::Entity, dao = "account_token_dao")]
#[derive(Clone, Service)]
pub struct AccountTokenService {
    #[inject(component)]
    handle: AppHandle,

    #[inject(component)]
    ws_manager: WsManager,

    #[inject(component)]
    config_service: ConfigService,

    #[inject(component)]
    account_token_dao: AccountTokenDao,
}

#[async_trait]
impl BaseApp for AccountTokenService {
    fn name(&self) -> &'static str {
        "AccountTokenService"
    }

    async fn start(&self, _ctx: &AppContext) -> Result<()> {
        Self::api_init(self.handle.package_info().version.to_string());
        let launch_tokens = self.bind_deep_link()?;
        if let Some(token) = self.get_token().await? {
            Self::api_set_token(Some(token));
        }
        self.spawn_session_loop();
        self.save_launch_tokens(launch_tokens).await;
        Ok(())
    }
}

impl BaseApiService for AccountTokenService {}

impl AccountTokenService {
    const SESSION_CHECK_INTERVAL_SECONDS: u64 = 300;

    fn spawn_session_loop(&self) {
        let svc = self.clone();
        tokio::spawn(async move {
            svc.run_session_loop().await;
        });
    }

    async fn run_session_loop(&self) {
        let mut rx = Self::api_subscribe();
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(Self::SESSION_CHECK_INTERVAL_SECONDS));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            tokio::select! {
                _ = interval.tick() => self.on_session_tick().await,
                event = rx.recv() => {
                    if Self::session_channel_closed(event) {
                        return;
                    }
                }
            }
        }
    }

    async fn on_session_tick(&self) {
        self.validate_session().await;
    }

    fn session_channel_closed(event: Result<ApiEvent, broadcast::error::RecvError>) -> bool {
        matches!(event, Err(broadcast::error::RecvError::Closed))
    }

    fn bind_deep_link(&self) -> Result<Vec<String>> {
        #[cfg(windows)]
        self.handle.deep_link().register_all().map_err(|err| anyhow::anyhow!("{err}"))?;
        #[cfg(target_os = "linux")]
        if let Err(err) = self.register_linux_url_handler() {
            summer::tracing::warn!("[account] 注册 termini:// 协议失败: {err:#}");
        }
        let svc = self.clone();
        self.handle.deep_link().on_open_url(move |event| {
            for url in event.urls() {
                svc.apply_deep_link(url.as_str());
            }
        });
        let (tx, mut rx) = unbounded_channel::<String>();
        let queued = {
            let mut pending = deep_link_pending().lock().unwrap_or_else(|err| err.into_inner());
            let _ = DEEP_LINK_TX.set(tx);
            pending.drain(..).collect::<Vec<_>>()
        };
        for url in &queued {
            self.apply_deep_link(url);
        }
        let svc = self.clone();
        tauri::async_runtime::spawn(async move {
            while let Some(url) = rx.recv().await {
                svc.apply_deep_link(&url);
            }
        });
        let mut tokens = Vec::new();
        if let Some(urls) = self.handle.deep_link().get_current().map_err(|err| anyhow::anyhow!("{err}"))? {
            tokens.extend(urls.iter().filter_map(|url| parse_deep_link_token(url.as_str())));
        }
        #[cfg(any(target_os = "windows", target_os = "linux"))]
        for arg in std::env::args() {
            if let Some(token) = parse_deep_link_token(&arg) {
                if !tokens.contains(&token) {
                    tokens.push(token);
                }
            }
        }
        for url in queued {
            if let Some(token) = parse_deep_link_token(&url) {
                if !tokens.contains(&token) {
                    tokens.push(token);
                }
            }
        }
        Ok(tokens)
    }

    async fn save_launch_tokens(&self, tokens: Vec<String>) {
        for token in tokens {
            if let Err(err) = self.save_token(token).await {
                summer::tracing::error!("[account] deep link save_token failed: {err:#}");
            }
        }
    }

    #[cfg(target_os = "linux")]
    fn register_linux_url_handler(&self) -> Result<()> {
        let status = std::process::Command::new("xdg-mime").args(["default", "Termini.desktop", "x-scheme-handler/termini"]).status().context("xdg-mime")?;
        if !status.success() {
            bail!("xdg-mime default failed: {status}");
        }
        Ok(())
    }

    fn apply_deep_link(&self, url: &str) {
        let Some(token) = parse_deep_link_token(url) else {
            let redacted = url.split("token=").next().unwrap_or("termini://");
            summer::tracing::warn!("[account] deep link ignored, no token: {redacted}");
            return;
        };
        summer::tracing::info!("[account] deep link token accepted, len={}", token.len());
        let svc = self.clone();
        tauri::async_runtime::spawn(async move {
            match svc.save_token(token).await {
                Ok(()) => summer::tracing::info!("[account] deep link save_token ok"),
                Err(err) => summer::tracing::error!("[account] deep link save_token failed: {err:#}"),
            }
        });
    }

    pub async fn get_token(&self) -> Result<Option<String>> {
        let Some(shared) = self.account_token_dao.find_by_id(AccountToken::DEFAULT_ID.to_string()).await? else {
            return Ok(None);
        };
        let token = shared.read().token.clone();
        if token.is_empty() {
            return Ok(None);
        }
        Ok(Some(token))
    }

    pub async fn save_token(&self, token: String) -> Result<()> {
        let trimmed = token.trim().to_string();
        if trimmed.is_empty() {
            return Err(ResultCode::TokenEmpty.into());
        }
        summer::tracing::info!("[account] save_token begin, len={}", trimmed.len());
        match self.register_device(&trimmed).await {
            Ok(DeviceRegisterOutcome::Ok) => {}
            Ok(DeviceRegisterOutcome::Revoked(message)) => {
                let payload = Self::login_error_payload(&message);
                if message == ResultCode::DeviceLimitExceeded.as_i32().to_string() {
                    summer::tracing::error!("[account] 设备数量超限, 取消本次登录");
                } else if message == ResultCode::UserDisabled.as_i32().to_string() {
                    summer::tracing::error!("[account] 账号已禁用, 取消本次登录");
                } else if message == CODE_UNAUTHORIZED.to_string() {
                    summer::tracing::error!("[account] 登录令牌已失效, 取消本次登录");
                } else {
                    summer::tracing::error!("[account] 登录令牌已失效, 取消本次登录: {}", message);
                }
                self.ws_manager.broadcast_all(&RespEventAccount::OnLoginError(payload.clone()).to_resp());
                bail!("{}", payload);
            }
            Ok(DeviceRegisterOutcome::Rejected(message)) => {
                summer::tracing::error!("[account] 设备注册被拒绝, 取消本次登录: {}", message);
                self.ws_manager.broadcast_all(&RespEventAccount::OnLoginError(message.clone()).to_resp());
                bail!("{}", message);
            }
            Err(error) => {
                let message = format!("{:#}", error);
                summer::tracing::error!("[account] 设备注册失败, 取消本次登录: {}", message);
                self.ws_manager.broadcast_all(&RespEventAccount::OnLoginError(message.clone()).to_resp());
                return Err(error);
            }
        }
        match self.account_token_dao.find_by_id(AccountToken::DEFAULT_ID.to_string()).await? {
            Some(_) => {
                let model = AccountToken { id: AccountToken::DEFAULT_ID.to_string(), token: trimmed.clone() };
                self.account_token_dao.update(model).await?;
            }
            None => {
                let model = AccountToken { id: AccountToken::DEFAULT_ID.to_string(), token: trimmed.clone() };
                self.account_token_dao.insert(model).await?;
            }
        }
        Self::api_set_token(Some(trimmed.clone()));
        cloud_provide::update_cloud_key(Some(trimmed.clone()));
        self.ws_manager.broadcast_all(&RespEventAccount::OnLogin(trimmed).to_resp());
        #[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
        crate::service::service_impl_system_util::show_main_window_with_attention(&self.handle);
        Ok(())
    }

    pub async fn delete_token(&self) -> Result<()> {
        self.account_token_dao.delete_by_id(AccountToken::DEFAULT_ID.to_string()).await?;
        Self::api_set_token(None);
        cloud_provide::update_cloud_key(None);
        self.ws_manager.broadcast_all(&RespEventAccount::OnLogout.to_resp());
        Ok(())
    }

    pub async fn current_token(&self) -> Result<Option<String>> {
        Ok(Self::api_token())
    }

    pub async fn login_url(&self) -> Result<RespLoginUrl> {
        let resp = self.config_service.login_url().await?;
        self.handle.opener().open_url(&resp.url, None::<&str>).map_err(|_| anyhow::Error::from(ResultCode::OpenUrlFailed))?;
        Ok(resp)
    }

    pub async fn client_login(&self, email: String, password: String) -> Result<()> {
        let result: ResultModel<RespAccountToken> = Self::api_send(Self::api_post("/api/userApi/clientLogin").json(&serde_json::json!({
            "email": email,
            "password": password,
        })))
        .await?;
        let token = Self::require_cloud_token(result)?;
        self.save_token(token).await
    }

    pub async fn send_register_code(&self, email: String) -> Result<()> {
        let result: ResultModel<serde_json::Value> = Self::api_send(Self::api_post("/api/userApi/registerCode").json(&serde_json::json!({
            "email": email,
        })))
        .await?;
        if result.code != CODE_OK {
            return Err(BizError { code: result.code, message: result.message }.into());
        }
        Ok(())
    }

    pub async fn send_reset_code(&self, email: String) -> Result<()> {
        let result: ResultModel<serde_json::Value> = Self::api_send(Self::api_post("/api/userApi/resetCode").json(&serde_json::json!({
            "email": email,
        })))
        .await?;
        if result.code != CODE_OK {
            return Err(BizError { code: result.code, message: result.message }.into());
        }
        Ok(())
    }

    pub async fn reset_password(&self, email: String, password: String, code: String) -> Result<()> {
        let result: ResultModel<serde_json::Value> = Self::api_send(Self::api_post("/api/userApi/resetPassword").json(&serde_json::json!({
            "email": email,
            "password": password,
            "code": code,
        })))
        .await?;
        if result.code != CODE_OK {
            return Err(BizError { code: result.code, message: result.message }.into());
        }
        Ok(())
    }

    pub async fn send_delete_code(&self) -> Result<()> {
        let result: ResultModel<serde_json::Value> = Self::api_send(Self::api_post("/api/userApi/deleteCode")).await?;
        if result.code != CODE_OK {
            return Err(BizError { code: result.code, message: result.message }.into());
        }
        Ok(())
    }

    pub async fn delete_account(&self, code: String) -> Result<()> {
        let result: ResultModel<serde_json::Value> = Self::api_send(Self::api_post("/api/userApi/deleteAccount").json(&serde_json::json!({
            "code": code,
        })))
        .await?;
        if result.code != CODE_OK {
            return Err(BizError { code: result.code, message: result.message }.into());
        }
        self.delete_token().await
    }

    pub async fn register_user(&self, email: String, password: String, code: String) -> Result<()> {
        let result: ResultModel<RespAccountToken> = Self::api_send(Self::api_post("/api/userApi/registerUser").json(&serde_json::json!({
            "email": email,
            "password": password,
            "code": code,
        })))
        .await?;
        let token = Self::require_cloud_token(result)?;
        self.save_token(token).await
    }

    fn require_cloud_token(result: ResultModel<RespAccountToken>) -> Result<String> {
        if result.code != CODE_OK {
            return Err(BizError { code: result.code, message: result.message }.into());
        }
        let Some(data) = result.data else {
            return Err(ResultCode::TokenEmpty.into());
        };
        if data.token.trim().is_empty() {
            return Err(ResultCode::TokenEmpty.into());
        }
        Ok(data.token)
    }

    pub async fn portal_url(&self) -> Result<RespLoginUrl> {
        let resp = self.config_service.portal_url().await?;
        self.handle.opener().open_url(&resp.url, None::<&str>).map_err(|_| anyhow::Error::from(ResultCode::OpenUrlFailed))?;
        Ok(resp)
    }

    async fn validate_session(&self) {
        let token = match self.get_token().await {
            Ok(Some(token)) => token,
            Ok(None) => return,
            Err(error) => {
                summer::tracing::warn!("[account] 读取本地令牌失败, 跳过本次会话校验: {:#}", error);
                return;
            }
        };
        match self.register_device(&token).await {
            Ok(DeviceRegisterOutcome::Ok) => {}
            Ok(DeviceRegisterOutcome::Revoked(message)) => {
                summer::tracing::error!("[account] 登录令牌已失效, 强制登出: {}", message);
                let _ = self.delete_token().await;
                let payload = Self::login_error_payload(&message);
                self.ws_manager.broadcast_all(&RespEventAccount::OnLoginError(payload).to_resp());
            }
            Ok(DeviceRegisterOutcome::Rejected(message)) => {
                summer::tracing::warn!("[account] 设备注册被拒绝, 等待下次校验: {}", message);
            }
            Err(error) => {
                summer::tracing::warn!("[account] 会话校验请求失败(离线?), 稍后重试: {:#}", error);
            }
        }
    }

    async fn register_device(&self, token: &str) -> Result<DeviceRegisterOutcome> {
        let device_id = self.config_service.ensure_client_device_id().await?;
        let device_name = sysinfo::System::host_name().unwrap_or_else(|| "unknown".to_string());
        let body = serde_json::json!({ "deviceId": device_id, "deviceName": device_name, "platform": std::env::consts::OS, "version": Self::api_version() });
        let request = Self::api_request(Method::POST, "/api/deviceApi/register", Some(token.to_string())).json(&body);
        let result: ResultModel<serde_json::Value> = Self::api_send(request).await?;
        if result.code == CODE_UPGRADE_REQUIRED {
            return Ok(DeviceRegisterOutcome::Ok);
        }
        if result.code == CODE_UNAUTHORIZED {
            return Ok(DeviceRegisterOutcome::Revoked(CODE_UNAUTHORIZED.to_string()));
        }
        if result.code == ResultCode::UserDisabled.as_i32() {
            return Ok(DeviceRegisterOutcome::Revoked(ResultCode::UserDisabled.as_i32().to_string()));
        }
        if result.code == ResultCode::DeviceLimitExceeded.as_i32() {
            return Ok(DeviceRegisterOutcome::Revoked(ResultCode::DeviceLimitExceeded.as_i32().to_string()));
        }
        if result.code != CODE_OK {
            return Ok(DeviceRegisterOutcome::Rejected(result.message));
        }
        Ok(DeviceRegisterOutcome::Ok)
    }

    pub async fn about_info(&self) -> Result<RespAboutInfo> {
        let website = match self.config_service.fetch_system_config().await {
            Ok(config) => config.website_url.map(|item| item.trim().trim_end_matches('/').to_string()).filter(|item| !item.is_empty()),
            Err(error) => {
                summer::tracing::warn!("[account] 获取官网地址失败(离线?): {:#}", error);
                None
            }
        };
        Ok(RespAboutInfo {
            version: Self::api_version(),
            platform: std::env::consts::OS.to_string(),
            privacy_url: website.as_ref().map(|item| format!("{}/privacy", item)),
            terms_url: website.as_ref().map(|item| format!("{}/terms", item)),
            website_url: website,
        })
    }

    pub async fn open_site(&self, kind: String) -> Result<()> {
        let about = self.about_info().await?;
        let url = match kind.as_str() {
            "website" => about.website_url,
            "privacy" => about.privacy_url,
            "terms" => about.terms_url,
            _ => bail!("unknown site kind: {}", kind),
        };
        let Some(url) = url else {
            return Err(ResultCode::LoginUrlEmpty.into());
        };
        self.handle.opener().open_url(&url, None::<&str>).map_err(|_| anyhow::Error::from(ResultCode::OpenUrlFailed))?;
        Ok(())
    }

    pub async fn report_error(&self, source: String, message: String, stack: Option<String>) -> Result<()> {
        if self.get_token().await?.is_none() {
            return Err(ResultCode::NotLoggedIn.into());
        }
        let body = serde_json::json!({ "source": source, "message": message, "stack": stack, "platform": std::env::consts::OS, "version": Self::api_version() });
        Self::api_result::<serde_json::Value>(Self::api_post("/api/clientErrorApi/report").json(&body)).await?;
        Ok(())
    }

    pub async fn submit_feedback(&self, content: String) -> Result<()> {
        let trimmed = content.trim().to_string();
        if trimmed.is_empty() {
            bail!("feedback content cannot be empty");
        }
        if self.get_token().await?.is_none() {
            return Err(ResultCode::NotLoggedIn.into());
        }
        let body = serde_json::json!({ "content": trimmed, "platform": std::env::consts::OS, "version": Self::api_version() });
        Self::api_result::<serde_json::Value>(Self::api_post("/api/feedbackApi/submit").json(&body)).await?;
        Ok(())
    }

    pub async fn profile(&self) -> Result<RespUserAccount> {
        if self.get_token().await?.is_none() {
            return Err(ResultCode::Unauthorized.into());
        }
        let body: ResultModel<RespUserAccount> = Self::api_send(Self::api_get("/api/userApi/profile")).await?;
        if body.code == CODE_UPGRADE_REQUIRED {
            bail!("{}", body.message);
        }
        if body.code == CODE_UNAUTHORIZED {
            summer::tracing::error!("[account] 账号令牌已失效, 强制登出: {}", body.message);
            let _ = self.delete_token().await;
            return Err(BizError { code: CODE_UNAUTHORIZED, message: body.message }.into());
        }
        if body.code != CODE_OK {
            bail!("{}", body.message);
        }
        let mut account = body.data.context("profile data is empty")?;
        let system = self.config_service.fetch_system_config().await?;
        account.credit_scale = system.credit_scale;
        Ok(account)
    }

    fn login_error_payload(message: &str) -> String {
        if message == ResultCode::UserDisabled.as_i32().to_string() || message == ResultCode::DeviceLimitExceeded.as_i32().to_string() || message == CODE_UNAUTHORIZED.to_string() {
            return message.to_string();
        }
        "Login has expired: this device may have been removed on the website".to_string()
    }
}

enum DeviceRegisterOutcome {
    Ok,
    Revoked(String),
    Rejected(String),
}

static DEEP_LINK_TX: OnceLock<UnboundedSender<String>> = OnceLock::new();
static DEEP_LINK_PENDING: OnceLock<Mutex<Vec<String>>> = OnceLock::new();

fn deep_link_pending() -> &'static Mutex<Vec<String>> {
    DEEP_LINK_PENDING.get_or_init(|| Mutex::new(Vec::new()))
}

pub fn submit_deep_link_url(url: String) {
    let url = url.trim().to_string();
    if !url.starts_with("termini://") {
        return;
    }
    let mut pending = deep_link_pending().lock().unwrap_or_else(|err| err.into_inner());
    if let Some(tx) = DEEP_LINK_TX.get() {
        let _ = tx.send(url);
        return;
    }
    pending.push(url);
}

fn parse_deep_link_token(url: &str) -> Option<String> {
    let url = url.trim();
    if let Some(token) = parse_deep_link_token_via_url(url) {
        return Some(token);
    }
    parse_deep_link_token_raw(url)
}

fn parse_deep_link_token_via_url(url: &str) -> Option<String> {
    let parsed = url::Url::parse(url).ok()?;
    if parsed.scheme() != "termini" {
        return None;
    }
    let token = parsed.query_pairs().find(|(key, _)| key == "token")?.1.into_owned();
    if token.is_empty() {
        None
    } else {
        Some(token)
    }
}

fn parse_deep_link_token_raw(url: &str) -> Option<String> {
    if !url.starts_with("termini://") {
        return None;
    }
    let query = url.split_once('?')?.1;
    for pair in query.split('&') {
        let pair = pair.split_once('#').map(|(item, _)| item).unwrap_or(pair);
        if let Some(value) = pair.strip_prefix("token=") {
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}
