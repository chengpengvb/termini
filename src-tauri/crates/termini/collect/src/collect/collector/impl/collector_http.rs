use std::sync::Arc;
use summer::tracing::debug;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use std::time::{Duration, Instant};
use crate::collect::dto::CollectContext;
use crate::collect::collector::Collector;
use crate::collect::collector::AgentCollector;
use crate::base::base_app::{AppContext, BaseApp};
use crate::model::job_indicator::JobIndicatorProtocol;
use crate::model::job_indicator_value::JobIndicatorValue;
use crate::model::dto::job::indicator_arg_template::{IndicatorArgTemplate, IndicatorArgType};

pub const HTTP_ARG_ID: &str = "http-url";
pub const HTTP_TIMEOUT: Duration = Duration::from_secs(10);
pub const HTTP_ARG_LABEL_I18N: &str = "job.indicator.arg.http.url";

#[base_app(sort = 254)]
#[derive(Clone, Service)]
pub struct CollectorHttp {
    #[inject(component)]
    agent_collector: AgentCollector,
}

#[async_trait]
impl BaseApp for CollectorHttp {
    fn name(&self) -> &'static str {
        "CollectorHttp"
    }

    async fn start(&self, _ctx: &AppContext) -> anyhow::Result<()> {
        self.agent_collector.register(Arc::new(self.clone()));
        Ok(())
    }
}

#[async_trait]
impl Collector for CollectorHttp {
    fn get_name(&self) -> &'static str {
        "CollectorHttp"
    }

    fn get_protocol(&self) -> i16 {
        JobIndicatorProtocol::Http as i16
    }

    fn get_arg_template(&self) -> IndicatorArgTemplate {
        IndicatorArgTemplate { id: HTTP_ARG_ID.to_string(), arg_type: IndicatorArgType::Shell as i16, label_i18n: HTTP_ARG_LABEL_I18N.to_string(), protocol: self.get_protocol() }
    }

    async fn collection(&self, ctx: &CollectContext) -> anyhow::Result<Vec<JobIndicatorValue>> {
        let url = Self::resolve_url(ctx)?;
        let value = match Self::request_millis(&url).await {
            Ok(millis) => ctx.new_value(Some(format!("{:.2}", millis)), true, None),
            Err(error) => ctx.new_value(None, false, Some(format!("HTTP probe failed: {}", error))),
        };
        debug!("[collect] job_id={} url={} success={} value={:?}", ctx.job_id(), url, value.success, value.value);
        Ok(vec![value])
    }
}

impl CollectorHttp {
    fn resolve_url(ctx: &CollectContext) -> anyhow::Result<String> {
        let Some(url) = ctx.arg().map(|arg| arg.value.trim().to_string()).filter(|value| !value.is_empty()) else {
            anyhow::bail!("collection argument requires a full URL, e.g. https://example.com/health");
        };
        if !url.starts_with("http://") && !url.starts_with("https://") {
            anyhow::bail!("URL must start with http:// or https://: {}", url);
        }
        Ok(url)
    }

    async fn request_millis(url: &str) -> anyhow::Result<f64> {
        let client = reqwest::Client::builder().timeout(HTTP_TIMEOUT).danger_accept_invalid_certs(true).no_proxy().build()?;
        let started = Instant::now();
        let response = client.get(url).send().await?;
        let millis = started.elapsed().as_secs_f64() * 1000.0;
        let status = response.status();
        if status.is_client_error() || status.is_server_error() {
            anyhow::bail!("HTTP {}", status.as_u16());
        }
        Ok(millis)
    }
}
