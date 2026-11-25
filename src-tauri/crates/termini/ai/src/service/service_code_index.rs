use anyhow::Result;
use std::sync::Arc;
use dashmap::DashMap;
use std::path::PathBuf;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use summer::tracing::{info, warn};
use std::time::{Duration, Instant};
use parking_lot::Mutex as SyncMutex;
use crate::util::time_util::now_millis;
use crate::module::ai::code_index::CodeIndex;
use crate::base::base_app::{AppContext, BaseApp};
use crate::service::service_config::ConfigService;
use crate::module::common::util::task_hub::TaskGroup;
use crate::module::ai::code_index::walker::probe_index_size;
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use crate::module::ai::code_index::index_guard::{normalize, normalize_index_dirs};
use crate::module::ai::code_index::index_builder::{index_summary, CodeIndexBuilder, IndexStage};
use notify::{Config as NotifyConfig, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

const BUILD_MAX_RETRIES: usize = 3;
const TICK: Duration = Duration::from_secs(5);
const MAX_INDEX_BYTES: u64 = 300 * 1024 * 1024;
const WARMUP: Duration = Duration::from_secs(15);
const DEBOUNCE: Duration = Duration::from_secs(20);
const RETRY_BACKOFF: Duration = Duration::from_secs(45);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i16)]
pub enum IndexDirState {
    Idle = 0,
    Pending = 1,
    Building = 2,
    Ready = 3,
    Failed = 4,
}

impl IndexDirState {
    pub fn as_i16(self) -> i16 {
        self as i16
    }
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexDirStatus {
    pub dir: String,
    pub state: i16,
    pub chunk_count: usize,
    pub built_at: i64,
    pub stage: Option<String>,
    pub error: Option<String>,
}

struct DirRuntime {
    retries: usize,
    state: IndexDirState,
    due_at: Option<Instant>,
    stage: Option<String>,
    error: Option<String>,
}

impl Default for DirRuntime {
    fn default() -> Self {
        Self { state: IndexDirState::Idle, due_at: None, stage: None, error: None, retries: 0 }
    }
}

fn create_runtime() -> Arc<DashMap<String, Arc<SyncMutex<DirRuntime>>>> {
    Arc::new(DashMap::new())
}

fn create_watchers() -> Arc<SyncMutex<Vec<RecommendedWatcher>>> {
    Arc::new(SyncMutex::new(Vec::new()))
}

#[base_app(sort = 780)]
#[derive(Clone, Service)]
pub struct CodeIndexService {
    #[inject(component)]
    code_index: CodeIndex,

    #[inject(component)]
    config_service: ConfigService,

    #[inject(component)]
    code_index_builder: CodeIndexBuilder,

    #[inject(func = create_watchers())]
    watchers: Arc<SyncMutex<Vec<RecommendedWatcher>>>,

    #[inject(func = create_runtime())]
    runtime: Arc<DashMap<String, Arc<SyncMutex<DirRuntime>>>>,
}

#[async_trait]
impl BaseApp for CodeIndexService {
    fn name(&self) -> &'static str {
        "CodeIndexService"
    }

    fn is_core(&self) -> bool {
        false
    }

    async fn start(&self, _ctx: &AppContext) -> Result<()> {
        let service = self.clone();
        TaskGroup::global().spawn("code-index-loop", async move {
            tokio::time::sleep(WARMUP).await;
            if let Err(error) = service.reload().await {
                warn!("[ai] 启动装载索引目录失败: {error:#}");
            }
            service.run_loop().await;
        });
        Ok(())
    }

    async fn on_exit(&self, _ctx: &AppContext) -> Result<()> {
        self.watchers.lock().clear();
        Ok(())
    }
}

