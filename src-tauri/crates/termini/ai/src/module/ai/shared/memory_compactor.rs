use std::time::Duration;
use summer::plugin::Service;
use rig::completion::Message;
use termini_macros::base_app;
use crate::base::base_app::BaseApp;
use crate::model::ai_provide::AiProvide;
use tokio_util::sync::CancellationToken;
use crate::module::ai::dto::SharedMemory;
use summer::tracing::{error, info, warn};
use crate::model::dto::chat_memory::MemoryTurn;
use crate::service::service_agent::AgentService;
use crate::model::resp::resp_agent::RespAgentStatus;
use crate::module::ai::shared::ChatSessionCacheManager;
use crate::module::ai::stream::stream_runner::AiStreamRunner;
use crate::module::ai::agent::agent_build::stream_prompt_text;
use crate::model::resp::event::resp_event_agent::RespEventAgent;
use crate::module::ai::shared::chat_item_format::truncate_chars;
use crate::module::ai::agent::prompts::output_language_addon_text;
use crate::module::ai::shared::mem::mechanical::mechanical_summary;
use crate::module::ai::shared::mem::{budget, synthetic, transcript};
use crate::module::ai::agent::agent_fold::{AgentFold, FOLD_MAX_CHARS};
use crate::module::ai::shared::mem::fold_plan::{Compacted, FoldPlan, FoldState, PrefetchFold};

const FOLD_MODEL_FAIL_MAX: usize = 2;
const STAGE_THINKING: &str = "thinking";
const STAGE_MEMORY_TRIM: &str = "memoryTrim";
const FOLD_MODEL_TIMEOUT: Duration = Duration::from_secs(60);

#[base_app(sort = 766)]
#[derive(Clone, Service)]
pub struct MemoryCompactor {
    #[inject(component)]
    agent_fold: AgentFold,

    #[inject(component)]
    agent_service: AgentService,

    #[inject(component)]
    chat_session_cache_manager: ChatSessionCacheManager,
}

impl BaseApp for MemoryCompactor {
    fn name(&self) -> &'static str {
        "MemoryCompactor"
    }
}

impl MemoryCompactor {
    fn plan(&self, req_id: &str, messages: &[Message], reserved: usize, summary: &str) -> Option<FoldPlan> {
        let entry = self.chat_session_cache_manager.get(req_id)?;
        let token_limit = *entry.token_limit.lock();
        let provide = entry.light_provide();
        let addon = entry.addon_messages.lock().clone();
        drop(entry);
        Some(FoldPlan::build(token_limit, provide, &addon, messages, reserved, summary))
    }

    pub async fn compact(&self, req_id: &str, messages: &[Message], reserved: usize, summary: &str, cancel: &CancellationToken) -> Option<Compacted> {
        let mut failures = 0usize;
        self.compact_with_failures(req_id, messages, reserved, summary, cancel, &mut failures).await
    }

    async fn compact_with_failures(&self, req_id: &str, messages: &[Message], reserved: usize, summary: &str, cancel: &CancellationToken, failures: &mut usize) -> Option<Compacted> {
        let plan = self.plan(req_id, messages, reserved, summary)?;
        if !plan.over_high() {
            return Some(plan.intact(summary));
        }
        if cancel.is_cancelled() {
            warn!("[ai] 折叠被取消，上下文保持原样 req_id={}", req_id);
            return Some(plan.intact(summary));
        }
        let to_fold = plan.to_fold();
        if to_fold.is_empty() {
            return Some(plan.intact(summary));
        }

        info!("[ai] 上下文超过高水位 req_id={} tokens={} limit={} 折入:{}", req_id, plan.tokens, plan.limit, to_fold.len());
        self.emit_stage(req_id, STAGE_MEMORY_TRIM);
        let next = self.summarize_or_mechanical(req_id, &plan.provide, summary, &to_fold, cancel, failures).await;
        let dropped = to_fold.len();

        let mut body = plan.messages[dropped..].to_vec();
        let body_budget = plan.limit.saturating_sub(reserved + synthetic::summary_tokens(&next) + plan.pinned_tokens);
        if budget::estimate(&body) > body_budget {
            let (kept, hard) = budget::trim_to_token_limit(body, body_budget);
            warn!("[ai] 折叠后仍超限，按窗口硬裁 req_id={} 丢弃:{}", req_id, hard.len());
            body = kept;
        }
        let messages = budget::compose(&plan.pinned, body);
        let tokens_after = budget::estimate(&messages) + synthetic::summary_tokens(&next);
        self.emit_trimmed(req_id, plan.tokens, tokens_after);
        info!("[ai] 折叠完成 req_id={} 折入:{} limit={} tokens {}→{}", req_id, dropped, plan.limit, plan.tokens, tokens_after);
        Some(Compacted { messages, summary: next, dropped, aged: plan.aged })
    }

