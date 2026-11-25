use std::sync::Arc;
use parking_lot::RwLock;
use summer::tracing::error;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use crate::base::base_app::BaseApp;
use crate::collect::dto::CollectContext;
use crate::collect::callback::JobCallBack;
use crate::collect::collector::AgentCollector;
use crate::collect::callback::DefaultJobCallBack;
use crate::model::job_indicator_value::JobIndicatorValue;
use crate::service::service_log_throttle::LogThrottleService;

#[base_app(sort = 246)]
#[derive(Clone, Service)]
pub struct JobBase {
    #[inject(component)]
    collector: AgentCollector,

    #[inject(component)]
    call_back: DefaultJobCallBack,

    #[inject(component)]
    log_throttle: LogThrottleService,
}

#[async_trait]
impl BaseApp for JobBase {
    fn name(&self) -> &'static str {
        "JobBase"
    }
}

impl JobBase {
    pub async fn execute_group(&self, members: &Arc<RwLock<Vec<CollectContext>>>) {
        let snapshot: Vec<CollectContext> = members.read().clone();
        for ctx in &snapshot {
            if !ctx.enabled() {
                continue;
            }
            let _ = self.run_once(ctx, false).await;
        }
    }

    pub async fn run_once(&self, ctx: &CollectContext, manual: bool) -> anyhow::Result<Vec<JobIndicatorValue>> {
        let result = self.collect_once(ctx, manual).await;
        let echo = match &result {
            Ok(values) => Ok(values.clone()),
            Err(error) => Err(anyhow::anyhow!("{}", error)),
        };
        self.call_back.do_call_back(ctx, result).await;
        echo
    }

    async fn collect_once(&self, ctx: &CollectContext, manual: bool) -> anyhow::Result<Vec<JobIndicatorValue>> {
        let result = match manual {
            false => self.collector.collection(ctx).await,
            true => self.collector.collection_manual(ctx).await,
        };
        match result {
            Ok(values) => Ok(values),
            Err(error) => {
                let text = error.to_string();
                if self.log_throttle.allow(format!("collect.job_base.{}", ctx.job_id()), &text) {
                    error!("[collect] job_id={} collection error: {}", ctx.job_id(), text);
                }
                Err(error)
            }
        }
    }
}
