use dashmap::DashMap;
use std::time::Duration;
use summer::plugin::Service;
use termini_macros::base_app;
use std::sync::{Arc, OnceLock};
use tokio::sync::Mutex as AsyncMutex;
use crate::util::time_util::now_millis;
use crate::collect::dto::CollectContext;
use summer::tracing::{debug, info, warn};
use sacs::task::{Task, TaskId, TaskSchedule};
use std::sync::atomic::{AtomicI64, Ordering};
use crate::collect::dto::{JobEntry, JobModel};
use crate::base::base_app::{AppContext, BaseApp};
use crate::collect::scheduler::job_base::JobBase;
use crate::service::service_config::ConfigService;
use crate::collect::dto::job_model::PERIOD_SECS_MIN;
use crate::module::common::util::task_hub::TaskGroup;
use crate::model::config::JOB_WORKER_PARALLELISM_DEFAULT;
use crate::model::job_indicator_value::JobIndicatorValue;
use sacs::scheduler::{CancelOpts, GarbageCollector, Scheduler, SchedulerBuilder, TaskScheduler, WorkerParallelism, WorkerType};

const HEARTBEAT_SECS: u64 = 300;
const BOOT_GAP_MIN_MS: i64 = 30 * 60 * 1000;

pub struct JobTargetGroups {
    lock: AsyncMutex<()>,
    groups: DashMap<u64, JobEntry>,
}

type SharedJobTargetGroups = Arc<JobTargetGroups>;

fn create_jobs() -> Arc<DashMap<String, SharedJobTargetGroups>> {
    Arc::new(DashMap::new())
}

fn create_boot_gap() -> Arc<AtomicI64> {
    Arc::new(AtomicI64::new(0))
}

fn create_job_target_groups() -> SharedJobTargetGroups {
    Arc::new(JobTargetGroups { lock: AsyncMutex::new(()), groups: DashMap::new() })
}

fn create_scheduler_cell() -> Arc<OnceLock<Scheduler>> {
    Arc::new(OnceLock::new())
}

#[base_app(sort = 247)]
#[derive(Clone, Service)]
pub struct JobScheduler {
    #[inject(component)]
    job_base: JobBase,

    #[inject(component)]
    config_service: ConfigService,

    #[inject(func = create_scheduler_cell())]
    scheduler: Arc<OnceLock<Scheduler>>,

    #[inject(func = create_jobs())]
    jobs: Arc<DashMap<String, SharedJobTargetGroups>>,

    #[inject(func = create_boot_gap())]
    boot_gap: Arc<AtomicI64>,
}

#[async_trait::async_trait]
impl BaseApp for JobScheduler {
    fn name(&self) -> &'static str {
        "JobScheduler"
    }

    async fn start(&self, _ctx: &AppContext) -> anyhow::Result<()> {
        let parallelism = match self.config_service.select_config().await {
            Some(config) => config.resolve_job_worker_parallelism(),
            None => JOB_WORKER_PARALLELISM_DEFAULT as usize,
        };
        let scheduler = SchedulerBuilder::new().worker_type(WorkerType::CurrentRuntime).parallelism(WorkerParallelism::Limited(parallelism)).garbage_collector(GarbageCollector::Immediate).build();
        let _ = self.scheduler.set(scheduler);
        self.load_boot_gap().await;
        self.watch_heartbeat();
        info!("[collect] started with parallelism={}", parallelism);
        Ok(())
    }

    async fn on_exit(&self, _ctx: &AppContext) -> anyhow::Result<()> {
        let targets: Vec<SharedJobTargetGroups> = self.jobs.iter().map(|entry| entry.value().clone()).collect();
        self.jobs.clear();
        for target in targets {
            let periods: Vec<u64> = target.groups.iter().map(|entry| *entry.key()).collect();
            for period_secs in periods {
                if let Some((_, entry)) = target.groups.remove(&period_secs) {
                    self.unschedule(entry.task_id).await;
                }
            }
        }
        Ok(())
    }
}

impl JobScheduler {
    /// 上次心跳到现在这段时间 app 没跑，采集就是断的；必须在第一次打点之前读。
    async fn load_boot_gap(&self) {
        let Some(config) = self.config_service.select_config().await else {
            return;
        };
        if config.job_heartbeat_at <= 0 {
            return;
        }
        let gap = now_millis() - config.job_heartbeat_at;
        if gap < BOOT_GAP_MIN_MS {
            return;
        }
        self.boot_gap.store(gap, Ordering::Release);
        info!("[collect] 采集中断 {}ms", gap);
    }

