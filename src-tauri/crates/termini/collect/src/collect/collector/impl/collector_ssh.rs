use std::sync::Arc;
use summer::tracing::debug;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use crate::collect::dto::CollectContext;
use crate::collect::collector::Collector;
use summer::plugin::component::ComponentRef;
use crate::collect::collector::AgentCollector;
use crate::collect::collector::r#impl::parser;
use crate::base::base_app::{AppContext, BaseApp};
use crate::service::service_config::ConfigService;
use crate::service::service_device::DeviceService;
use crate::model::job_indicator::JobIndicatorProtocol;
use crate::model::config::JOB_EXEC_TIMEOUT_SECS_DEFAULT;
use crate::model::job_indicator_value::JobIndicatorValue;
use crate::service::service_config_ssh::ConfigSshService;
use crate::module::ssh::client_session_factory::ClientSessionFactory;
use crate::collect::collector::r#impl::pool::ssh_anchor_pool::SshAnchorPool;
use crate::model::dto::job::indicator_arg_template::{IndicatorArgTemplate, IndicatorArgType};
use crate::module::ssh_shell::ssh_shell_exec_core::{resolve_ssh_shell_context, ssh_shell_open_exec_close};

pub const SSH_ARG_ID: &str = "ssh-script";
pub const SSH_ARG_LABEL_I18N: &str = "job.indicator.arg.ssh.script";

#[base_app(sort = 251)]
#[derive(Clone, Service)]
pub struct CollectorSsh {
    #[inject(component)]
    device_service: DeviceService,

    #[inject(component)]
    config_service: ConfigService,

    #[inject(component)]
    agent_collector: AgentCollector,

    #[inject(component)]
    ssh_anchor_pool: SshAnchorPool,

    #[inject(component)]
    config_ssh_service: ConfigSshService,

    ssh_client_session_factory: ComponentRef<ClientSessionFactory>,
}

#[async_trait]
impl BaseApp for CollectorSsh {
    fn name(&self) -> &'static str {
        "CollectorSsh"
    }

    async fn start(&self, _ctx: &AppContext) -> anyhow::Result<()> {
        self.agent_collector.register(Arc::new(self.clone()));
        Ok(())
    }
}

#[async_trait]
impl Collector for CollectorSsh {
    fn get_name(&self) -> &'static str {
        "CollectorSsh"
    }

    fn get_protocol(&self) -> i16 {
        JobIndicatorProtocol::SSH as i16
    }

    fn get_arg_template(&self) -> IndicatorArgTemplate {
        IndicatorArgTemplate { id: SSH_ARG_ID.to_string(), arg_type: IndicatorArgType::Shell as i16, label_i18n: SSH_ARG_LABEL_I18N.to_string(), protocol: self.get_protocol() }
    }

    async fn collection(&self, ctx: &CollectContext) -> anyhow::Result<Vec<JobIndicatorValue>> {
        let Some(command) = ctx.script() else {
            anyhow::bail!("this indicator has no SSH command configured");
        };
        let device_id = ctx.device_id();
        let context = resolve_ssh_shell_context(&device_id, &self.device_service, &self.config_ssh_service).await.map_err(|error| anyhow::anyhow!("device SSH config unavailable: {}", error))?;
        self.ssh_anchor_pool.touch(&context).await;

        let exec_timeout = match self.config_service.select_config().await {
            Some(config) => config.resolve_job_exec_timeout(),
            None => std::time::Duration::from_secs(JOB_EXEC_TIMEOUT_SECS_DEFAULT as u64),
        };
        let value = match ssh_shell_open_exec_close(command, exec_timeout, &context, None, self.ssh_client_session_factory.clone()).await {
            Ok((exit_code, output)) => parser::build_value(ctx, Some(exit_code), &output),
            Err(error) => ctx.new_value(None, false, Some(format!("could not connect to device or command failed: {}", error))),
        };
        debug!("[ssh] job_id={} device={} success={} value={:?}", ctx.job_id(), device_id, value.success, value.value);
        Ok(vec![value])
    }
}
