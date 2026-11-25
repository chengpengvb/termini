use std::sync::Arc;
use std::net::IpAddr;
use std::time::Duration;
use summer::tracing::debug;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use crate::dao::dao_base::BaseDao;
use crate::dao::dao_device::DeviceDao;
use crate::collect::dto::CollectContext;
use crate::collect::collector::Collector;
use crate::collect::collector::AgentCollector;
use crate::base::base_app::{AppContext, BaseApp};
use crate::model::job_indicator::JobIndicatorProtocol;
use crate::model::job_indicator_value::JobIndicatorValue;
use crate::model::dto::job::indicator_arg_template::{IndicatorArgTemplate, IndicatorArgType};

pub const ICMP_COUNT_MAX: u16 = 10;
pub const ICMP_COUNT_DEFAULT: u16 = 4;
pub const ICMP_ARG_ID: &str = "icmp-count";
pub const ICMP_TIMEOUT: Duration = Duration::from_secs(2);
pub const ICMP_ARG_LABEL_I18N: &str = "job.indicator.arg.icmp.count";

#[base_app(sort = 253)]
#[derive(Clone, Service)]
pub struct CollectorIcmp {
    #[inject(component)]
    device_dao: DeviceDao,

    #[inject(component)]
    agent_collector: AgentCollector,
}

#[async_trait]
impl BaseApp for CollectorIcmp {
    fn name(&self) -> &'static str {
        "CollectorIcmp"
    }

    async fn start(&self, _ctx: &AppContext) -> anyhow::Result<()> {
        self.agent_collector.register(Arc::new(self.clone()));
        Ok(())
    }
}

#[async_trait]
impl Collector for CollectorIcmp {
    fn get_name(&self) -> &'static str {
        "CollectorIcmp"
    }

    fn get_protocol(&self) -> i16 {
        JobIndicatorProtocol::Icmp as i16
    }

    fn get_arg_template(&self) -> IndicatorArgTemplate {
        IndicatorArgTemplate { id: ICMP_ARG_ID.to_string(), arg_type: IndicatorArgType::Number as i16, label_i18n: ICMP_ARG_LABEL_I18N.to_string(), protocol: self.get_protocol() }
    }

    async fn collection(&self, ctx: &CollectContext) -> anyhow::Result<Vec<JobIndicatorValue>> {
        let host = self.resolve_host(ctx).await?;
        let count = Self::resolve_count(ctx);
        let value = match Self::ping_avg_millis(&host, count).await {
            Ok(millis) => ctx.new_value(Some(format!("{:.2}", millis)), true, None),
            Err(error) => ctx.new_value(None, false, Some(format!("device unreachable: {}", error))),
        };
        debug!("[collect] job_id={} host={} count={} success={} value={:?}", ctx.job_id(), host, count, value.success, value.value);
        Ok(vec![value])
    }
}

impl CollectorIcmp {
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

    fn resolve_count(ctx: &CollectContext) -> u16 {
        let count = ctx.arg().and_then(|arg| arg.value.trim().parse::<u16>().ok()).unwrap_or(ICMP_COUNT_DEFAULT);
        count.clamp(1, ICMP_COUNT_MAX)
    }

    async fn resolve_addr(host: &str) -> anyhow::Result<IpAddr> {
        if let Ok(addr) = host.parse::<IpAddr>() {
            return Ok(addr);
        }
        let Some(addr) = tokio::net::lookup_host((host, 0)).await?.next() else {
            anyhow::bail!("resolve host failed: {}", host);
        };
        Ok(addr.ip())
    }

    #[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
    async fn ping_avg_millis(host: &str, count: u16) -> anyhow::Result<f64> {
        let mut received = 0u16;
        let mut total = Duration::ZERO;
        let addr = Self::resolve_addr(host).await?;
        use ping_async::{IcmpEchoRequestor, IcmpEchoStatus};
        let requestor = IcmpEchoRequestor::new(addr, None, None, Some(ICMP_TIMEOUT))?;
        let mut last_error = String::from("no reply");
        for _ in 0..count {
            match requestor.send().await {
                Ok(reply) if reply.status() == IcmpEchoStatus::Success => {
                    total += reply.round_trip_time();
                    received += 1;
                }
                Ok(reply) => last_error = reply.status().ok().err().unwrap_or_default(),
                Err(error) => last_error = error.to_string(),
            }
        }
        if received == 0 {
            anyhow::bail!("{}", last_error);
        }
        Ok(total.as_secs_f64() * 1000.0 / received as f64)
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    async fn ping_avg_millis(host: &str, _count: u16) -> anyhow::Result<f64> {
        let _ = Self::resolve_addr(host).await?;
        anyhow::bail!("icmp unsupported on {}", std::env::consts::OS);
    }
}
