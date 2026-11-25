use std::sync::Arc;
use dashmap::DashMap;
use serde_json::Value;
use parking_lot::Mutex;
use summer::plugin::Service;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use termini_macros::base_app;
use crate::util::id_factory::uuid;
use summer::plugin::LazyComponent;
use summer::tracing::{info, warn};
use crate::base::base_app::BaseApp;
use crate::module::ai::dto::AiRunContext;
use crate::module::ai::shared::ToolCallDonePush;
use crate::service::service_agent::AgentService;
use rig::tool::{DynamicTool, ToolContext, ToolSet};
use crate::module::ai::tools::tool_base::ToolDomain;
use crate::module::ai::shared::ChatSessionCacheManager;
use crate::module::ai::mcp::mcp_client_pool::McpClientPool;
use crate::model::resp::event::resp_event_agent::RespEventAgent;
use crate::module::common::dto::shell_chunk_callback::ShellChunkCallback;
use crate::module::ai::tools::base::{ToolBaseOutWrap, TOOL_OUTPUT_MAX_TOKENS};
use crate::model::resp::resp_agent::{RespAgentBase, RespAgentRunBase, RespAgentToolCall};

pub struct BridgeSession {
    pub on_chunk: ShellChunkCallback,
    pub run_context: Arc<AiRunContext>,
}

fn create_sessions() -> Arc<DashMap<String, Arc<BridgeSession>>> {
    Arc::new(DashMap::new())
}

fn create_endpoint() -> Arc<Mutex<Option<String>>> {
    Arc::new(Mutex::new(None))
}

#[base_app(sort = 759)]
#[derive(Clone, Service)]
pub struct ToolBridge {
    #[inject(component)]
    agent_service: AgentService,

    #[inject(func = create_endpoint())]
    endpoint: Arc<Mutex<Option<String>>>,

    #[inject(component)]
    tool_call_done_push: ToolCallDonePush,

    mcp_client_pool: LazyComponent<McpClientPool>,

    #[inject(component)]
    chat_session_cache_manager: ChatSessionCacheManager,

    #[inject(func = create_sessions())]
    sessions: Arc<DashMap<String, Arc<BridgeSession>>>,
}

#[async_trait]
impl BaseApp for ToolBridge {
    fn name(&self) -> &'static str {
        "ToolBridge"
    }
}

impl ToolBridge {
    pub fn set_endpoint(&self, endpoint: String) {
        *self.endpoint.lock() = Some(endpoint);
    }

    pub fn endpoint(&self) -> Result<String> {
        self.endpoint.lock().clone().ok_or_else(|| anyhow!("tool bridge endpoint is not configured"))
    }

    pub fn issue(&self, run_context: Arc<AiRunContext>, on_chunk: ShellChunkCallback) -> String {
        let token = uuid();
        self.sessions.insert(token.clone(), Arc::new(BridgeSession { on_chunk, run_context }));
        token
    }

    pub fn revoke(&self, token: &str) {
        self.sessions.remove(token);
    }

    pub async fn call(&self, token: &str, name: &str, args: Value) -> Result<String> {
        let session = self.sessions.get(token).map(|entry| entry.value().clone()).ok_or_else(|| anyhow!("invalid or expired tool bridge token"))?;
        let run_context = session.run_context.clone();
        if run_context.cancel_token.is_cancelled() {
            return Err(anyhow!("the owning run has been cancelled"));
        }
        let tool = self.resolve_tool(&run_context.req_id, name)?;
        let call_id = uuid();
        let mut args = match args {
            Value::Object(map) => map,
            Value::Null => serde_json::Map::new(),
            other => return Err(anyhow!("tool args must be a JSON object, got {}", other)),
        };
        args.insert("__internal_call_id".to_string(), Value::String(call_id.clone()));
        let args_value = Value::Object(args);
        self.chat_session_cache_manager.insert_tool_cancel(&run_context.req_id, run_context.run_id.clone(), call_id.clone())?;
        self.emit_tool_call(&run_context, &call_id, name, &args_value);
        (session.on_chunk)(format!("[tool] {}\n", name).into_bytes()).await;
        let mut tool_context = ToolContext::new();
        tool_context.insert(Arc::clone(&run_context));
        let tool_name = tool.name().to_string();
        let result = ToolSet::from_dynamic_tools(vec![tool]).execute(&tool_name, args_value.to_string(), &mut tool_context).await;
        if let Err(error) = self.chat_session_cache_manager.remove_tool_cancel(&run_context.req_id, &run_context.run_id, &call_id) {
            warn!("[ai] 移除 tool cancel 失败 req_id={} call_id={} error={}", run_context.req_id, call_id, error);
        }
        if let Some(error) = result.error() {
            warn!("[ai] 嵌套工具失败 req_id={} tool={} call_id={} error={}", run_context.req_id, name, call_id, error);
            return Err(anyhow!("{}", error));
        }
        if let Some(error) = result.refusal() {
            warn!("[ai] 嵌套工具失败 req_id={} tool={} call_id={} error={}", run_context.req_id, name, call_id, error);
            return Err(anyhow!("{}", error));
        }
        let Some(text) = result.output().as_text() else {
            warn!("[ai] 嵌套工具失败 req_id={} tool={} call_id={} error=output is not text", run_context.req_id, name, call_id);
            return Err(anyhow!("tool {} output is not text", name));
        };
        info!("[ai] 嵌套工具完成 req_id={} tool={} call_id={}", run_context.req_id, name, call_id);
        Ok(text.to_string())
    }

    fn resolve_tool(&self, req_id: &str, name: &str) -> Result<DynamicTool> {
        let entry = self.chat_session_cache_manager.get(req_id).ok_or_else(|| anyhow!("unknown req_id: {}", req_id))?;
        if let Some(tool) = entry.tools.iter().find(|tool| tool.tool_id().as_str() == name && tool.domains().contains(&ToolDomain::Main)).cloned() {
            drop(entry);
            return Ok(tool.as_rig_tool(Some(self.tool_call_done_push.clone())));
        }
        drop(entry);
        let mcp_client_pool = self.mcp_client_pool.get().map_err(|error| anyhow!(error.to_string()))?;
        let mcp = mcp_client_pool.collect_rig_tools().into_iter().find(|tool| tool.name() == name).ok_or_else(|| anyhow!("unknown tool: {}", name))?;
        Ok(ToolBaseOutWrap::wrap(mcp, TOOL_OUTPUT_MAX_TOKENS, Some(self.tool_call_done_push.clone())))
    }

    fn emit_tool_call(&self, run_context: &AiRunContext, call_id: &str, name: &str, args: &Value) {
        let Some(entry) = self.chat_session_cache_manager.get(&run_context.req_id) else {
            return;
        };
        let chat_id = entry.context.ai_chat_id.clone();
        drop(entry);
        let item_id = match self.chat_session_cache_manager.ensure_tool_item_id(&run_context.req_id, call_id) {
            Ok(id) => id,
            Err(error) => {
                warn!("[ai] 登记 tool item id 失败 req_id={} call_id={} error={}", run_context.req_id, call_id, error);
                return;
            }
        };
        self.chat_session_cache_manager.set_tool_id(&run_context.req_id, call_id, name);
        let event = RespEventAgent::OnToolCall(RespAgentToolCall {
            args: args.clone(),
            tool_id: name.to_string(),
            call_id: call_id.to_string(),
            base: RespAgentBase::new(run_context.req_id.clone()),
            run: RespAgentRunBase { run_id: run_context.run_id.clone() },
        });
        self.agent_service.handler_agent_event(event, chat_id, Some(item_id), None, None);
    }
}
