use std::sync::Arc;
use parking_lot::RwLock;
use summer::tracing::warn;
use rig::tool::DynamicTool;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use crate::dao::dao_base::BaseDao;
use crate::base::base_app::BaseApp;
use std::collections::{HashMap, HashSet};
use crate::model::ai_mcp_server::AiMcpServer;
use crate::model::resp::resp_model::ResultCode;
use crate::dao::dao_ai_mcp_server::AiMcpServerDao;
use crate::module::ai::mcp::wrap::ToolBaseMcpWrap;
use crate::module::ai::shared::ToolConflictManager;
use crate::module::common::util::task_hub::TaskGroup;
use crate::module::ai::shared::ChatSessionCacheManager;
use crate::module::ai::mcp::mcp_server_runtime::McpServerRuntime;
use crate::module::ai::mcp::dto::mcp_tool_snapshot::McpToolSnapshot;
use crate::module::ai::mcp::mcp_command_resolver::McpCommandResolver;
use rmcp::model::{GetPromptRequestParams, GetPromptResult, JsonObject};
use crate::module::ai::mcp::dto::mcp_prompt_snapshot::McpPromptSnapshot;

fn create_runtimes() -> Arc<RwLock<HashMap<String, McpServerRuntime>>> {
    Arc::new(RwLock::new(HashMap::new()))
}

#[base_app(sort = 766)]
#[derive(Clone, Service)]
pub struct McpClientPool {
    #[inject(component)]
    ai_mcp_server_dao: AiMcpServerDao,

    #[inject(component)]
    mcp_command_resolver: McpCommandResolver,

    #[inject(component)]
    tool_conflict_manager: ToolConflictManager,

    #[inject(component)]
    chat_session_cache_manager: ChatSessionCacheManager,

    #[inject(func = create_runtimes())]
    runtimes: Arc<RwLock<HashMap<String, McpServerRuntime>>>,
}

#[async_trait]
impl BaseApp for McpClientPool {
    fn is_core(&self) -> bool {
        false
    }

    fn name(&self) -> &'static str {
        "McpClientPool"
    }

    async fn start(&self, _ctx: &crate::base::base_app::AppContext) -> anyhow::Result<()> {
        let pool = self.clone();
        TaskGroup::global().spawn("mcp-sync", async move {
            pool.sync_from_db().await;
        });
        Ok(())
    }

    async fn on_exit(&self, _ctx: &crate::base::base_app::AppContext) -> anyhow::Result<()> {
        self.runtimes.write().clear();
        Ok(())
    }
}

impl McpClientPool {
    pub async fn sync_from_db(&self) {
        let servers = match self.ai_mcp_server_dao.find_all().await {
            Ok(list) => list,
            Err(error) => {
                warn!("[mcp] MCP sync_from_db find_all failed: {}", error);
                return;
            }
        };
        let enabled: Vec<AiMcpServer> = servers.into_iter().map(|item| item.read().clone()).filter(|item| item.enabled).collect();
        let enabled_ids: HashSet<String> = enabled.iter().map(|item| item.id.clone()).collect();

        {
            let mut guard = self.runtimes.write();
            let stale: Vec<String> = guard.keys().filter(|id| !enabled_ids.contains(*id)).cloned().collect();
            for id in stale {
                guard.remove(&id);
            }
        }

        for server in enabled {
            self.start_server(server).await;
        }
    }

    pub async fn start_server(&self, server: AiMcpServer) {
        if !server.enabled {
            self.stop_server(&server.id);
            return;
        }
        let runtime = match self.mcp_command_resolver.resolve(&server).await {
            Ok(resolved) => McpServerRuntime::start(resolved).await,
            Err(error) => {
                warn!("[mcp] MCP server [{}] resolve launcher failed: {}", server.id, error);
                McpServerRuntime::disconnected(&server, error.to_string())
            }
        };
        self.runtimes.write().insert(runtime.server_id.clone(), runtime);
    }

    pub fn stop_server(&self, id: &str) {
        self.runtimes.write().remove(id);
    }

