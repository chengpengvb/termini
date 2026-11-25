use std::sync::Arc;
use dashmap::DashMap;
use rig::tool::DynamicTool;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use std::collections::HashSet;
use summer::plugin::LazyComponent;
use summer::tracing::{info, warn};
use crate::base::base_app::BaseApp;
use crate::module::ai::shared::ToolCallDonePush;
use crate::module::ai::tools::base::ToolBaseCollect;
use crate::module::ai::tools::tool_base::ToolDomain;
use rig::tool::server::{ToolServer, ToolServerHandle};
use crate::module::ai::shared::ChatSessionCacheManager;
use crate::module::ai::mcp::mcp_client_pool::McpClientPool;

const SUMMARY_MAX_CHARS: usize = 80;

#[derive(Clone, Debug)]
pub struct DeferredTool {
    pub name: String,
    pub summary: String,
    pub description: String,
}

impl DeferredTool {
    fn new(name: String, description: String) -> Self {
        let end = match description.find(['。', '.', '\n']) {
            Some(index) => index,
            None => description.len(),
        };
        let first = description[..end].trim();
        let summary: String = first.chars().take(SUMMARY_MAX_CHARS).collect();
        Self { name, summary, description }
    }
}

#[derive(Clone)]
pub struct ToolCatalog {
    pub always: Vec<String>,
    pub deferred: Vec<DeferredTool>,
}

fn create_activated() -> Arc<DashMap<String, HashSet<String>>> {
    Arc::new(DashMap::new())
}

fn create_catalogs() -> Arc<DashMap<String, Arc<ToolCatalog>>> {
    Arc::new(DashMap::new())
}

fn create_handles() -> Arc<DashMap<String, ToolServerHandle>> {
    Arc::new(DashMap::new())
}

fn create_name_sets() -> Arc<DashMap<String, HashSet<String>>> {
    Arc::new(DashMap::new())
}

#[base_app(sort = 759)]
#[derive(Clone, Service)]
pub struct ToolActivationStore {
    mcp_client_pool: LazyComponent<McpClientPool>,

    #[inject(func = create_activated())]
    activated: Arc<DashMap<String, HashSet<String>>>,

    #[inject(func = create_catalogs())]
    catalogs: Arc<DashMap<String, Arc<ToolCatalog>>>,

    #[inject(func = create_handles())]
    handles: Arc<DashMap<String, ToolServerHandle>>,

    #[inject(func = create_name_sets())]
    handle_names: Arc<DashMap<String, HashSet<String>>>,

    #[inject(func = create_name_sets())]
    mcp_names: Arc<DashMap<String, HashSet<String>>>,

    #[inject(func = create_name_sets())]
    builtin_names: Arc<DashMap<String, HashSet<String>>>,

    tool_call_done_push: LazyComponent<ToolCallDonePush>,

    #[inject(component)]
    chat_session_cache_manager: ChatSessionCacheManager,
}

#[async_trait]
impl BaseApp for ToolActivationStore {
    fn name(&self) -> &'static str {
        "ToolActivationStore"
    }
}

impl ToolActivationStore {
    pub fn catalog(&self, req_id: &str) -> Option<Arc<ToolCatalog>> {
        if let Some(found) = self.catalogs.get(req_id) {
            return Some(found.value().clone());
        }
        let entry = self.chat_session_cache_manager.get(req_id)?;
        let mut always: Vec<String> = Vec::new();
        let mut deferred: Vec<DeferredTool> = Vec::new();
        for tool in entry.tools.iter().filter(|tool| tool.domains().contains(&ToolDomain::Main)) {
            let rig_tool = tool.as_rig_tool(None);
            let definition = rig_tool.definition();
            if tool.deferred() {
                deferred.push(DeferredTool::new(definition.name, definition.description));
            } else {
                always.push(definition.name);
            }
        }
        drop(entry);
        let mcp_client_pool = match self.mcp_client_pool.get() {
            Ok(pool) => pool,
            Err(error) => {
                warn!("[ai] McpClientPool 不可用, 本轮不构建工具目录 req_id={} error={}", req_id, error);
                return None;
            }
        };
        for tool in mcp_client_pool.collect_rig_tools() {
            if always.iter().any(|name| name == tool.name()) {
                continue;
            }
            let definition = tool.definition();
            deferred.push(DeferredTool::new(definition.name, definition.description));
        }
        let catalog = Arc::new(ToolCatalog { always, deferred });
        self.catalogs.insert(req_id.to_string(), catalog.clone());
        Some(catalog)
    }

