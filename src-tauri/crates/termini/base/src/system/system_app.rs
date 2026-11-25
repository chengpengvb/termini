use std::sync::Arc;
use summer::component;
use parking_lot::RwLock;
use std::sync::OnceLock;
use std::time::Duration;
use summer::tracing::info;
use futures::future::join_all;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use crate::base::base_app::{AppContext, BaseApp};
use termini_core::common::util::task_hub::TaskGroup;

static GLOBAL: OnceLock<SystemApp> = OnceLock::new();

pub fn set_global(app: SystemApp) {
    let _ = GLOBAL.set(app);
}

pub fn try_global() -> Option<SystemApp> {
    GLOBAL.get().cloned()
}

#[derive(Clone, Default)]
pub struct SystemApp {
    exited: Arc<AtomicBool>,
    apps: Arc<RwLock<BTreeMap<i16, BTreeMap<i16, Vec<Arc<dyn BaseApp>>>>>>,
}

impl SystemApp {
    pub fn register(&self, app: Arc<dyn BaseApp>, sort: i16) {
        let phase = app.phase();
        let mut apps = self.apps.write();
        apps.entry(sort).or_default().entry(phase).or_default().push(app);
    }

    fn collect_core_by_phase(&self) -> BTreeMap<i16, Vec<Arc<dyn BaseApp>>> {
        let apps = self.apps.read();
        let mut map: BTreeMap<i16, Vec<Arc<dyn BaseApp>>> = BTreeMap::new();
        for inner in apps.values() {
            for (phase, list) in inner {
                for app in list {
                    if app.is_core() {
                        map.entry(*phase).or_default().push(app.clone());
                    }
                }
            }
        }
        map
    }

    fn collect_non_core(&self) -> Vec<Arc<dyn BaseApp>> {
        let apps = self.apps.read();
        apps.values().flat_map(|inner| inner.values().flatten()).filter(|app| !app.is_core()).cloned().collect()
    }

    async fn start_one(app: &Arc<dyn BaseApp>, ctx: &AppContext) {
        if let Err(e) = app.start(ctx).await {
            eprintln!("{} start failed: {e:?}", app.name());
        }
    }

    async fn start_core(&self, ctx: &AppContext) {
        let by_phase = self.collect_core_by_phase();
        for (_phase, group) in by_phase {
            join_all(group.iter().map(|app| Self::start_one(app, ctx))).await;
        }
    }

    async fn start_non_core(&self, ctx: &AppContext) {
        let group = self.collect_non_core();
        join_all(group.iter().map(|app| Self::start_one(app, ctx))).await;
    }

    fn sort_apps(&self) -> Vec<Arc<dyn BaseApp>> {
        let apps = self.apps.read();
        apps.values().flat_map(|inner| inner.values().flatten()).cloned().collect()
    }

    pub async fn on_complete(&self, ctx: &AppContext, ready: &Arc<AtomicBool>) {
        self.start_core(ctx).await;
        ready.store(true, Ordering::Release);
        self.start_non_core(ctx).await;
    }

    pub fn exited(&self) -> bool {
        self.exited.load(Ordering::SeqCst)
    }

    pub async fn on_exit(&self, ctx: &AppContext) {
        if self.exited.swap(true, Ordering::SeqCst) {
            return;
        }
        info!("[system] 开始退出收尾, 按启动相反顺序调用 on_exit.");
        TaskGroup::global().abort();
        let mut sorted_apps = self.sort_apps();
        sorted_apps.reverse();
        for app in sorted_apps {
            if let Err(e) = app.on_exit(ctx).await {
                eprintln!("{} on_exit failed: {e:?}", app.name());
            }
        }
        TaskGroup::global().join_os(Duration::from_secs(2)).await;
        info!("[system] 退出收尾完成.");
    }
}

#[component]
fn create_system_app() -> SystemApp {
    SystemApp::default()
}
