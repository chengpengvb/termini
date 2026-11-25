use async_trait::async_trait;
use crate::collect::dto::CollectContext;
use crate::model::job_indicator_value::JobIndicatorValue;

#[async_trait]
pub trait JobCallBack: Send + Sync {
    async fn do_call_back(&self, ctx: &CollectContext, result: anyhow::Result<Vec<JobIndicatorValue>>);
}
