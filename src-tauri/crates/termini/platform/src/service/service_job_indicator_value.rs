use std::sync::Arc;
use dashmap::DashMap;
use parking_lot::Mutex;
use summer::plugin::Service;
use std::collections::HashSet;
use std::collections::VecDeque;
use crate::util::id_factory::uuid;
use crate::util::time_util::now_millis;
use crate::web::ws::ws_manager::WsManager;
use crate::model::resp::resp_model::ResultCode;
use crate::service::service_config::ConfigService;
use crate::model::job_indicator_value::JobIndicatorValue;
use crate::model::config::JOB_INDICATOR_VALUE_CACHE_SIZE_DEFAULT;
use crate::model::resp::event::resp_event_job_metric::RespEventJobMetric;
use crate::service::service_job_indicator_alert::JobIndicatorAlertService;
use crate::model::resp::resp_metric_item::{RespMetricItem, RespMetricStatus};

const RECENT_VALUE_LIMIT: usize = 5;

#[derive(Clone, Eq, Hash, PartialEq)]
struct ValueKey {
    job_device_id: String,
    job_indicator_id: String,
}

fn create_value_cache() -> Arc<DashMap<ValueKey, Mutex<VecDeque<JobIndicatorValue>>>> {
    Arc::new(DashMap::new())
}

#[derive(Clone, Service)]
pub struct JobIndicatorValueService {
    #[inject(component)]
    ws_manager: WsManager,

    #[inject(component)]
    config_service: ConfigService,

    #[inject(component)]
    job_indicator_alert_service: JobIndicatorAlertService,

    #[inject(func = create_value_cache())]
    cache: Arc<DashMap<ValueKey, Mutex<VecDeque<JobIndicatorValue>>>>,
}

impl JobIndicatorValueService {
    pub async fn push_all(&self, values: &mut Vec<JobIndicatorValue>) -> anyhow::Result<()> {
        if values.is_empty() {
            return Ok(());
        }
        let limit = self.resolve_cache_size().await;
        for value in values.iter_mut() {
            if value.job_device_id.trim().is_empty() {
                return Err(ResultCode::JobDeviceIdRequired.into());
            }
            if value.job_indicator_id.trim().is_empty() {
                return Err(ResultCode::JobIndicatorIdRequired.into());
            }
            if value.id.trim().is_empty() {
                value.id = uuid();
            }
            value.updated_at = now_millis();
            let key = ValueKey { job_device_id: value.job_device_id.clone(), job_indicator_id: value.job_indicator_id.clone() };
            let entry = self.cache.entry(key).or_insert_with(|| Mutex::new(VecDeque::new()));
            let mut queue = entry.lock();
            queue.push_back(value.clone());
            while queue.len() > limit {
                queue.pop_front();
            }
        }
        Ok(())
    }

    pub async fn notify_metric(&self, job_device_id: &str, job_indicator_id: &str) -> anyhow::Result<()> {
        let metric = self.build_metric(job_device_id, job_indicator_id).await?;
        self.ws_manager.broadcast_all(&RespEventJobMetric::OnChanged(metric).to_resp());
        Ok(())
    }

    pub async fn build_metric(&self, job_device_id: &str, job_indicator_id: &str) -> anyhow::Result<RespMetricItem> {
        let mut values = self.list_by_range(job_device_id, job_indicator_id, 0);
        if values.len() > RECENT_VALUE_LIMIT {
            values.drain(..values.len() - RECENT_VALUE_LIMIT);
        }
        let alert = self.job_indicator_alert_service.find_firing(job_device_id, job_indicator_id).await?;
        let updated_at = values.last().map(|item| item.updated_at);
        let status = Self::metric_status(values.last());
        Ok(RespMetricItem { status, job_device_id: job_device_id.to_string(), updated_at, job_indicator_id: job_indicator_id.to_string(), values, alert })
    }

    fn metric_status(last: Option<&JobIndicatorValue>) -> i16 {
        let Some(last) = last else {
            return RespMetricStatus::Pending as i16;
        };
        if last.success {
            RespMetricStatus::Ok as i16
        } else {
            RespMetricStatus::Error as i16
        }
    }

    pub fn list_by_range(&self, job_device_id: &str, job_indicator_id: &str, duration_ms: i64) -> Vec<JobIndicatorValue> {
        let key = ValueKey { job_device_id: job_device_id.to_string(), job_indicator_id: job_indicator_id.to_string() };
        let Some(entry) = self.cache.get(&key) else {
            return Vec::new();
        };
        let queue = entry.lock();
        if duration_ms <= 0 {
            return queue.iter().cloned().collect();
        }
        let since = now_millis().saturating_sub(duration_ms);
        queue.iter().filter(|item| item.updated_at >= since).cloned().collect()
    }

    pub fn find_by_id(&self, id: &str) -> Option<JobIndicatorValue> {
        for entry in self.cache.iter() {
            if let Some(found) = entry.value().lock().iter().find(|item| item.id.as_str() == id) {
                return Some(found.clone());
            }
        }
        None
    }

    pub fn delete_by_indicator(&self, job_device_id: &str, job_indicator_id: &str) -> u64 {
        let key = ValueKey { job_device_id: job_device_id.to_string(), job_indicator_id: job_indicator_id.to_string() };
        if self.cache.remove(&key).is_none() {
            return 0;
        }
        1
    }

    pub fn delete_by_job_device_ids(&self, job_device_ids: Vec<String>) -> u64 {
        if job_device_ids.is_empty() {
            return 0;
        }
        let before = self.cache.len();
        self.cache.retain(|key, _| !job_device_ids.iter().any(|id| key.job_device_id.as_str() == id.as_str()));
        before.saturating_sub(self.cache.len()) as u64
    }

    pub fn referenced_job_device_ids(&self) -> HashSet<String> {
        self.cache.iter().map(|entry| entry.key().job_device_id.clone()).collect()
    }

    async fn resolve_cache_size(&self) -> usize {
        let size = self.config_service.select_config().await.map(|config| config.job_indicator_value_cache_size).unwrap_or(JOB_INDICATOR_VALUE_CACHE_SIZE_DEFAULT);
        if size <= 0 {
            JOB_INDICATOR_VALUE_CACHE_SIZE_DEFAULT as usize
        } else {
            size as usize
        }
    }
}
