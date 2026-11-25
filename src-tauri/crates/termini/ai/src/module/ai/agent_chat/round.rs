use anyhow::Result;
use std::sync::Arc;
use parking_lot::Mutex;
use std::time::Instant;
use std::sync::atomic::Ordering;
use crate::util::id_factory::uuid;
use tokio_util::sync::CancellationToken;
use summer::tracing::{error, info, warn};
use crate::module::ai::agent_chat::AgentChat;
use crate::module::ai::tools::tool_base::ToolBase;
use crate::model::resp::event::resp_event_agent::RespEventAgent;
use crate::module::ai::stream::stream_runner::StreamErrorEmitted;
use crate::model::req::req_chat_context::{ChatOrigin, ReqChatContext};
use crate::module::ai::dto::{ChatRun, ChatSessionCacheEntry, RunEndState, SharedMemory};
use crate::model::resp::resp_agent::{RespAgentBase, RespAgentDone, RespAgentError, RespAgentOpen, RespAgentUser};

impl AgentChat {
    pub(super) async fn open_chat(&self, req_id: String, context: Arc<ReqChatContext>) -> Result<ChatRun> {
        let user_item_id = uuid();
        let loaded = self.ai_chat_memory_service.load(context.ai_chat_id.clone()).await?;
        let messages = loaded.messages();
        self.ai_chat_memory_snapshot_service.save_snapshot(context.ai_chat_id.clone(), user_item_id.clone(), &loaded).await?;
        let prompt = self.resolve_user_prompt(&context.ai_chat_id, context.message.clone()).await?;
        let cancel_token = CancellationToken::new();
        let image_paths = self.save_images(&context.ai_chat_id, &context.images).await?;
        let ai_provide = self.resolve_ai_provide(&context).await?;
        let provide_model = ai_provide.model.clone();
        let inline_images = self.build_inline_images(&ai_provide, &image_paths).await?;
        let llm_prompt = Self::build_llm_prompt(&prompt, &image_paths, inline_images.len());
        let token_limit = self.ai_provide_service.resolve_token_limit(&ai_provide)?;
        info!("[ai] 加载记忆 chat_id={} turns={} msgs={} summary={} token_limit={}", context.ai_chat_id, loaded.turns.len(), messages.len(), loaded.summary.chars().count(), token_limit);
        let memory: SharedMemory = Arc::new(Mutex::new(messages));
        let tools: Vec<Arc<dyn ToolBase>> = self.tool_manager.tools();
        let entry = ChatSessionCacheEntry::new(loaded.summary, token_limit, memory.clone(), ai_provide, user_item_id.clone(), loaded.turns, loaded.scene, context.clone(), tools, cancel_token.clone());
        *entry.assist_provide.lock() = self.ai_provide_service.resolve_assist().await;
        self.chat_session_cache_manager.insert(req_id.clone(), entry);
        self.tool_manager.on_start(&req_id);
        Ok(ChatRun { req_id, user_item_id, prompt, llm_prompt, inline_images, memory, cancel_token, provide_model, started_at: Instant::now(), context })
    }

    pub(super) async fn run_chat(&self, run: &ChatRun) -> Result<()> {
        self.emit_open(run);
        self.emit_user(run);
        if let Err(error) = self.ai_chat_service.touch(run.context.ai_chat_id.to_string()).await {
            warn!("[ai] 刷新会话活动时间失败 chat_id={} error={:#}", run.context.ai_chat_id, error);
        }
        self.decline_pending(&run.context.ai_chat_id).await;
        self.memory_compactor.compact_before_run(&run.req_id, &run.memory, &run.cancel_token).await;
        if run.cancel_token.is_cancelled() {
            info!("[ai] req_id={} 在记忆压缩阶段被取消，本轮不再执行.", run.req_id);
            return Ok(());
        }
        self.run_agent(run.req_id.clone(), run.llm_prompt.clone(), run.inline_images.clone(), run.memory.clone(), run.cancel_token.clone()).await
    }

    pub(super) async fn close_chat(&self, run: ChatRun, result: Result<()>) -> Result<()> {
        let end_state = self.end_state(&run, result);
        self.emit_done(&run);
        self.tool_manager.on_stop(&run.req_id);
        self.agent_service.save_run_memory(run.req_id.clone()).await;
        self.log_run_trace(&run.req_id, &run.context.ai_chat_id, &run.provide_model, run.started_at, end_state);
        if let Some(next) = self.continue_context(&run, end_state) {
            self.chat_session_cache_manager.remove(&run.req_id);
            return Box::pin(self.chat_with_context(uuid(), next)).await;
        }
        self.chat_session_cache_manager.remove(&run.req_id);
        self.wake_bus.on_round_end(&run.context.ai_chat_id);
        Ok(())
    }

    fn emit_open(&self, run: &ChatRun) {
        let event = RespEventAgent::OnOpen(RespAgentOpen { base: RespAgentBase::new(run.req_id.clone()) });
        self.agent_service.handler_agent_event(event, run.context.ai_chat_id.to_string(), None, None, None);
    }

    fn emit_user(&self, run: &ChatRun) {
        let event = RespEventAgent::OnUser(RespAgentUser { message: run.prompt.clone(), origin: run.context.origin.as_i16(), base: RespAgentBase::new(run.req_id.clone()) });
        self.agent_service.handler_agent_event(event, run.context.ai_chat_id.to_string(), Some(run.user_item_id.clone()), None, None);
    }

    fn emit_done(&self, run: &ChatRun) {
        let event = RespEventAgent::OnDone(RespAgentDone { base: RespAgentBase::new(run.req_id.clone()) });
        self.agent_service.handler_agent_event(event, run.context.ai_chat_id.to_string(), None, None, None);
    }

    async fn decline_pending(&self, chat_id: &str) {
        match self.ai_chat_item_service.decline_pending_questions(chat_id.to_string()).await {
            Ok(declined) => {
                for (pid, question) in declined {
                    self.emit_question_answered(chat_id, pid, &question);
                }
            }
            Err(error) => warn!("[ai] 关闭未作答提问失败 chat_id={} error={:#}", chat_id, error),
        }
    }

    fn end_state(&self, run: &ChatRun, result: Result<()>) -> RunEndState {
        match result {
            Ok(_) if run.cancel_token.is_cancelled() => RunEndState::Cancelled,
            Ok(_) => RunEndState::Done,
            Err(error) => {
                error!("[ai] req_id={} error={:#}", run.req_id, error);
                if !StreamErrorEmitted::displayed(&error) {
                    let event = RespEventAgent::OnError(RespAgentError { code: None, error: format!("{:#}", error), base: RespAgentBase::new(run.req_id.clone()) });
                    self.agent_service.handler_agent_event(event, run.context.ai_chat_id.to_string(), None, None, None);
                }
                RunEndState::Failed
            }
        }
    }

    fn continue_context(&self, run: &ChatRun, end_state: RunEndState) -> Option<Arc<ReqChatContext>> {
        if end_state != RunEndState::Done {
            return None;
        }
        let entry = self.chat_session_cache_manager.get(&run.req_id)?;
        if !entry.auto_continue.load(Ordering::SeqCst) {
            return None;
        }
        let confirm_mode = *entry.confirm_mode.borrow();
        Some(Arc::new(ReqChatContext {
            confirm_mode,
            images: Vec::new(),
            message: "continue".to_string(),
            origin: ChatOrigin::Continue,
            ai_chat_id: run.context.ai_chat_id.clone(),
            ai_provide_id: run.context.ai_provide_id.clone(),
            thinking_level: run.context.thinking_level.clone(),
        }))
    }
}
