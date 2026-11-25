use parking_lot::RwLock;
use std::sync::OnceLock;
use summer::tracing::info;
use tokio::sync::broadcast;
use async_trait::async_trait;
use reqwest::header::HeaderMap;
use serde::de::DeserializeOwned;
use std::net::{IpAddr, Ipv4Addr};
use anyhow::{bail, Context, Result};
use crate::model::resp::resp_model::BizError;
use std::sync::atomic::{AtomicI64, Ordering};
use reqwest::{Client, Method, RequestBuilder};
use crate::model::server::resp::result_model::{ResultModel, CODE_OK, CODE_UNAUTHORIZED, CODE_UPGRADE_REQUIRED};

const EVENT_CAPACITY: usize = 8;
const ACCOUNT_API_URL: &str = env!("ACCOUNT_API_URL");
const HEADER_CONFIG_VERSION: &str = "X-Config-Version";
const HEADER_CLIENT_VERSION: &str = "X-Client-Version";

static CLIENT: OnceLock<Client> = OnceLock::new();
static VERSION: OnceLock<String> = OnceLock::new();
static CONFIG_VERSION: AtomicI64 = AtomicI64::new(0);
static TOKEN: OnceLock<RwLock<Option<String>>> = OnceLock::new();
static EVENTS: OnceLock<broadcast::Sender<ApiEvent>> = OnceLock::new();

#[derive(Clone, Debug)]
pub enum ApiEvent {
    UpgradeRequired(String),
    ConfigVersionChanged { previous: i64, current: i64 },
}

#[async_trait]
pub trait BaseApiService: Send + Sync {
    fn api_init(version: String) {
        VERSION.set(version).expect("api version already initialized");
    }

    fn api_base() -> &'static str {
        ACCOUNT_API_URL.trim_end_matches('/')
    }

    fn api_version() -> String {
        VERSION.get().expect("api version not initialized").clone()
    }

    fn api_token() -> Option<String> {
        TOKEN.get_or_init(Default::default).read().clone()
    }

    fn api_set_token(token: Option<String>) {
        *TOKEN.get_or_init(Default::default).write() = token;
    }

    fn api_events() -> &'static broadcast::Sender<ApiEvent> {
        EVENTS.get_or_init(|| broadcast::channel(EVENT_CAPACITY).0)
    }

    fn api_subscribe() -> broadcast::Receiver<ApiEvent> {
        Self::api_events().subscribe()
    }

    fn api_observe_config_version(headers: &HeaderMap) {
        let Some(version) = headers.get(HEADER_CONFIG_VERSION).and_then(|value| value.to_str().ok()).and_then(|value| value.trim().parse::<i64>().ok()) else {
            return;
        };
        let previous = CONFIG_VERSION.swap(version, Ordering::SeqCst);
        if previous == 0 || previous == version {
            return;
        }
        info!("[api] 服务端配置版本变化 {} -> {}", previous, version);
        let _ = Self::api_events().send(ApiEvent::ConfigVersionChanged { previous, current: version });
    }

    fn api_http_client() -> &'static Client {
        CLIENT.get_or_init(|| Client::builder().http1_only().local_address(IpAddr::V4(Ipv4Addr::UNSPECIFIED)).build().expect("build api client"))
    }

    fn api_request(method: Method, path: &str, token: Option<String>) -> RequestBuilder {
        let request = Self::api_http_client().request(method, format!("{}{}", Self::api_base(), path)).header(HEADER_CLIENT_VERSION, Self::api_version());
        match token {
            Some(token) => request.bearer_auth(token),
            None => request,
        }
    }

    fn api_get(path: &str) -> RequestBuilder {
        Self::api_request(Method::GET, path, Self::api_token())
    }

    fn api_post(path: &str) -> RequestBuilder {
        Self::api_request(Method::POST, path, Self::api_token())
    }

    async fn api_send<T: DeserializeOwned + Send>(request: RequestBuilder) -> Result<ResultModel<T>> {
        let request = request.build().context("build api request")?;
        let path = request.url().path().to_string();
        let response = Self::api_http_client().execute(request).await.with_context(|| format!("request {}", path))?;
        Self::api_observe_config_version(response.headers());
        let result = response.json::<ResultModel<T>>().await.with_context(|| format!("parse {}", path))?;
        if result.code == CODE_UPGRADE_REQUIRED {
            let _ = Self::api_events().send(ApiEvent::UpgradeRequired(result.message.clone()));
        }
        Ok(result)
    }

    async fn api_result<T: DeserializeOwned + Send>(request: RequestBuilder) -> Result<Option<T>> {
        let result = Self::api_send::<T>(request).await?;
        if result.code == CODE_UNAUTHORIZED {
            return Err(BizError { code: CODE_UNAUTHORIZED, message: result.message }.into());
        }
        if result.code != CODE_OK {
            bail!("{}", result.message);
        }
        Ok(result.data)
    }

    async fn api_data<T: DeserializeOwned + Send>(request: RequestBuilder) -> Result<T> {
        Self::api_result::<T>(request).await?.context("api data is empty")
    }
}
