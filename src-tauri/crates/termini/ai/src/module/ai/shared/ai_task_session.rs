use anyhow::Result;
use std::sync::Arc;
use parking_lot::Mutex;
use summer::plugin::Service;
use async_trait::async_trait;
use rig::completion::Message;
use termini_macros::base_app;
use crate::util::id_factory::uuid;
use crate::base::base_app::BaseApp;
use summer::tracing::{error, info};
use tokio_util::sync::CancellationToken;
use crate::service::service_base::BaseService;
use crate::model::resp::resp_model::ResultCode;
use crate::service::service_agent::AgentService;
use crate::module::ai::tools::tool_base::ToolBase;
use crate::module::ai::agent::agent_base::AgentBase;
use crate::module::ai::shared::ChatSessionCacheManager;
use crate::model::req::req_chat_context::ReqChatContext;
use crate::module::ai::tools::tool_manager::ToolManager;
use crate::service::service_ai_provide::AiProvideService;
use crate::module::ai::stream::stream_output::StreamOutput;
use crate::module::ai::stream::stream_runner::AiStreamRunner;
use crate::model::resp::event::resp_event_agent::RespEventAgent;
use crate::service::service_ai_chat_memory::AiChatMemoryService;
use crate::module::ai::dto::{AiRunContext, ChatSessionCacheEntry, SharedMemory};
use crate::module::ai::tools::common::task::session::subagent_addon::SubagentRunAddon;
use crate::model::resp::resp_agent::{RespAgentBase, RespAgentCancelled, RespAgentDone, RespAgentError, RespAgentOpen, RespAgentUser};

pub struct AiTaskRun {
    pub req_id: String,
    pub memory: SharedMemory,
    pub context: Arc<ReqChatContext>,
    pub cancel_token: CancellationToken,
}

impl AiTaskRun {
    fn base(&self) -> RespAgentBase {
        RespAgentBase::new(self.req_id.clone())
    }
}

#[base_app(sort = 773)]
#[derive(Clone, Service)]
pub struct AiTaskSession {
    #[inject(component)]
    tool_manager: ToolManager,

    #[inject(component)]
    agent_service: AgentService,

    #[inject(component)]
    stream_runner: AiStreamRunner,

    #[inject(component)]
    ai_provide_service: AiProvideService,

    #[inject(component)]
    ai_chat_memory_service: AiChatMemoryService,

    #[inject(component)]
    chat_session_cache_manager: ChatSessionCacheManager,
}

#[async_trait]
impl BaseApp for AiTaskSession {
    fn name(&self) -> &'static str {
        "AiTaskSession"
    }
}

impl AiTaskSession {
    pub async fn open(&self, context: ReqChatContext, cancel_token: CancellationToken) -> Result<AiTaskRun> {
        let req_id = uuid();
        let user_item_id = uuid();
        let loaded = self.ai_chat_memory_service.load(context.ai_chat_id.clone()).await?;
        let memory: SharedMemory = Arc::new(Mutex::new(loaded.messages()));

        let ai_provide_shared = self.ai_provide_service.find_by_id(context.ai_provide_id.clone()).await?.ok_or_else(|| anyhow::Error::from(ResultCode::ProviderNotFound))?;
        let ai_provide = ai_provide_shared.read().clone();
        let token_limit = self.ai_provide_service.resolve_token_limit(&ai_provide)?;

        let context = Arc::new(context);
        let tools: Vec<Arc<dyn ToolBase>> = self.tool_manager.tools();
        let entry = ChatSessionCacheEntry::new(loaded.summary, token_limit, memory.clone(), ai_provide, user_item_id.clone(), loaded.turns, loaded.scene, context.clone(), tools, cancel_token.clone());
        *entry.assist_provide.lock() = self.ai_provide_service.resolve_assist().await;
        self.chat_session_cache_manager.insert(req_id.clone(), entry);
        self.tool_manager.on_start(&req_id);

        let run = AiTaskRun { req_id, memory, context, cancel_token };
        self.emit(&run, RespEventAgent::OnOpen(RespAgentOpen { base: run.base() }), None);
        self.emit(&run, RespEventAgent::OnUser(RespAgentUser { message: run.context.message.clone(), origin: run.context.origin.as_i16(), base: run.base() }), Some(user_item_id));
        Ok(run)
    }

