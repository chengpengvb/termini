use serde_json::Value;
use tokio::sync::watch;
use summer::tracing::warn;
use anyhow::{bail, Result};
use tokio_util::sync::CancellationToken;
use crate::module::ai::dto::AiRunContext;
use crate::model::dto::cancel_session::cancelled;
use crate::module::ai::shared::ToolConflictManager;
use crate::model::req::req_chat_context::ConfirmMode;
use crate::model::req::event::ws_agent::AgentAnswerCode;
use crate::model::resp::event::resp_event_agent::RespEventAgent;
use crate::model::resp::resp_agent::{RespAgentBase, RespAgentConflict, RespAgentConflictAnswered, RespAgentRunBase};

impl ToolConflictManager {
    pub(super) fn emit_conflict(&self, context: &AiRunContext, call_id: &str, tool_id: &str, args: Value, auto_allowed: bool, audit_reason: Option<String>, session_prefix: Option<String>) {
        let Some(ai_chat_id) = self.resolve_chat_id(&context.req_id) else {
            warn!("[ai] 推送确认事件失败：未知 req_id={}", context.req_id);
            return;
        };
        let pid = self.chat_session_cache_manager.get_tool_item_id(&context.req_id, call_id);
        let event = RespEventAgent::OnConflict(RespAgentConflict {
            args,
            auto_allowed,
            audit_reason,
            session_prefix,
            call_id: call_id.to_string(),
            tool_id: tool_id.to_string(),
            base: RespAgentBase::new(context.req_id.clone()),
            run: RespAgentRunBase { run_id: context.run_id.clone() },
        });
        self.agent_service.handler_agent_event(event, ai_chat_id, None, pid, None);
    }

    pub(super) fn emit_answered(&self, context: &AiRunContext, call_id: &str, tool_id: &str, code: AgentAnswerCode) {
        let Some(ai_chat_id) = self.resolve_chat_id(&context.req_id) else {
            warn!("[ai] 推送答复事件失败：未知 req_id={}", context.req_id);
            return;
        };
        let pid = self.chat_session_cache_manager.get_tool_item_id(&context.req_id, call_id);
        let event = RespEventAgent::OnConflictAnswered(RespAgentConflictAnswered {
            code: code as i16,
            call_id: call_id.to_string(),
            tool_id: tool_id.to_string(),
            base: RespAgentBase::new(context.req_id.clone()),
            run: RespAgentRunBase { run_id: context.run_id.clone() },
        });
        self.agent_service.handler_agent_event(event, ai_chat_id, None, pid, None);
    }

    pub(super) fn resolve_chat_id(&self, req_id: &str) -> Option<String> {
        let entry = self.chat_session_cache_manager.get(req_id)?;
        let ai_chat_id = entry.context.ai_chat_id.clone();
        Some(ai_chat_id)
    }

    pub(super) async fn wait_answer(&self, call_id: &str, cancel_token: &CancellationToken, mode_rx: &mut watch::Receiver<ConfirmMode>) -> Result<Option<AgentAnswerCode>> {
        let Some(pending) = self.pendings.get(call_id).map(|entry| entry.value().clone()) else {
            bail!("unknown tool conflict call_id: {}", call_id);
        };
        let inbox = pending.inbox.clone();
        loop {
            if cancel_token.is_cancelled() {
                return Err(cancelled());
            }
            let notified = inbox.notify.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();

            let found = {
                let mut guard = inbox.queue.lock().await;
                let pos = guard.iter().position(|entry| entry.call_id == call_id);
                pos.and_then(|index| guard.remove(index)).map(|entry| entry.code)
            };
            if let Some(code) = found {
                return Ok(Some(code));
            }

            tokio::select! {
                _ = &mut notified => {}
                changed = mode_rx.changed() => {
                    if changed.is_err() {
                        return Err(cancelled());
                    }
                    return Ok(None);
                }
                _ = cancel_token.cancelled() => {
                    return Err(cancelled());
                }
            }
        }
    }
}
