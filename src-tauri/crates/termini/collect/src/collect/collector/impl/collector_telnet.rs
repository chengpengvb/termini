use std::sync::Arc;
use std::time::Duration;
use summer::tracing::debug;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use crate::collect::dto::CollectContext;
use crate::collect::collector::Collector;
use crate::collect::collector::AgentCollector;
use crate::collect::collector::r#impl::parser;
use crate::base::base_app::{AppContext, BaseApp};
use crate::service::service_config::ConfigService;
use crate::service::service_device::DeviceService;
use crate::model::job_indicator::JobIndicatorProtocol;
use crate::model::config::JOB_EXEC_TIMEOUT_SECS_DEFAULT;
use crate::model::job_indicator_value::JobIndicatorValue;
use crate::service::service_config_telnet::ConfigTelnetService;
use crate::collect::collector::r#impl::pool::telnet_anchor_pool::TelnetAnchorPool;
use crate::module::telnet_shell::telnet_shell_exec_core::resolve_telnet_shell_context;
use crate::model::dto::job::indicator_arg_template::{IndicatorArgTemplate, IndicatorArgType};

pub const TELNET_ARG_ID: &str = "telnet-script";
pub const TELNET_ARG_LABEL_I18N: &str = "job.indicator.arg.telnet.script";

#[base_app(sort = 252)]
#[derive(Clone, Service)]
pub struct CollectorTelnet {
    #[inject(component)]
    device_service: DeviceService,

    #[inject(component)]
    config_service: ConfigService,

    #[inject(component)]
    agent_collector: AgentCollector,

    #[inject(component)]
    telnet_anchor_pool: TelnetAnchorPool,

    #[inject(component)]
    config_telnet_service: ConfigTelnetService,
}

#[async_trait]
impl BaseApp for CollectorTelnet {
    fn name(&self) -> &'static str {
        "CollectorTelnet"
    }

    async fn start(&self, _ctx: &AppContext) -> anyhow::Result<()> {
        self.agent_collector.register(Arc::new(self.clone()));
        Ok(())
    }
}

#[async_trait]
impl Collector for CollectorTelnet {
    fn get_name(&self) -> &'static str {
        "CollectorTelnet"
    }

    fn get_protocol(&self) -> i16 {
        JobIndicatorProtocol::Telnet as i16
    }

    fn get_arg_template(&self) -> IndicatorArgTemplate {
        IndicatorArgTemplate { id: TELNET_ARG_ID.to_string(), arg_type: IndicatorArgType::Shell as i16, label_i18n: TELNET_ARG_LABEL_I18N.to_string(), protocol: self.get_protocol() }
    }

    async fn collection(&self, ctx: &CollectContext) -> anyhow::Result<Vec<JobIndicatorValue>> {
        self.collect(ctx, false).await
    }

    async fn collection_manual(&self, ctx: &CollectContext) -> anyhow::Result<Vec<JobIndicatorValue>> {
        self.collect(ctx, true).await
    }
}

impl CollectorTelnet {
    async fn collect(&self, ctx: &CollectContext, wait: bool) -> anyhow::Result<Vec<JobIndicatorValue>> {
        let Some(script) = ctx.script() else {
            anyhow::bail!("this indicator has no Telnet command configured");
        };
        let device_id = ctx.device_id();
        let context = resolve_telnet_shell_context(&device_id, &self.device_service, &self.config_telnet_service).await.map_err(|error| anyhow::anyhow!("device Telnet config unavailable: {}", error))?;
        let exec_timeout = match self.config_service.select_config().await {
            Some(config) => config.resolve_job_exec_timeout(),
            None => Duration::from_secs(JOB_EXEC_TIMEOUT_SECS_DEFAULT as u64),
        };
        let value = match self.telnet_anchor_pool.exec(&context, script, exec_timeout, wait).await {
            Ok(Some(output)) => parser::build_value(ctx, None, &output),
            Ok(None) => {
                debug!("[telnet] job_id={} skipped, 设备 telnet 并发名额已满", ctx.job_id());
                return Ok(Vec::new());
            }
            Err(error) => ctx.new_value(None, false, Some(format!("could not connect to device or command failed: {}", error))),
        };
        debug!("[telnet] job_id={} device={} success={} value={:?}", ctx.job_id(), device_id, value.success, value.value);
        Ok(vec![value])
    }
}