    pub fn open_nested(&self, parent_req_id: &str, cancel_token: CancellationToken) -> Result<AiTaskRun> {
        let parent = self.chat_session_cache_manager.get(parent_req_id).ok_or_else(|| anyhow::anyhow!("unknown req_id: {}", parent_req_id))?;
        let context = parent.context.clone();
        let ai_provide = parent.ai_provide.clone();
        let assist_provide = parent.assist_provide.lock().clone();
        let token_limit = *parent.token_limit.lock();
        let confirm_mode = *parent.confirm_mode.borrow();
        drop(parent);

        let req_id = uuid();
        let memory: SharedMemory = Arc::new(Mutex::new(Vec::new()));
        let entry = ChatSessionCacheEntry::new(String::new(), token_limit, memory.clone(), ai_provide, uuid(), Vec::new(), None, context.clone(), self.tool_manager.tools(), cancel_token.clone());
        *entry.parent_req_id.lock() = Some(parent_req_id.to_string());
        *entry.assist_provide.lock() = assist_provide;
        entry.confirm_mode.send_replace(confirm_mode);
        self.chat_session_cache_manager.insert(req_id.clone(), entry);
        self.tool_manager.on_start(&req_id);
        Ok(AiTaskRun { req_id, memory, context, cancel_token })
    }

    pub fn close_nested(&self, run: AiTaskRun) {
        if run.cancel_token.is_cancelled() {
            self.emit(&run, RespEventAgent::OnCancelled(RespAgentCancelled { base: run.base() }), None);
        } else {
            self.emit(&run, RespEventAgent::OnDone(RespAgentDone { base: run.base() }), None);
        }
        self.tool_manager.on_stop(&run.req_id);
        self.chat_session_cache_manager.remove(&run.req_id);
        info!("[ai] 嵌套会话结束 req_id={} chat_id={}", run.req_id, run.context.ai_chat_id);
    }

    pub async fn run<A, O>(&self, run: &AiTaskRun, agent_base: &A, prompt: String, addon_messages: Vec<Message>) -> Result<O>
    where
        A: AgentBase + Clone + 'static,
        O: StreamOutput,
    {
        self.run_with_addon(run, agent_base, prompt, addon_messages, None).await
    }

    pub async fn run_with_addon<A, O>(&self, run: &AiTaskRun, agent_base: &A, prompt: String, addon_messages: Vec<Message>, subagent: Option<Arc<SubagentRunAddon>>) -> Result<O>
    where
        A: AgentBase + Clone + 'static,
        O: StreamOutput,
    {
        run.memory.lock().extend(addon_messages.iter().cloned());
        self.chat_session_cache_manager.set_addon_messages(&run.req_id, addon_messages.clone());

        let emit = agent_base.emit();
        let req_id = run.req_id.clone();
        let builder = agent_base.clone();
        let max_turns = agent_base.max_turns();
        let agent = agent_base.build_ai_agent(&run.req_id)?;
        let tool_concurrency = agent_base.tool_concurrency();
        let chat_id = self.chat_session_cache_manager.chat_id(&run.req_id);
        let rebuild_agent = Arc::new(move |id: &str| builder.build_ai_agent(id));
        let mut run_context = AiRunContext::new(req_id, agent, prompt, emit, max_turns, run.memory.clone(), tool_concurrency, chat_id, rebuild_agent, addon_messages.clone(), run.cancel_token.clone());
        if let Some(addon) = subagent {
            run_context = run_context.with_subagent(addon);
        }
        let result = self.stream_runner.run_stream::<O>(Arc::new(run_context), agent_base.role()).await;
        run.memory.lock().retain(|message| !addon_messages.iter().any(|addon| addon == message));
        self.chat_session_cache_manager.set_addon_messages(&run.req_id, Vec::new());
        result
    }

    pub async fn close(&self, run: AiTaskRun, error: Option<String>) {
        if let Some(error) = error {
            error!("[ai] req_id={} error={}", run.req_id, error);
            self.emit(&run, RespEventAgent::OnError(RespAgentError { code: None, error, base: run.base() }), None);
        }
        self.emit(&run, RespEventAgent::OnDone(RespAgentDone { base: run.base() }), None);
        self.tool_manager.on_stop(&run.req_id);
        self.agent_service.save_run_memory(run.req_id.clone()).await;
        self.chat_session_cache_manager.remove(&run.req_id);
        info!("[ai] req_id={} chat_id={} 会话结束.", run.req_id, run.context.ai_chat_id);
    }

    fn emit(&self, run: &AiTaskRun, event: RespEventAgent, id: Option<String>) {
        self.agent_service.handler_agent_event(event, run.context.ai_chat_id.clone(), id, None, None);
    }
}
