use std::sync::Arc;
use parking_lot::RwLock;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use std::collections::HashMap;
use summer::tracing::{info, warn};
use crate::base::base_app::BaseApp;
use crate::collect::dto::CollectContext;
use crate::collect::collector::Collector;
use crate::model::job_indicator_value::JobIndicatorValue;
use crate::service::service_log_throttle::LogThrottleService;
use crate::model::dto::job::indicator_arg_template::IndicatorArgTemplate;

fn create_collectors() -> Arc<RwLock<HashMap<i16, Arc<dyn Collector>>>> {
    Arc::new(RwLock::new(HashMap::new()))
}

#[base_app(sort = 248)]
#[derive(Clone, Service)]
pub struct AgentCollector {
    #[inject(component)]
    log_throttle: LogThrottleService,

    #[inject(func = create_collectors())]
    collectors: Arc<RwLock<HashMap<i16, Arc<dyn Collector>>>>,
}

#[async_trait]
impl BaseApp for AgentCollector {
    fn name(&self) -> &'static str {
        "AgentCollector"
    }
}

impl AgentCollector {
    pub fn register(&self, collector: Arc<dyn Collector>) {
        let name = collector.get_name();
        let protocol = collector.get_protocol();
        self.collectors.write().insert(protocol, collector);
        info!("[collect] 注册 collector [{}] protocol={}.", name, protocol);
    }

    pub fn list_arg_template(&self) -> Vec<IndicatorArgTemplate> {
        let mut list: Vec<IndicatorArgTemplate> = self.collectors.read().values().map(|collector| collector.get_arg_template()).collect();
        list.sort_by_key(|template| template.protocol);
        list
    }

    pub async fn collection(&self, ctx: &CollectContext) -> anyhow::Result<Vec<JobIndicatorValue>> {
        self.resolve(ctx)?.collection(ctx).await
    }

    pub async fn collection_manual(&self, ctx: &CollectContext) -> anyhow::Result<Vec<JobIndicatorValue>> {
        self.resolve(ctx)?.collection_manual(ctx).await
    }

    fn resolve(&self, ctx: &CollectContext) -> anyhow::Result<Arc<dyn Collector>> {
        let protocol = ctx.protocol();
        let collector = self.collectors.read().get(&protocol).cloned();
        let Some(collector) = collector else {
            let text = format!("protocol={}", protocol);
            if self.log_throttle.allow(format!("collect.agent.protocol.{}", ctx.job_id()), &text) {
                warn!("[collect] 未找到对应 protocol 的 collector protocol={} job_id={}", protocol, ctx.job_id());
            }
            return Err(anyhow::anyhow!("no collector for protocol {}", protocol));
        };
        Ok(collector)
    }
}