    pub fn handle(&self, req_id: &str) -> Option<ToolServerHandle> {
        self.handles.get(req_id).map(|item| item.clone())
    }

    pub fn handle_names(&self, req_id: &str) -> Option<HashSet<String>> {
        self.handle_names.get(req_id).map(|names| names.clone())
    }

    pub fn bind_handle(&self, req_id: &str, tools: Vec<DynamicTool>, builtin_names: HashSet<String>) -> ToolServerHandle {
        if let Some(existing) = self.handles.get(req_id) {
            return existing.clone();
        }
        let mut server = ToolServer::new();
        let mut names = HashSet::new();
        let mut mcp = HashSet::new();
        for tool in tools {
            let name = tool.name().to_string();
            if !builtin_names.contains(&name) {
                mcp.insert(name.clone());
            }
            names.insert(name);
            server = server.dynamic_tool(tool);
        }
        let handle = server.run();
        self.handles.insert(req_id.to_string(), handle.clone());
        self.handle_names.insert(req_id.to_string(), names);
        self.mcp_names.insert(req_id.to_string(), mcp);
        self.builtin_names.insert(req_id.to_string(), builtin_names);
        handle
    }

    pub async fn forget_catalog(&self, req_id: &str) {
        self.sync_handle(req_id).await;
        self.catalogs.remove(req_id);
    }

    pub fn forget_req(&self, req_id: &str) {
        self.catalogs.remove(req_id);
        self.handles.remove(req_id);
        self.handle_names.remove(req_id);
        self.mcp_names.remove(req_id);
        self.builtin_names.remove(req_id);
    }

    pub fn activated(&self, chat_id: &str) -> HashSet<String> {
        self.activated.get(chat_id).map(|names| names.clone()).unwrap_or_default()
    }

    pub fn activate(&self, chat_id: &str, names: &[String]) {
        let mut set = self.activated.entry(chat_id.to_string()).or_default();
        for name in names {
            set.insert(name.clone());
        }
        info!("[ai] 激活延迟工具 chat_id={} names={:?}", chat_id, names);
    }

    pub fn forget_chat(&self, chat_id: &str) {
        self.activated.remove(chat_id);
    }

    async fn sync_handle(&self, req_id: &str) {
        let Some(handle) = self.handles.get(req_id).map(|item| item.clone()) else {
            return;
        };
        let Some(builtin_names) = self.builtin_names.get(req_id).map(|names| names.clone()) else {
            return;
        };
        let mcp_client_pool = match self.mcp_client_pool.get() {
            Ok(pool) => pool,
            Err(error) => {
                warn!("[ai] McpClientPool 不可用, 不同步工具表 req_id={} error={}", req_id, error);
                return;
            }
        };
        let done_push = match self.tool_call_done_push.get() {
            Ok(push) => push,
            Err(error) => {
                warn!("[ai] ToolCallDonePush 不可用, 不同步工具表 req_id={} error={}", req_id, error);
                return;
            }
        };
        let Some(old_mcp) = self.mcp_names.get(req_id).map(|names| names.clone()) else {
            return;
        };
        let new_tools = ToolBaseCollect::collect_mcp(&mcp_client_pool, &builtin_names, &done_push);
        let mut new_mcp = HashSet::new();
        for tool in new_tools {
            new_mcp.insert(tool.name().to_string());
            handle.add_dynamic_tool(tool).await;
        }
        for name in old_mcp.difference(&new_mcp) {
            handle.remove_tool(name).await;
        }
        let mut names = builtin_names;
        names.extend(new_mcp.iter().cloned());
        self.handle_names.insert(req_id.to_string(), names);
        self.mcp_names.insert(req_id.to_string(), new_mcp);
    }
}