    pub async fn compact_before_run(&self, req_id: &str, memory: &SharedMemory, cancel: &CancellationToken) {
        let messages = memory.lock().clone();
        let summary = self.chat_session_cache_manager.summary(req_id);
        let Some(result) = self.compact(req_id, &messages, 0, &summary, cancel).await else {
            warn!("[ai] 跳过上下文压缩：未知 req_id={}", req_id);
            return;
        };
        if result.dropped == 0 && result.aged == 0 {
            return;
        }
        *memory.lock() = AiStreamRunner::sanitize_tool_history(result.messages);
        if result.dropped > 0 {
            self.trim_turns(req_id, result.dropped);
        }
        self.chat_session_cache_manager.set_summary(req_id, result.summary);
    }

    pub async fn fold_for_turn(&self, req_id: &str, history: &[Message], prompt: &Message, state: &mut FoldState, cancel: &CancellationToken) -> Option<Vec<Message>> {
        let reserved = budget::estimate_one(prompt);
        self.adopt_prefetch(req_id, state, false).await;

        let tail = &history[state.folded.min(history.len())..];
        let plan = self.plan(req_id, tail, reserved, &state.summary)?;
        if plan.over_high() {
            // 后台预折叠还没跑完就等它, 比重新起一次摘要便宜
            self.adopt_prefetch(req_id, state, true).await;
            let tail = &history[state.folded.min(history.len())..];
            let mut failures = state.model_failures;
            let result = self.compact_with_failures(req_id, tail, reserved, &state.summary, cancel, &mut failures).await?;
            state.model_failures = failures;
            if result.dropped > 0 {
                state.summary = result.summary.clone();
                state.folded += result.dropped;
                self.chat_session_cache_manager.set_summary(req_id, result.summary);
            }
            return Some(synthetic::with_summary(&state.summary, result.messages));
        }
        if plan.over_low() && state.prefetch.is_none() && !cancel.is_cancelled() {
            self.spawn_prefetch(req_id, state, &plan, cancel);
        }
        if plan.aged == 0 && state.summary.is_empty() {
            return None;
        }
        Some(synthetic::with_summary(&state.summary, budget::compose(&plan.pinned, plan.messages)))
    }

    fn spawn_prefetch(&self, req_id: &str, state: &mut FoldState, plan: &FoldPlan, cancel: &CancellationToken) {
        let to_fold = plan.to_fold();
        if to_fold.is_empty() {
            return;
        }
        let end = state.folded + to_fold.len();
        info!("[ai] 超过低水位, 后台预折叠 req_id={} tokens={} limit={} 折入:{}", req_id, plan.tokens, plan.limit, to_fold.len());
        let compactor = self.clone();
        let req_id_owned = req_id.to_string();
        let provide = plan.provide.clone();
        let previous = state.summary.clone();
        let cancel = cancel.clone();
        let handle = tokio::spawn(async move { compactor.summarize(&req_id_owned, &provide, &previous, &to_fold, &cancel).await });
        state.prefetch = Some(PrefetchFold { from: state.folded, end, handle });
    }

    async fn adopt_prefetch(&self, req_id: &str, state: &mut FoldState, wait: bool) {
        let Some(prefetch) = state.prefetch.as_ref() else {
            return;
        };
        if !wait && !prefetch.handle.is_finished() {
            return;
        }
        let Some(prefetch) = state.prefetch.take() else {
            return;
        };
        if prefetch.from != state.folded {
            prefetch.handle.abort();
            return;
        }
        if wait {
            self.emit_stage(req_id, STAGE_MEMORY_TRIM);
        }
        let outcome = prefetch.handle.await;
        if wait {
            self.emit_stage(req_id, STAGE_THINKING);
        }
        match outcome {
            Ok(Some(summary)) => {
                info!("[ai] 套用后台预折叠 req_id={} folded {}→{}", req_id, state.folded, prefetch.end);
                state.summary = summary.clone();
                state.folded = prefetch.end;
                self.chat_session_cache_manager.set_summary(req_id, summary);
            }
            Ok(None) => {
                state.model_failures += 1;
            }
            Err(error) => {
                warn!("[ai] 后台预折叠任务异常 req_id={} error={}", req_id, error);
                state.model_failures += 1;
            }
        }
    }

