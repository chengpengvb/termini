use std::sync::Arc;
use parking_lot::RwLock;
use crate::collect::dto::CollectContext;

pub const PERIOD_SECS_MIN: u64 = 60;

#[derive(Clone)]
pub struct JobModel {
    pub id: String,
    pub period_secs: u64,
    pub members: Arc<RwLock<Vec<CollectContext>>>,
}

impl JobModel {
    pub fn new(context: CollectContext, period_secs: u64) -> Self {
        Self { id: Self::target_id(&context), period_secs, members: Arc::new(RwLock::new(vec![context])) }
    }

    pub fn target_id(context: &CollectContext) -> String {
        format!("{}:{}", context.job_device_id(), context.protocol())
    }

    pub fn clamp_period(interval_secs: i64) -> u64 {
        if interval_secs < PERIOD_SECS_MIN as i64 {
            PERIOD_SECS_MIN
        } else {
            interval_secs as u64
        }
    }
}