impl CodeIndexService {
    pub async fn reload(&self) -> Result<()> {
        let dirs = self.config_service.select_index_dirs().await;
        let (valid, rejected) = normalize_index_dirs(&dirs);
        for reason in &rejected {
            warn!("[ai] 跳过无效索引目录 {}", reason);
        }
        let keys: Vec<String> = valid.iter().map(|dir| normalize(&PathBuf::from(dir))).collect();
        self.runtime.retain(|key, _| keys.contains(key));

        let (tx, mut rx) = unbounded_channel::<String>();
        let mut watchers = Vec::new();
        for dir in &valid {
            let key = normalize(&PathBuf::from(dir));
            self.runtime.entry(key.clone()).or_insert_with(|| Arc::new(SyncMutex::new(DirRuntime::default())));
            match Self::spawn_watcher(dir, key.clone(), tx.clone()) {
                Ok(watcher) => watchers.push(watcher),
                Err(error) => warn!("[ai] 监听目录失败 dir={} error={:#}", dir, error),
            }
        }
        *self.watchers.lock() = watchers;
        drop(tx);

        let service = self.clone();
        TaskGroup::global().spawn("code-index-dirty", async move {
            while let Some(key) = rx.recv().await {
                service.mark_dirty(&key);
            }
        });
        info!("[ai] 已装载 {} 个索引目录", valid.len());
        self.schedule_catch_up(keys);
        Ok(())
    }

    fn schedule_catch_up(&self, keys: Vec<String>) {
        if keys.is_empty() {
            return;
        }
        let service = self.clone();
        TaskGroup::global().spawn("code-index-catch-up", async move {
            for key in keys {
                if service.is_building(&key) {
                    continue;
                }
                service.build_one(&key).await;
            }
        });
    }

    fn is_building(&self, key: &str) -> bool {
        let Some(entry) = self.runtime.get(key) else {
            return false;
        };
        let runtime = entry.value().clone();
        drop(entry);
        let state = runtime.lock().state;
        matches!(state, IndexDirState::Building)
    }

