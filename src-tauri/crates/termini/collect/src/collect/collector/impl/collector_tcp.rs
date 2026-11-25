use std::sync::Arc;
use summer::tracing::debug;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use crate::dao::dao_base::BaseDao;
use std::time::{Duration, Instant};
use crate::dao::dao_device::DeviceDao;
use crate::collect::dto::CollectContext;
use crate::collect::collector::Collector;
use crate::collect::collector::AgentCollector;
use crate::base::base_app::{AppContext, BaseApp};
use crate::model::job_indicator::JobIndicatorProtocol;
use crate::model::job_indicator_value::JobIndicatorValue;
use crate::model::dto::job::indicator_arg_template::{IndicatorArgTemplate, IndicatorArgType};

pub const TCP_ARG_ID: &str = "tcp-port";
pub const TCP_TIMEOUT: Duration = Duration::from_secs(5);
pub const TCP_ARG_LABEL_I18N: &str = "job.indicator.arg.tcp.port";

#[base_app(sort = 255)]
#[derive(Clone, Service)]
pub struct CollectorTcp {
    #[inject(component)]
    device_dao: DeviceDao,

    #[inject(component)]
    agent_collector: AgentCollector,
}

#[async_trait]
impl BaseApp for CollectorTcp {
    fn name(&self) -> &'static str {
        "CollectorTcp"
    }

    async fn start(&self, _ctx: &AppContext) -> anyhow::Result<()> {
        self.agent_collector.register(Arc::new(self.clone()));
        Ok(())
    }
}

#[async_trait]
impl Collector for CollectorTcp {
    fn get_name(&self) -> &'static str {
        "CollectorTcp"
    }

    fn get_protocol(&self) -> i16 {
        JobIndicatorProtocol::Tcp as i16
    }

    fn get_arg_template(&self) -> IndicatorArgTemplate {
        IndicatorArgTemplate { id: TCP_ARG_ID.to_string(), arg_type: IndicatorArgType::Number as i16, label_i18n: TCP_ARG_LABEL_I18N.to_string(), protocol: self.get_protocol() }
    }

    async fn collection(&self, ctx: &CollectContext) -> anyhow::Result<Vec<JobIndicatorValue>> {
        let host = self.resolve_host(ctx).await?;
        let port = Self::resolve_port(ctx)?;
        let value = match Self::connect_millis(&host, port).await {
            Ok(millis) => ctx.new_value(Some(format!("{:.2}", millis)), true, None),
            Err(error) => ctx.new_value(None, false, Some(format!("TCP port unreachable: {}", error))),
        };
        debug!("[collect] job_id={} host={} port={} success={} value={:?}", ctx.job_id(), host, port, value.success, value.value);
        Ok(vec![value])
    }
}

impl CollectorTcp {
    async fn resolve_host(&self, ctx: &CollectContext) -> anyhow::Result<String> {
        let device_id = ctx.device_id();
        let Some(device) = self.device_dao.find_by_id(device_id.clone()).await? else {
            anyhow::bail!("device not found, it may have been deleted");
        };
        let host = device.read().host.clone();
        if host.trim().is_empty() {
            anyhow::bail!("device has no IP or hostname configured");
        }
        Ok(host)
    }

    fn resolve_port(ctx: &CollectContext) -> anyhow::Result<u16> {
        let Some(port) = ctx.arg().and_then(|arg| arg.value.trim().parse::<u16>().ok()).filter(|port| *port > 0) else {
            anyhow::bail!("collection argument requires a port number to probe (1~65535)");
        };
        Ok(port)
    }

    async fn connect_millis(host: &str, port: u16) -> anyhow::Result<f64> {
        let started = Instant::now();
        let connect = tokio::net::TcpStream::connect((host, port));
        tokio::time::timeout(TCP_TIMEOUT, connect).await.map_err(|_| anyhow::anyhow!("connection timed out ({}s)", TCP_TIMEOUT.as_secs()))??;
        Ok(started.elapsed().as_secs_f64() * 1000.0)
    }
}
