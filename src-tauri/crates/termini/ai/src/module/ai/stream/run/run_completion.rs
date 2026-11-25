use std::sync::Arc;
use futures::StreamExt;
use rig::tool::ToolContext;
use anyhow::{anyhow, Result};
use rig::completion::Message;
use rig::message::UserContent;
use std::collections::HashSet;
use rig::agent::StreamingError;
use summer::tracing::{info, warn};
use rig::completion::CompletionError;
use crate::module::ai::agent::AgentRole;
use crate::module::ai::dto::AiRunContext;
use rig::agent::{Agent, HookStack, MultiTurnStreamItem};
use crate::module::ai::shared::mem::fold_plan::FoldState;
use crate::module::ai::stream::stream_output::StreamOutput;
use crate::module::ai::shared::generated_image::GeneratedImage;
use crate::module::ai::stream::run::run_retry::COMPLETION_RETRY_MAX;
use rig::streaming::{StreamedAssistantContent, StreamedUserContent, StreamingPrompt};
use crate::module::ai::stream::stream_runner::{AiStreamRunner, OpenSegment, StreamErrorEmitted, StreamIdleTimeout, STREAM_IDLE_TIMEOUT};
use crate::module::ai::stream::hook::{CancelHook, ContextFoldHook, FinishHook, InvalidToolCallSkipHook, QueryDedupHook, SessionContextHook, ToolActiveHook, ToolCallIdInjectHook};

impl AiStreamRunner {
    pub(in crate::module::ai::stream) async fn consume_stream<O>(&self, agent: &Agent, context: Arc<AiRunContext>, role: AgentRole) -> Result<O>
    where
        O: StreamOutput,
    {
        let history = {
            let sanitized = Self::sanitize_tool_history(context.memory.lock().clone());
            *context.memory.lock() = sanitized.clone();
            sanitized
        };
        if context.emit.log {
            info!("[ai] 开始执行流 req_id={} run_id={} role={} history_msgs={}", context.req_id, context.run_id, role.as_name(), history.len());
        }
        let prompt_message = context.prompt_message();
        self.run_completion(agent, context, role, history, prompt_message, COMPLETION_RETRY_MAX).await
    }

