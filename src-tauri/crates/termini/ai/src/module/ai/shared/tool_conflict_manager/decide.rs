use std::sync::Arc;
use serde_json::Value;
use summer::tracing::info;
use crate::module::ai::dto::AiRunContext;
use crate::module::ai::shared::ToolConflictManager;
use crate::model::req::req_chat_context::ConfirmMode;
use crate::model::req::event::ws_agent::AgentAnswerCode;
use crate::module::ai::tools::ai_tool_error::AiToolError;
use crate::module::ai::shared::conflict::{AuditOutcome, ToolConflictPending};

impl ToolConflictManager {
    pub(super) async fn decide(
        &self,
        context: &AiRunContext,
        call_id: &str,
        pending: &Arc<ToolConflictPending>,
        args: Value,
        rule_reason: Option<String>,
        prefix: Option<String>,
        extra_danger: &[String],
    ) -> Result<(), AiToolError> {
        let tool_id = pending.tool_id.clone();
        let Some(mut mode_rx) = self.chat_session_cache_manager.watch_confirm_mode(&context.req_id) else {
            return Err(AiToolError::Execution(format!("unknown req_id: {}", context.req_id)));
        };
        let mut asked = false;
        let mut audited = rule_reason.is_some();
        let mut reason = rule_reason;
        loop {
            let mode = self.resolve_confirm_mode(&context.req_id);
            if mode == ConfirmMode::Silent && !Self::hits_danger_word(&pending.subject, extra_danger) {
                if !asked {
                    return self.guard_by_danger_word(context, call_id, &tool_id, args, &pending.subject, extra_danger, prefix);
                }
                info!("[ai] 档位切到完全放行, 无危险词自动放行 tool_id={} call_id={}", tool_id, call_id);
                return self.finish(context, call_id, &tool_id, AgentAnswerCode::Allow);
            }
            if mode == ConfirmMode::Auto && !audited {
                if self.is_read_only_tool(&context.req_id, &tool_id) {
                    info!("[ai] 只读工具自动放行, 跳过审计 tool_id={} call_id={}", tool_id, call_id);
                    if asked {
                        return self.finish(context, call_id, &tool_id, AgentAnswerCode::Allow);
                    }
                    self.emit_conflict(context, call_id, &tool_id, args, true, None, prefix);
                    return Ok(());
                }
                let outcome = {
                    let audit = self.audit(context, call_id, &tool_id, &args);
                    tokio::select! {
                        outcome = audit => outcome,
                        answer = self.wait_answer(call_id, &context.cancel_token, &mut mode_rx) => match answer {
                            Ok(Some(code)) => {
                                info!("[ai] 自动审核期间收到手动答复, 以手动为准 tool_id={} call_id={} code={:?}", tool_id, call_id, code);
                                return self.finish(context, call_id, &tool_id, code);
                            }
                            Ok(None) => continue,
                            Err(_) => return self.finish(context, call_id, &tool_id, AgentAnswerCode::Reject),
                        },
                    }
                };
                audited = true;
                match outcome {
                    AuditOutcome::Allow(checked) => {
                        if asked {
                            info!("[ai] 档位切到替我审批, 审核通过放行 tool_id={} call_id={}", tool_id, call_id);
                            return self.finish(context, call_id, &tool_id, AgentAnswerCode::Allow);
                        }
                        self.emit_conflict(context, call_id, &tool_id, args, true, checked, prefix);
                        return Ok(());
                    }
                    AuditOutcome::Escalate(text) => reason = Some(text),
                }
            }
            if !asked {
                asked = true;
                self.emit_conflict(context, call_id, &tool_id, args.clone(), false, reason.clone(), prefix.clone());
            }
            match self.wait_answer(call_id, &context.cancel_token, &mut mode_rx).await {
                Ok(Some(code)) => return self.finish(context, call_id, &tool_id, code),
                Ok(None) => continue,
                Err(_) => return self.finish(context, call_id, &tool_id, AgentAnswerCode::Reject),
            }
        }
    }

    fn finish(&self, context: &AiRunContext, call_id: &str, tool_id: &str, code: AgentAnswerCode) -> Result<(), AiToolError> {
        self.emit_answered(context, call_id, tool_id, code);
        if code.is_allow() {
            return Ok(());
        }
        Err(AiToolError::Denied("user rejected this operation".to_string()))
    }
}