    fn spawn_watcher(dir: &str, key: String, tx: UnboundedSender<String>) -> Result<RecommendedWatcher> {
        let mut watcher = RecommendedWatcher::new(
            move |result: notify::Result<Event>| {
                let Ok(event) = result else {
                    return;
                };
                if !matches!(event.kind, EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)) {
                    return;
                }
                let _ = tx.send(key.clone());
            },
            NotifyConfig::default(),
        )?;
        watcher.watch(&PathBuf::from(dir), RecursiveMode::Recursive)?;
        Ok(watcher)
    }

    fn mark_dirty(&self, key: &str) {
        let Some(entry) = self.runtime.get(key) else {
            return;
        };
        self.code_index.invalidate(&PathBuf::from(key));
        let mut runtime = entry.lock();
        let due = Instant::now() + DEBOUNCE;
        if matches!(runtime.state, IndexDirState::Building) {
            runtime.due_at = Some(due);
            return;
        }
        runtime.due_at = Some(due);
        runtime.state = IndexDirState::Pending;
    }

    async fn run_loop(&self) {
        loop {
            tokio::time::sleep(TICK).await;
            let due: Vec<String> = self
                .runtime
                .iter()
                .filter(|entry| {
                    let runtime = entry.value().lock();
                    match (runtime.state, runtime.due_at) {
                        (IndexDirState::Pending, Some(at)) => Instant::now() >= at,
                        _ => false,
                    }
                })
                .map(|entry| entry.key().clone())
                .collect();
            for key in due {
                self.build_one(&key).await;
            }
        }
    }

    async fn build_one(&self, key: &str) {
        let Some(entry) = self.runtime.get(key) else {
            return;
        };
        let runtime = entry.value().clone();
        drop(entry);
        {
            let mut state = runtime.lock();
            state.state = IndexDirState::Building;
            state.due_at = None;
            state.error = None;
        }
        let root = PathBuf::from(key);
        let stage_runtime = runtime.clone();
        let sink = move |stage: IndexStage| {
            stage_runtime.lock().stage = Some(describe_stage(&stage));
        };
        let result = self.code_index_builder.ensure_index(&root, false, &sink).await;
        let mut state = runtime.lock();
        match result {
            Ok(count) => {
                state.state = IndexDirState::Ready;
                state.stage = None;
                state.retries = 0;
                info!("[ai] 后台索引完成 dir={} 新编码={}", key, count);
            }
            Err(error) => {
                state.stage = None;
                state.error = Some(format!("{:#}", error));
                warn!("[ai] 后台索引失败 dir={} error={:#}", key, error);
                if state.retries < BUILD_MAX_RETRIES {
                    state.retries += 1;
                    state.state = IndexDirState::Pending;
                    state.due_at = Some(Instant::now() + RETRY_BACKOFF);
                    return;
                }
                state.state = IndexDirState::Failed;
            }
        }
        if state.due_at.is_some() {
            state.state = IndexDirState::Pending;
        }
    }

    pub async fn status(&self) -> Vec<IndexDirStatus> {
        let dirs = self.config_service.select_index_dirs().await;
        let (valid, _) = normalize_index_dirs(&dirs);
        let mut out = Vec::with_capacity(valid.len());
        for dir in valid {
            let key = normalize(&PathBuf::from(&dir));
            let (state, stage, error) = match self.runtime.get(&key) {
                Some(entry) => {
                    let runtime = entry.value().lock();
                    (runtime.state, runtime.stage.clone(), runtime.error.clone())
                }
                None => (IndexDirState::Idle, None, None),
            };
            let summary = CodeIndexBuilder::index_dir(&PathBuf::from(&dir)).ok().and_then(|path| index_summary(&path));
            let (chunk_count, built_at) = summary.unwrap_or((0, 0));
            out.push(IndexDirStatus { dir, state: state.as_i16(), chunk_count, built_at, stage, error });
        }
        out
    }

    pub async fn screen_dirs(&self, dirs: &[String]) -> (Vec<String>, Vec<String>) {
        let (valid, mut rejected) = normalize_index_dirs(dirs);
        let (rules, include_tests) = self.code_index_builder.ignore_rules().await;
        let mut kept = Vec::with_capacity(valid.len());
        for dir in valid {
            let root = PathBuf::from(&dir);
            let probe_rules = rules.clone();
            let probed = tokio::task::spawn_blocking(move || probe_index_size(&root, &probe_rules, include_tests, MAX_INDEX_BYTES)).await;
            match probed {
                Ok(Ok(probe)) if probe.exceeded => {
                    rejected.push(format!(
                        "{}: source code alone already exceeds {} MB and is still growing; the directory is too large to index as a whole, choose a more specific subdirectory",
                        dir,
                        MAX_INDEX_BYTES / 1024 / 1024
                    ));
                }
                Ok(Ok(probe)) => {
                    info!("[ai] 目录体量 dir={} 文件={} 体积={}MB", dir, probe.files, probe.bytes / 1024 / 1024);
                    kept.push(dir);
                }
                Ok(Err(error)) => rejected.push(format!("{}: failed to probe directory {:#}", dir, error)),
                Err(error) => rejected.push(format!("{}: failed to schedule probe {}", dir, error)),
            }
        }
        (kept, rejected)
    }

    pub async fn rebuild(&self, dir: &str) -> Result<()> {
        let key = normalize(&PathBuf::from(dir));
        if !self.runtime.contains_key(&key) {
            anyhow::bail!("directory is not in the index configuration: {}", dir);
        }
        let service = self.clone();
        TaskGroup::global().spawn("code-index-rebuild", async move {
            service.build_one(&key).await;
        });
        Ok(())
    }
}

fn describe_stage(stage: &IndexStage) -> String {
    match stage {
        IndexStage::Scan { files } => format!("scan:{}", files),
        IndexStage::Chunk { total, reuse, pending } => format!("chunk:{}/{}/{}", total, reuse, pending),
        IndexStage::Embed { done, total } => format!("embed:{}/{}", done, total),
        IndexStage::Save { chunks } => format!("save:{}", chunks),
    }
}

pub fn built_ago_secs(built_at: i64) -> i64 {
    if built_at <= 0 {
        return -1;
    }
    (now_millis() - built_at) / 1000
}