    pub async fn reconnect(&self, id: &str) -> anyhow::Result<()> {
        let shared = self.ai_mcp_server_dao.find_by_id(id.to_string()).await?.ok_or_else(|| anyhow::Error::from(ResultCode::McpServerNotFound))?;
        let server = shared.read().clone();
        if !server.enabled {
            self.stop_server(id);
            return Err(anyhow::anyhow!("MCP server is disabled"));
        }
        self.start_server(server).await;
        let guard = self.runtimes.read();
        let runtime = guard.get(id).ok_or_else(|| anyhow::anyhow!("MCP server runtime is missing"))?;
        if runtime.connected {
            return Ok(());
        }
        match &runtime.last_error {
            Some(error) => Err(anyhow::anyhow!(error.clone())),
            None => Err(anyhow::anyhow!("MCP server is not connected")),
        }
    }

    pub fn runtime_status(&self, server: &AiMcpServer) -> (bool, i32, Option<String>) {
        let guard = self.runtimes.read();
        match guard.get(&server.id) {
            Some(runtime) => (runtime.connected, runtime.snapshots.len() as i32, runtime.last_error.clone()),
            None => (false, 0, None),
        }
    }

    pub fn list_snapshots(&self, id: &str) -> anyhow::Result<Vec<McpToolSnapshot>> {
        let guard = self.runtimes.read();
        let runtime = guard.get(id).ok_or_else(|| anyhow::anyhow!("MCP server is not connected"))?;
        if !runtime.connected {
            match &runtime.last_error {
                Some(error) => return Err(anyhow::anyhow!(error.clone())),
                None => return Err(anyhow::anyhow!("MCP server is not connected")),
            }
        }
        Ok(runtime.snapshots.clone())
    }

    pub fn list_tools(&self) -> Vec<McpToolSnapshot> {
        let guard = self.runtimes.read();
        Self::connected_sorted(&guard).into_iter().flat_map(|runtime| runtime.snapshots.iter()).cloned().collect()
    }

    pub fn list_prompts(&self) -> Vec<McpPromptSnapshot> {
        let guard = self.runtimes.read();
        Self::connected_sorted(&guard).into_iter().flat_map(|runtime| runtime.prompts.iter()).cloned().collect()
    }

    pub fn find_prompt(&self, server_id: &str, name: &str) -> anyhow::Result<McpPromptSnapshot> {
        let guard = self.runtimes.read();
        let runtime = guard.get(server_id).ok_or_else(|| anyhow::anyhow!("MCP server is not connected: {}", server_id))?;
        runtime.prompts.iter().find(|prompt| prompt.name == name).cloned().ok_or_else(|| anyhow::anyhow!("MCP prompt not found: {}/{}", server_id, name))
    }

    pub async fn get_prompt(&self, server_id: &str, name: &str, arguments: Option<JsonObject>) -> anyhow::Result<GetPromptResult> {
        let peer = {
            let guard = self.runtimes.read();
            let runtime = guard.get(server_id).ok_or_else(|| anyhow::anyhow!("MCP server is not connected: {}", server_id))?;
            runtime.peer().ok_or_else(|| anyhow::anyhow!("MCP server is not connected: {}", server_id))?
        };
        let mut params = GetPromptRequestParams::new(name);
        if let Some(arguments) = arguments {
            params = params.with_arguments(arguments);
        }
        peer.get_prompt(params).await.map_err(|error| anyhow::anyhow!("get prompt failed: {}/{}: {}", server_id, name, error))
    }

    fn connected_sorted(guard: &HashMap<String, McpServerRuntime>) -> Vec<&McpServerRuntime> {
        let mut runtimes: Vec<&McpServerRuntime> = guard.values().filter(|item| item.connected).collect();
        runtimes.sort_by_key(|item| item.sort);
        runtimes
    }

    pub fn collect_rig_tools(&self) -> Vec<DynamicTool> {
        let guard = self.runtimes.read();
        let runtimes = Self::connected_sorted(&guard);

        let mut seen: HashSet<String> = HashSet::new();
        let mut out: Vec<DynamicTool> = Vec::new();
        for runtime in runtimes {
            for tool in runtime.tools() {
                let name = tool.name();
                if seen.contains(&name) {
                    warn!("[mcp] MCP tool name conflict, keep earlier: {} (server={})", name, runtime.server_id);
                    continue;
                }
                seen.insert(name);
                out.push(ToolBaseMcpWrap::new(tool.clone(), self.tool_conflict_manager.clone(), self.chat_session_cache_manager.clone()).as_rig_tool());
            }
        }
        out
    }
}
