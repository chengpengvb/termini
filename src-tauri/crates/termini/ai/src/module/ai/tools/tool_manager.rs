use std::sync::Arc;
use std::path::Path;
use dashmap::DashMap;
use parking_lot::RwLock;
use summer::tracing::info;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use crate::base::base_app::BaseApp;
use crate::module::ai::tools::tool_base::ToolBase;
use crate::module::ai::shared::ChatSessionCacheManager;
use tokio::sync::{Mutex as AsyncMutex, OwnedMutexGuard};
use crate::module::ai::tools::common::task::session::task_registry::TaskRegistry;

fn create_tools() -> Arc<RwLock<Vec<Arc<dyn ToolBase>>>> {
    Arc::new(RwLock::new(Vec::new()))
}

fn create_path_locks() -> Arc<DashMap<String, Arc<AsyncMutex<()>>>> {
    Arc::new(DashMap::new())
}

#[base_app(sort = 760)]
#[derive(Clone, Service)]
pub struct ToolManager {
    #[inject(component)]
    task_registry: TaskRegistry,

    #[inject(func = create_tools())]
    tools: Arc<RwLock<Vec<Arc<dyn ToolBase>>>>,

    #[inject(component)]
    chat_session_cache_manager: ChatSessionCacheManager,

    #[inject(func = create_path_locks())]
    path_locks: Arc<DashMap<String, Arc<AsyncMutex<()>>>>,
}

#[async_trait]
impl BaseApp for ToolManager {
    fn name(&self) -> &'static str {
        "ToolManager"
    }
}

impl ToolManager {
    pub fn register(&self, tool: Arc<dyn ToolBase>) {
        if !tool.platform_supported() {
            info!("[ai] 当前平台不支持该工具, 跳过注册 tool={}", tool.tool_id().as_str());
            return;
        }
        self.tools.write().push(tool);
    }

    pub fn tools(&self) -> Vec<Arc<dyn ToolBase>> {
        self.tools.read().clone()
    }

    pub fn on_start(&self, req_id: &str) {
        for tool in self.tools.read().iter() {
            tool.on_start(req_id);
        }
    }

    pub fn on_stop(&self, req_id: &str) {
        for tool in self.tools.read().iter() {
            tool.on_stop(req_id);
        }
        let cancelled = match self.chat_session_cache_manager.get(req_id) {
            Some(entry) => entry.cancel_token.is_cancelled(),
            None => return,
        };
        if cancelled {
            self.task_registry.cancel_by_req_id(req_id);
        }
    }

    pub async fn lock_path(&self, key: &str) -> OwnedMutexGuard<()> {
        let mutex = {
            let entry = self.path_locks.entry(key.to_string()).or_insert_with(|| Arc::new(AsyncMutex::new(())));
            Arc::clone(entry.value())
        };
        mutex.lock_owned().await
    }

    pub fn local_path_key(path: &Path) -> String {
        let text = path.to_string_lossy().replace('\\', "/");
        if cfg!(windows) {
            return text.to_lowercase();
        }
        text
    }
}