    fn watch_heartbeat(&self) {
        let config_service = self.config_service.clone();
        TaskGroup::global().spawn("job-heartbeat", async move {
            let mut ticker = tokio::time::interval(Duration::from_secs(HEARTBEAT_SECS));
            ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            loop {
                ticker.tick().await;
                if let Err(error) = config_service.touch_job_heartbeat().await {
                    warn!("[collect] 写采集心跳失败: {:#}", error);
                }
            }
        });
    }

    /// 取一次就清零，中断横幅每次启动只弹一次。
    pub fn take_boot_gap(&self) -> i64 {
        self.boot_gap.swap(0, Ordering::AcqRel)
    }

    pub async fn add_job(&self, context: CollectContext, period_secs: u64) -> bool {
        let job_id = context.job_id();
        if job_id.trim().is_empty() {
            warn!("[collect] add_job rejected: empty job id");
            return false;
        }
        let target_id = JobModel::target_id(&context);
        let target = self.jobs.entry(target_id.clone()).or_insert_with(create_job_target_groups).value().clone();
        let _guard = target.lock.lock().await;
        self.detach(&target, &target_id, &job_id).await;

        if let Some(members) = target.groups.get(&period_secs).map(|entry| entry.model.members.clone()) {
            members.write().push(context);
            debug!("[collect] add_job {} joined {} period={}", job_id, target_id, period_secs);
            return true;
        }

        let model = JobModel::new(context, period_secs);
        target.groups.insert(period_secs, JobEntry { model: model.clone(), task_id: None });
        if !self.schedule(&target, &model).await {
            target.groups.remove(&period_secs);
            return false;
        }
        true
    }

    pub async fn remove_job(&self, context: &CollectContext, job_id: &str) {
        let target_id = JobModel::target_id(context);
        let Some(target) = self.jobs.get(&target_id).map(|entry| entry.value().clone()) else {
            return;
        };
        let _guard = target.lock.lock().await;
        self.detach(&target, &target_id, job_id).await;
    }

    async fn detach(&self, target: &SharedJobTargetGroups, target_id: &str, job_id: &str) {
        let hits: Vec<u64> = target.groups.iter().filter(|entry| entry.value().model.members.read().iter().any(|item| item.job_id() == job_id)).map(|entry| *entry.key()).collect();
        for period_secs in hits {
            let Some(members) = target.groups.get(&period_secs).map(|entry| entry.model.members.clone()) else {
                continue;
            };
            let empty = {
                let mut members = members.write();
                members.retain(|item| item.job_id() != job_id);
                members.is_empty()
            };
            debug!("[collect] remove_job {} from {} period={}", job_id, target_id, period_secs);
            if !empty {
                continue;
            }
            if let Some((_, entry)) = target.groups.remove(&period_secs) {
                self.unschedule(entry.task_id).await;
                debug!("[collect] {} period={} removed, no member left", target_id, period_secs);
            }
        }
    }

    pub async fn run_once(&self, context: &CollectContext) -> anyhow::Result<Vec<JobIndicatorValue>> {
        self.job_base.run_once(context, true).await
    }

    async fn schedule(&self, target: &SharedJobTargetGroups, model: &JobModel) -> bool {
        let Some(scheduler) = self.scheduler.get() else {
            warn!("[collect] schedule {} period={} failed: scheduler not started", model.id, model.period_secs);
            return false;
        };
        let period = Duration::from_secs(model.period_secs.max(PERIOD_SECS_MIN));

        let members = model.members.clone();
        let job_base = self.job_base.clone();
        let task = Task::new(TaskSchedule::Interval(period), move |_| {
            let members = members.clone();
            let job_base = job_base.clone();
            Box::pin(async move {
                job_base.execute_group(&members).await;
            })
        });

        let task_id = task.id();
        if let Err(error) = scheduler.add(task).await {
            warn!("[collect] schedule {} period={} failed: {}", model.id, model.period_secs, error);
            return false;
        }
        if let Some(mut entry) = target.groups.get_mut(&model.period_secs) {
            entry.task_id = Some(task_id);
        }
        debug!("[collect] schedule {} period={}s", model.id, model.period_secs);
        true
    }

    async fn unschedule(&self, task_id: Option<TaskId>) {
        let (Some(task_id), Some(scheduler)) = (task_id, self.scheduler.get()) else {
            return;
        };
        if let Err(error) = scheduler.cancel(task_id.clone(), CancelOpts::Kill).await {
            debug!("[collect] cancel task {} skipped: {}", task_id, error);
        }
    }
}
