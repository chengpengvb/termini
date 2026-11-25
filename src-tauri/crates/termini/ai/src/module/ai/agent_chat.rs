use std::sync::Arc;
use summer::plugin::Service;
use anyhow::{anyhow, Result};
use termini_macros::base_app;
use rig::message::UserContent;
use crate::util::id_factory::uuid;
use summer::tracing::{info, warn};
use crate::model::ai_provide::AiProvide;
use tokio_util::sync::CancellationToken;
use crate::module::ai::flow::FlowAgentChat;
use crate::service::service_base::BaseService;
use crate::model::resp::resp_model::ResultCode;
use crate::service::service_agent::AgentService;
use crate::base::base_app::{AppContext, BaseApp};
use crate::model::req::event::ws_agent::ChatEvent;
use crate::service::service_ai_chat::AiChatService;
use crate::model::resp::resp_agent::RespAgentQuestion;
use crate::model::req::event::ws_agent::AgentAnswerCode;
use crate::model::req::req_ws_context::ReqSocketContext;
use crate::module::ai::tools::tool_manager::ToolManager;
use crate::service::service_ai_provide::AiProvideService;
use crate::service::service_ai_chat_item::AiChatItemService;
use crate::model::req::event::ws_agent::AgentQuestionAnswerData;
use crate::model::resp::event::resp_event_agent::RespEventAgent;
use crate::service::service_ai_chat_memory::AiChatMemoryService;
use crate::module::ai::tools::common::tool_ask_user::ToolAskUser;
use crate::module::ai::dto::{RunEndState, RunTrace, SharedMemory};
use crate::model::resp::resp_agent::{RespAgentBase, RespAgentCancelled};
use crate::module::ai::tools::common::task::session::task_registry::TaskRegistry;
use crate::service::service_ai_chat_memory_snapshot::AiChatMemorySnapshotService;
use crate::model::req::req_chat_context::{ChatOrigin, ConfirmMode, ReqChatContext};
use crate::module::ai::tools::common::image::vision_transcriber::VisionTranscriber;
use crate::module::ai::shared::{ChatFileStore, ChatSessionCacheManager, MemoryCompactor, ToolConflictManager, WakeBus};

mod media;
mod round;

#[base_app(sort = 772)]
#[derive(Clone, Service)]
pub struct AgentChat {
    #[inject(component)]
    wake_bus: WakeBus,

    #[inject(component)]
    tool_manager: ToolManager,

    #[inject(component)]
    task_registry: TaskRegistry,

    #[inject(component)]
    agent_service: AgentService,

    #[inject(component)]
    chat_file_store: ChatFileStore,

    #[inject(component)]
    ai_chat_service: AiChatService,

    #[inject(component)]
    flow_agent_chat: FlowAgentChat,

    #[inject(component)]
    memory_compactor: MemoryCompactor,

    #[inject(component)]
    ai_provide_service: AiProvideService,

    #[inject(component)]
    vision_transcriber: VisionTranscriber,

    #[inject(component)]
    ai_chat_item_service: AiChatItemService,

    #[inject(component)]
    tool_conflict_manager: ToolConflictManager,

    #[inject(component)]
    ai_chat_memory_service: AiChatMemoryService,

    #[inject(component)]
    chat_session_cache_manager: ChatSessionCacheManager,

    #[inject(component)]
    ai_chat_memory_snapshot_service: AiChatMemorySnapshotService,
}

