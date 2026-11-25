use async_trait::async_trait;
use crate::collect::dto::CollectContext;
use crate::model::job_indicator_value::JobIndicatorValue;
use crate::model::dto::job::indicator_arg_template::IndicatorArgTemplate;

#[async_trait]
pub trait Collector: Send + Sync {
    fn get_name(&self) -> &'static str;

    fn get_protocol(&self) -> i16;

    fn get_arg_template(&self) -> IndicatorArgTemplate;

    async fn collection(&self, ctx: &CollectContext) -> anyhow::Result<Vec<JobIndicatorValue>>;

    async fn collection_manual(&self, ctx: &CollectContext) -> anyhow::Result<Vec<JobIndicatorValue>> {
        self.collection(ctx).await
    }
}