    pub(in crate::module::ai::stream) async fn run_completion<O>(
        &self,
        agent: &Agent,
        context: Arc<AiRunContext>,
        role: AgentRole,
        history: Vec<Message>,
        prompt_message: Message,
        retries_left: usize,
    ) -> Result<O>
    where
        O: StreamOutput,
    {
        let mut final_text = String::new();
        let mut output_text = String::new();

        let mut tool_item_id: Option<String> = None;
        let mut text_segment: Option<OpenSegment> = None;
        let mut reasoning_segment: Option<OpenSegment> = None;
        let mut active_call_ids: HashSet<String> = HashSet::new();

        let cancel_token = context.cancel_token.clone();
        let emit_status = context.emit.status;
        let mut tool_context = ToolContext::new();
        tool_context.insert(Arc::clone(&context));

        let mut images_rejected = false;
        let mut pending_text = String::new();
        let mut transient_error: Option<StreamingError> = None;
        let mut generated_images: Vec<GeneratedImage> = Vec::new();
        let mut run_messages: Vec<Message> = vec![prompt_message.clone()];
        let fold_state = Arc::new(tokio::sync::Mutex::new(FoldState::new(self.chat_session_cache_manager.summary(&context.req_id))));
        let dedup_tools = self.collect_dedup_tools(&context.req_id);
        let supersede_tools = self.collect_supersede_tools(&context.req_id);
        let chat_id = self.chat_session_cache_manager.get(&context.req_id).map(|entry| entry.context.ai_chat_id.clone()).ok_or_else(|| anyhow!("unknown req_id: {}", context.req_id))?;
        let mut prompt = agent
            .stream_prompt(prompt_message)
            .tool_context(tool_context)
            .tool_concurrency(context.tool_concurrency)
            .add_hook(InvalidToolCallSkipHook)
            .add_hook(QueryDedupHook::new(dedup_tools))
            .add_hook(CancelHook { token: cancel_token.clone() })
            .add_hook(FinishHook { token: context.finish_token.clone() })
            .add_hook(ContextFoldHook {
                req_id: context.req_id.clone(),
                state: fold_state.clone(),
                supersede_tools,
                cancel_token: cancel_token.clone(),
                memory_compactor: self.memory_compactor.clone(),
                fold_generation: context.fold_generation.clone(),
            })
            .add_hook(SessionContextHook { req_id: context.req_id.clone(), derived_store: self.derived_store.clone(), chat_session_cache_manager: self.chat_session_cache_manager.clone() })
            .add_hook(ToolActiveHook { enabled: role == AgentRole::Main, req_id: context.req_id.clone(), chat_id, tool_activation_store: self.tool_activation_store.clone() })
            .add_hook(ToolCallIdInjectHook { req_id: context.req_id.clone(), chat_session_cache_manager: self.chat_session_cache_manager.clone() });
        if let Some(addon) = context.subagent.as_ref() {
            let mut addon_hooks = HookStack::new();
            addon.install(&mut addon_hooks);
            prompt = prompt.add_hook(addon_hooks);
        }
        let prompt = prompt.history(history.clone()).max_turns(context.max_turns);
        let mut open_idle = false;
        let opened = tokio::select! {
            biased;
            _ = cancel_token.cancelled() => None,
            _ = tokio::time::sleep(STREAM_IDLE_TIMEOUT) => {
                open_idle = true;
                None
            }
            ready = prompt => Some(ready),
        };
        if cancel_token.is_cancelled() {
            return Ok(O::from_stream_text(""));
        }
        if open_idle {
            let error = StreamingError::Completion(CompletionError::RequestError(Box::new(StreamIdleTimeout(STREAM_IDLE_TIMEOUT.as_secs()))));
            warn!("[ai] 模型 {}s 无输出 req_id={} run_id={} retries_left={}", STREAM_IDLE_TIMEOUT.as_secs(), context.req_id, context.run_id, retries_left);
            if retries_left > 0 {
                return self.retry_completion(context, role, error, retries_left).await;
            }
            if self.handle_error(&context, role, error).await {
                return Err(anyhow!(StreamErrorEmitted));
            }
            return Ok(O::from_stream_text(""));
        }
        let Some(mut stream) = opened else {
            return Ok(O::from_stream_text(""));
        };

        if emit_status {
            self.emit_thinking_status(&context);
        }

        loop {
            let watch_idle = active_call_ids.is_empty();
            let mut idle_timed_out = false;
            let next = tokio::select! {
                biased;
                _ = cancel_token.cancelled() => None,
                item = stream.next() => item,
                _ = tokio::time::sleep(STREAM_IDLE_TIMEOUT), if watch_idle => {
                    idle_timed_out = true;
                    None
                }
            };
            if cancel_token.is_cancelled() {
                break;
            }
            if idle_timed_out {
                let error = StreamingError::Completion(CompletionError::RequestError(Box::new(StreamIdleTimeout(STREAM_IDLE_TIMEOUT.as_secs()))));
                warn!("[ai] 模型 {}s 无输出 req_id={} run_id={} retries_left={}", STREAM_IDLE_TIMEOUT.as_secs(), context.req_id, context.run_id, retries_left);
                let completion_idle = pending_text.trim().is_empty() && matches!(run_messages.last(), Some(Message::User { .. }));
                Self::push_pending_text(&mut run_messages, &mut pending_text);
                let base = Self::folded_history(&history, &fold_state).await;
                Self::keep_run_messages(&context, &base, &run_messages);
                if retries_left > 0 && completion_idle {
                    transient_error = Some(error);
                    break;
                }
                if self.handle_error(&context, role, error).await {
                    self.finish_open_segments(&context, &mut text_segment, &mut reasoning_segment).await;
                    self.finish_active_tool_calls(&context, &mut active_call_ids);
                    return Err(anyhow!(StreamErrorEmitted));
                }
                break;
            }
            let Some(item) = next else {
                break;
            };
            match item {
                Ok(MultiTurnStreamItem::StreamAssistantItem(content)) => match content {
                    StreamedAssistantContent::Text(text) => {
                        self.append_text_delta(&context, &mut text_segment, &mut reasoning_segment, &text.text).await;
                        output_text.push_str(&text.text);
                        pending_text.push_str(&text.text);
                    }
                    StreamedAssistantContent::Reasoning { reasoning, .. } => {
                        let delta = reasoning.display_text();
                        self.append_reasoning_delta(&context, &mut text_segment, &mut reasoning_segment, &delta).await;
                    }
                    StreamedAssistantContent::ReasoningDelta { reasoning, .. } => {
                        self.append_reasoning_delta(&context, &mut text_segment, &mut reasoning_segment, &reasoning).await;
                    }
                    StreamedAssistantContent::ToolCall { tool_call, internal_call_id } => {
                        active_call_ids.insert(internal_call_id.clone());
                        Self::push_pending_text(&mut run_messages, &mut pending_text);
                        Self::push_tool_call(&mut run_messages, tool_call.clone());
                        self.handle_stream_tool_call(&tool_call, &context, internal_call_id, &mut tool_item_id, &mut text_segment, &mut reasoning_segment).await;
                    }
                    StreamedAssistantContent::Unknown(payload) => {
                        self.finish_open_segments(&context, &mut text_segment, &mut reasoning_segment).await;
                        if let Some(image) = self.handle_stream_image(&context, payload.value()).await {
                            generated_images.push(image);
                        }
                    }
                    StreamedAssistantContent::ToolCallDelta { .. } | StreamedAssistantContent::Final(_) => {
                        self.handle_ignored();
                    }
                },
                Ok(MultiTurnStreamItem::StreamUserItem(StreamedUserContent::ToolResult { tool_result, internal_call_id })) => {
                    active_call_ids.remove(&internal_call_id);
                    run_messages.push(Message::User { content: vec![UserContent::ToolResult(tool_result.clone())] });
                    self.handle_stream_tool_result(&context, internal_call_id, &mut tool_item_id, &tool_result, &mut text_segment, &mut reasoning_segment).await;
                    if emit_status && active_call_ids.is_empty() {
                        self.emit_thinking_status(&context);
                    }
                }
                Ok(MultiTurnStreamItem::CompletionCall(call)) => {
                    if emit_status {
                        self.record_context_tokens(&context, &call.usage);
                    }
                }
                Ok(MultiTurnStreamItem::FinalResponse(response)) => {
                    self.record_usage(&context, &response.usage, response.completion_calls.len());
                    self.handle_continue(&context, &response, &active_call_ids);
                    final_text = response.output;
                    if let Some(messages) = response.messages {
                        let base = Self::folded_history(&history, &fold_state).await;
                        *context.memory.lock() = Self::merge_run_messages(&base, messages, &context.addon_messages);
                    }
                    Self::note_generated_images(&context, &generated_images);
                }
                Ok(_) => {
                    self.handle_ignored();
                }
                Err(error) => {
                    if self.handle_cancelled(&context, &error, &mut active_call_ids) {
                        break;
                    }
                    if Self::is_images_rejected(&context, &error, run_messages.len() > 1 || !output_text.is_empty()) {
                        images_rejected = true;
                        break;
                    }
                    let completion_idle = pending_text.trim().is_empty() && matches!(run_messages.last(), Some(Message::User { .. }));
                    Self::push_pending_text(&mut run_messages, &mut pending_text);
                    let base = Self::folded_history(&history, &fold_state).await;
                    Self::keep_run_messages(&context, &base, &run_messages);
                    if retries_left > 0 && completion_idle && Self::is_transient_completion_error(&error) {
                        transient_error = Some(error);
                        break;
                    }
                    if self.handle_error(&context, role, error).await {
                        self.finish_open_segments(&context, &mut text_segment, &mut reasoning_segment).await;
                        self.finish_active_tool_calls(&context, &mut active_call_ids);
                        return Err(anyhow!(StreamErrorEmitted));
                    }
                }
            }
        }

        drop(stream);
        if cancel_token.is_cancelled() {
            Self::push_pending_text(&mut run_messages, &mut pending_text);
            Self::keep_run_messages(&context, &history, &run_messages);
        }

        self.finish_open_segments(&context, &mut text_segment, &mut reasoning_segment).await;
        self.finish_active_tool_calls(&context, &mut active_call_ids);
        if images_rejected {
            let dropped = Self::drop_memory_images(&context);
            warn!("[ai] 模型不接受图片内容，已忽略 {} 张图片改用纯文本重试 req_id={} run_id={}", dropped, context.req_id, context.run_id);
            self.notify_images_ignored(&context);
            return Box::pin(self.consume_stream(agent, context, role)).await;
        }
        if let Some(error) = transient_error {
            return self.retry_completion(context, role, error, retries_left).await;
        }
        if final_text.trim().is_empty() {
            final_text = output_text;
        }
        if context.emit.log {
            info!("[ai] 流执行完成 req_id={} run_id={} role:{}.", context.req_id, context.run_id, role.as_name());
        }
        Ok(O::from_stream_text(&final_text))
    }
}
