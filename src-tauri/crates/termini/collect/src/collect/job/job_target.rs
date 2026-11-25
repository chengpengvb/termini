use std::sync::Arc;
use dashmap::DashMap;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use crate::dao::dao_base::BaseDao;
use crate::base::base_app::BaseApp;
use summer::tracing::{debug, info, warn};
use crate::collect::scheduler::JobScheduler;
use crate::dao::dao_job_device::JobDeviceDao;
use crate::dao::dao_job_indicator::JobIndicatorDao;
use crate::collect::dto::{CollectContext, JobModel};
use crate::model::job_indicator_value::JobIndicatorValue;
use crate::dao::dao_job_device_2_indicator::JobDevice2IndicatorDao;

fn create_contexts() -> Arc<DashMap<String, CollectContext>> {
    Arc::new(DashMap::new())
}

#[base_app(sort = 260)]
#[derive(Clone, Service)]
pub struct JobTargetService {
    #[inject(component)]
    scheduler: JobScheduler,

    #[inject(component)]
    job_device_dao: JobDeviceDao,

    #[inject(component)]
    job_indicator_dao: JobIndicatorDao,

    #[inject(func = create_contexts())]
    contexts: Arc<DashMap<String, CollectContext>>,

    #[inject(component)]
    job_device_2_indicator_dao: JobDevice2IndicatorDao,
}

#[async_trait]
impl BaseApp for JobTargetService {
    fn name(&self) -> &'static str {
        "JobTargetService"
    }

    async fn start(&self, _ctx: &crate::base::base_app::AppContext) -> anyhow::Result<()> {
        self.start_all().await;
        Ok(())
    }
}

impl JobTargetService {
    pub async fn start_all(&self) {
        info!("[collect] init jobs...");
        let mut added = 0usize;
        let contexts = self.build_all_contexts().await;
        let size = contexts.len();
        for context in contexts {
            if self.add_job(context).await {
                added += 1;
            }
        }
        info!("[collect] init job {} size, added {}", size, added);
    }

    pub async fn start_job_device(&self, job_device_id: &str) {
        self.delete_by_job_device_ids(vec![job_device_id.to_string()]).await;
        let contexts = self.build_contexts_for_device(job_device_id).await;
        let size = contexts.len();
        let mut added = 0usize;
        for context in contexts {
            if self.add_job(context).await {
                added += 1;
            }
        }
        info!("[collect] start job_device={} jobs={} added={}", job_device_id, size, added);
    }

    pub async fn delete_by_job_device_ids(&self, job_device_ids: Vec<String>) {
        if job_device_ids.is_empty() {
            return;
        }
        let job_ids = self.list_job_ids_by_job_device_ids(&job_device_ids);
        for job_id in &job_ids {
            self.delete_job(job_id).await;
        }
        info!("[collect] delete_by_job_device_ids devices={} jobs={}", job_device_ids.len(), job_ids.len());
    }

    pub async fn add_job(&self, context: CollectContext) -> bool {
        let job_id = context.job_id();
        let period_secs = JobModel::clamp_period(context.interval_secs());
        if !self.scheduler.add_job(context.clone(), period_secs).await {
            debug!("[collect] add job {} ok=false", job_id);
            return false;
        }
        self.contexts.insert(job_id.clone(), context);
        debug!("[collect] add job {} ok=true", job_id);
        true
    }

    pub async fn delete_job(&self, job_id: &str) {
        let Some((_, context)) = self.contexts.remove(job_id) else {
            return;
        };
        self.scheduler.remove_job(&context, job_id).await;
        info!("[collect] delete job {}", job_id);
    }

    pub async fn run_once(&self, job_id: &str) -> anyhow::Result<Vec<JobIndicatorValue>> {
        let Some(context) = self.get_context(job_id) else {
            anyhow::bail!("this collection job has not started yet; make sure the device and indicator are saved");
        };
        self.scheduler.run_once(&context).await
    }

    pub fn get_context(&self, job_id: &str) -> Option<CollectContext> {
        self.contexts.get(job_id).map(|entry| entry.value().clone())
    }

    /// 没有在跑的采集任务就没有"中断"可言，不给用户看无意义的横幅。
    pub fn take_collect_gap(&self) -> i64 {
        if self.contexts.is_empty() {
            return 0;
        }
        self.scheduler.take_boot_gap()
    }

    fn list_job_ids_by_job_device_ids(&self, job_device_ids: &[String]) -> Vec<String> {
        self.contexts.iter().filter(|entry| job_device_ids.iter().any(|id| entry.value().job_device.read().id.as_str() == id.as_str())).map(|entry| entry.key().clone()).collect()
    }

    async fn build_all_contexts(&self) -> Vec<CollectContext> {
        let devices = match self.job_device_dao.find_all().await {
            Ok(list) => list,
            Err(error) => {
                warn!("[collect] find job_device failed: {}", error);
                return Vec::new();
            }
        };
        let mut contexts = Vec::new();
        for device in devices {
            let job_device_id = device.read().id.clone();
            contexts.extend(self.build_contexts_for_device(&job_device_id).await);
        }
        contexts
    }

    async fn build_contexts_for_device(&self, job_device_id: &str) -> Vec<CollectContext> {
        let Some(job_device) = self.job_device_dao.find_by_id(job_device_id.to_string()).await.ok().flatten() else {
            warn!("[collect] job_device not found: {}", job_device_id);
            return Vec::new();
        };

        let links = match self.job_device_2_indicator_dao.find_all().await {
            Ok(list) => list,
            Err(error) => {
                warn!("[collect] find job_device_2_indicator failed: {}", error);
                return Vec::new();
            }
        };

        let mut contexts = Vec::new();
        for job_device_2_indicator in links {
            if job_device_2_indicator.read().job_device_id.as_str() != job_device_id {
                continue;
            }
            let job_indicator_id = job_device_2_indicator.read().job_indicator_id.clone();
            let Some(indicator) = self.job_indicator_dao.find_by_id(job_indicator_id).await.ok().flatten() else {
                continue;
            };
            contexts.push(CollectContext { job_device: job_device.clone(), job_indicator: indicator, job_device_2_indicator });
        }
        contexts
    }
}