    async fn summarize_or_mechanical(&self, req_id: &str, provide: &AiProvide, summary: &str, to_fold: &[Message], cancel: &CancellationToken, failures: &mut usize) -> String {
        if *failures < FOLD_MODEL_FAIL_MAX {
            if let Some(text) = self.summarize(req_id, provide, summary, to_fold, cancel).await {
                return text;
            }
            *failures += 1;
        } else {
            info!("[ai] 摘要模型已连续失败 {} 次, 本次直接机械摘录 req_id={}", failures, req_id);
        }
        mechanical_summary(summary, to_fold)
    }

    async fn summarize(&self, req_id: &str, provide: &AiProvide, summary: &str, to_fold: &[Message], cancel: &CancellationToken) -> Option<String> {
        let transcript = transcript::build_transcript(to_fold);
        if transcript.is_empty() {
            return None;
        }
        let agent = match self.agent_fold.build_ai_agent(provide) {
            Ok(agent) => agent,
            Err(error) => {
                warn!("[ai] 构建折叠模型失败 req_id={} model={} error={:#}", req_id, provide.model, error);
                return None;
            }
        };
        let previous = if summary.is_empty() { "（无）" } else { summary };
        let mut prompt = AgentFold::render_user_prompt(previous, &transcript);
        if let Some(language_text) = self.chat_session_cache_manager.get(req_id).and_then(|entry| output_language_addon_text(entry.addon_messages.lock().as_slice())) {
            prompt = format!("{}\n\n{}", language_text, prompt);
        }
        let started = std::time::Instant::now();
        let raw = tokio::select! {
            biased;
            _ = cancel.cancelled() => return None,
            result = tokio::time::timeout(FOLD_MODEL_TIMEOUT, stream_prompt_text(agent, prompt)) => match result {
                Ok(Ok(raw)) => raw,
                Ok(Err(error)) => {
                    warn!("[ai] 折叠模型调用失败 req_id={} model={} 耗时={:.1}s error={:#}", req_id, provide.model, started.elapsed().as_secs_f64(), error);
                    return None;
                }
                Err(_) => {
                    warn!("[ai] 折叠模型超时 req_id={} model={} 上限={}s, 改用机械摘录", req_id, provide.model, FOLD_MODEL_TIMEOUT.as_secs());
                    return None;
                }
            },
        };
        let cleaned = truncate_chars(raw.trim(), FOLD_MAX_CHARS);
        if cleaned.is_empty() {
            warn!("[ai] 折叠模型返回空摘要 req_id={} model={}", req_id, provide.model);
            return None;
        }
        info!("[ai] 折叠摘要完成 req_id={} model={} 耗时={:.1}s 摘要={}字", req_id, provide.model, started.elapsed().as_secs_f64(), cleaned.chars().count());
        Some(cleaned)
    }

    fn trim_turns(&self, req_id: &str, dropped: usize) {
        let turns = self.chat_session_cache_manager.turns(req_id);
        let mut left = dropped;
        let mut next = Vec::with_capacity(turns.len());
        for turn in turns {
            let messages = synthetic::strip_synthetic(turn.messages);
            let take = left.min(messages.len());
            left -= take;
            next.push(MemoryTurn { item_id: turn.item_id, messages: messages.into_iter().skip(take).collect() });
        }
        if left > 0 {
            error!("[ai] 丢弃条数超出记忆分组 req_id={} 剩余:{}", req_id, left);
        }
        self.chat_session_cache_manager.set_turns(req_id, next);
    }

    fn emit_stage(&self, req_id: &str, stage: &str) {
        self.emit_status(req_id, RespAgentStatus::stage(req_id, stage));
    }

    fn emit_trimmed(&self, req_id: &str, tokens_before: usize, tokens_after: usize) {
        self.emit_status(req_id, RespAgentStatus::memory_trimmed(req_id, tokens_before, tokens_after));
    }

    fn emit_status(&self, req_id: &str, status: RespAgentStatus) {
        let Some(entry) = self.chat_session_cache_manager.get(req_id) else {
            return;
        };
        let ai_chat_id = entry.context.ai_chat_id.clone();
        drop(entry);
        let resp = RespEventAgent::OnStatus(status);
        self.agent_service.handler_agent_event(resp, ai_chat_id, None, None, None);
    }
}