#[async_trait::async_trait]
impl BaseApp for AgentChat {
    fn name(&self) -> &'static str {
        "AgentChat"
    }

    async fn on_exit(&self, _ctx: &AppContext) -> Result<()> {
        let req_ids = self.chat_session_cache_manager.active_req_ids();
        if req_ids.is_empty() {
            return Ok(());
        }
        info!("[ai] 退出前取消 {} 个进行中的对话并等待数据保存.", req_ids.len());
        for req_id in &req_ids {
            if let Err(error) = self.cancel(req_id) {
                warn!("[ai] 退出取消对话失败 req_id={} error={:#}", req_id, error);
            }
        }
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(8);
        while !self.chat_session_cache_manager.is_active_empty() {
            if std::time::Instant::now() >= deadline {
                warn!("[ai] 等待对话收尾超时, 剩余 {} 个会话未保存完.", self.chat_session_cache_manager.active_req_ids().len());
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
        info!("[ai] 对话收尾完成.");
        Ok(())
    }
}

impl AgentChat {
    fn get_context(&self, req: Arc<ReqSocketContext>) -> Result<Arc<ReqChatContext>> {
        let payload: ChatEvent = req.as_payload()?;
        Ok(Arc::new(ReqChatContext::from_payload(payload.data)))
    }

    async fn resolve_ai_provide(&self, context: &ReqChatContext) -> Result<AiProvide> {
        let shared = self.ai_provide_service.find_by_id(context.ai_provide_id.clone()).await?.ok_or_else(|| anyhow::Error::from(ResultCode::ProviderNotFound))?;
        let provide = shared.read().clone();
        Ok(provide)
    }

    pub async fn chat(&self, req_id: String, req: Arc<ReqSocketContext>) -> Result<()> {
        let context = self.get_context(req.clone())?;
        self.chat_with_context(req_id, context).await
    }

    pub async fn chat_with_context(&self, req_id: String, context: Arc<ReqChatContext>) -> Result<()> {
        let run = self.open_chat(req_id, context).await?;
        let result = self.run_chat(&run).await;
        self.close_chat(run, result).await
    }

    fn log_run_trace(&self, req_id: &str, chat_id: &str, provide_model: &str, started_at: std::time::Instant, end_state: RunEndState) {
        let (trace, tool_calls) = match self.chat_session_cache_manager.get(req_id) {
            Some(entry) => (*entry.run_trace.lock(), entry.tool_ids.len()),
            None => (RunTrace::default(), 0),
        };
        info!(
            "[ai] req_id={} chat_id={} provide={} end_state={} duration_ms={} tool_calls={} completion_calls={} input_tokens={} output_tokens={}",
            req_id,
            chat_id,
            provide_model,
            end_state.as_str(),
            started_at.elapsed().as_millis(),
            tool_calls,
            trace.completion_calls,
            trace.input_tokens,
            trace.output_tokens
        );
    }

    async fn run_agent(&self, req_id: String, prompt: String, images: Vec<UserContent>, memory: SharedMemory, cancel_token: CancellationToken) -> Result<()> {
        self.flow_agent_chat.run(req_id, prompt, images, memory, cancel_token).await
    }

    pub fn cancel(&self, req_id: &str) -> Result<()> {
        let Some(entry) = self.chat_session_cache_manager.get(req_id) else {
            return Ok(());
        };
        let ai_chat_id = entry.context.ai_chat_id.clone();
        drop(entry);

        self.chat_session_cache_manager.cancel(req_id)?;
        self.tool_manager.on_stop(req_id);

        let resp_cancelled = RespEventAgent::OnCancelled(RespAgentCancelled { base: RespAgentBase::new(req_id.to_string()) });
        self.agent_service.handler_agent_event(resp_cancelled, ai_chat_id.to_string(), None, None, None);
        Ok(())
    }

    pub fn tool_cancel(&self, req_id: &str, run_id: &str, call_id: &str) -> Result<()> {
        if self.chat_session_cache_manager.cancel_tool(req_id, run_id, call_id).is_ok() {
            return Ok(());
        }
        if self.task_registry.cancel_by_call_id(call_id) {
            return Ok(());
        }
        Err(anyhow!("unknown tool cancel: req_id={} run_id={} call_id={}", req_id, run_id, call_id))
    }

    pub async fn tool_answer(&self, call_id: &str, code: AgentAnswerCode) -> Result<()> {
        self.tool_conflict_manager.resolve(call_id, code).await
    }

    fn emit_question_answered(&self, chat_id: &str, pid: Option<String>, question: &RespAgentQuestion) {
        let Some(answered) = question.answered() else {
            return;
        };
        self.agent_service.handler_agent_event(RespEventAgent::OnQuestionAnswered(answered), chat_id.to_string(), None, pid, None);
    }

    pub async fn question_answer(&self, data: AgentQuestionAnswerData) -> Result<()> {
        let Some((item, mut question)) = self.ai_chat_item_service.find_question(data.ai_chat_id.clone(), &data.call_id).await? else {
            return Err(anyhow!("question not found: chat_id={} call_id={}", data.ai_chat_id, data.call_id));
        };
        if let Some(state) = &question.state {
            return Err(anyhow!("question already settled: call_id={} state={}", data.call_id, state));
        }
        let pid = item.pid.clone();
        if data.skipped {
            question.state = Some(RespAgentQuestion::STATE_SKIPPED.to_string());
            self.ai_chat_item_service.save_question(item, &question).await?;
            self.emit_question_answered(&data.ai_chat_id, pid, &question);
            return Ok(());
        }
        let message = ToolAskUser::render_answer(&question.questions, &data.answers)?;
        if self.chat_session_cache_manager.get_req_id_by_chat_id(&data.ai_chat_id).is_some() {
            return Err(anyhow!("chat is running, answer rejected: chat_id={}", data.ai_chat_id));
        }
        let confirm_mode = ConfirmMode::from_i8(data.confirm_mode).ok_or_else(|| anyhow!("unknown confirm mode: {}", data.confirm_mode))?;
        question.state = Some(RespAgentQuestion::STATE_ANSWERED.to_string());
        question.answers = Some(serde_json::to_value(&data.answers)?);
        self.ai_chat_item_service.save_question(item, &question).await?;
        self.emit_question_answered(&data.ai_chat_id, pid, &question);
        let context = Arc::new(ReqChatContext {
            message,
            confirm_mode,
            images: Vec::new(),
            origin: ChatOrigin::Answer,
            ai_chat_id: data.ai_chat_id,
            ai_provide_id: data.ai_provide_id,
            thinking_level: data.thinking_level,
        });
        self.chat_with_context(uuid(), context).await
    }

    pub fn set_confirm_mode(&self, ai_chat_id: &str, mode: ConfirmMode) {
        let changed = self.chat_session_cache_manager.set_confirm_mode(ai_chat_id, mode);
        if changed.is_empty() {
            return;
        }
        info!("[ai] 运行中切换审批档位 chat_id={} mode={:?} req_ids={:?}", ai_chat_id, mode, changed);
    }
}
