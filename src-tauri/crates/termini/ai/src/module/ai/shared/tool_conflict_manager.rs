use std::sync::Arc;
use dashmap::DashMap;
use serde::Serialize;
use serde_json::Value;
use anyhow::{bail, Result};
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use summer::tracing::{info, warn};
use crate::base::base_app::BaseApp;
use crate::module::ai::dto::AiRunContext;
use crate::module::ai::shared::danger_word;
use crate::module::ai::flow::FlowCommandAudit;
use crate::module::ai::shared::ConflictSubject;
use crate::service::service_agent::AgentService;
use crate::service::service_config::ConfigService;
use crate::model::req::req_chat_context::ConfirmMode;
use crate::module::ai::shared::ChatSessionCacheManager;
use crate::model::req::event::ws_agent::AgentAnswerCode;
use crate::module::ai::tools::ai_tool_error::AiToolError;
use crate::service::service_command_allow::CommandAllowService;
use crate::service::service_command_rule::{CommandRuleService, RuleHit};
use crate::model::command_rule::{RULE_DECISION_ALLOW, RULE_DECISION_FORBIDDEN};
use crate::module::ai::shared::conflict::{AuditOutcome, CommandPrefixRule, SessionAllowStore, ToolConflictInbox, ToolConflictPending};

mod emit;
mod decide;

const APPROVED_LOG_MAX: usize = 20;

fn create_pendings() -> Arc<DashMap<String, Arc<ToolConflictPending>>> {
    Arc::new(DashMap::new())
}

fn create_approved() -> Arc<DashMap<String, Vec<String>>> {
    Arc::new(DashMap::new())
}

#[base_app(sort = 763)]
#[derive(Clone, Service)]
pub struct ToolConflictManager {
    #[inject(component)]
    agent_service: AgentService,

    #[inject(component)]
    config_service: ConfigService,

    #[inject(func = SessionAllowStore::default())]
    session_allows: SessionAllowStore,

    #[inject(component)]
    flow_command_audit: FlowCommandAudit,

    #[inject(component)]
    command_rule_service: CommandRuleService,

    #[inject(component)]
    command_allow_service: CommandAllowService,

    #[inject(func = create_approved())]
    approved: Arc<DashMap<String, Vec<String>>>,

    #[inject(component)]
    chat_session_cache_manager: ChatSessionCacheManager,

    #[inject(func = create_pendings())]
    pendings: Arc<DashMap<String, Arc<ToolConflictPending>>>,
}

#[async_trait]
impl BaseApp for ToolConflictManager {
    fn name(&self) -> &'static str {
        "ToolConflictManager"
    }
}

impl ToolConflictManager {
    pub async fn confirm_before_execute<A: Serialize>(&self, context: &AiRunContext, call_id: &str, tool_id: &str, args: &A) -> Result<(), AiToolError> {
        let confirm_args = serde_json::to_value(args).map_err(|error| AiToolError::Execution(error.to_string()))?;
        let subject = ConflictSubject::from_args(&confirm_args);
        let config = self.config_service.select_config().await;
        let prefix_rule = CommandPrefixRule::from_config(config.as_ref());
        let prefix = Self::batch_prefix(&prefix_rule, &subject);
        let token_list: Vec<Vec<String>> = subject.shell_commands.iter().map(|command| prefix_rule.tokens_of(command)).collect();
        // 批量整批只按这一批的 hash 记住放行, 不把其中某条的前缀记成长期规则
        let command_tokens = if token_list.len() == 1 { token_list[0].clone() } else { Vec::new() };
        let session_key = subject.session_prefix_key(tool_id, prefix.as_deref());
        let extra_danger = config.as_ref().map(|config| danger_word::split_words(&config.danger_words)).unwrap_or_default();

        let strictest = token_list.iter().filter_map(|tokens| self.command_rule_service.evaluate(tokens, &subject.allow_device_ids())).max_by_key(|hit| hit.decision);
        let rule_reason = match strictest {
            Some(hit) if hit.decision == RULE_DECISION_FORBIDDEN => {
                let reason = Self::rule_reason("forbidden by command rule", &hit);
                warn!("[ai] 命令规则禁止 tool_id={} call_id={} rule_id={} prefix={}", tool_id, call_id, hit.rule_id, hit.matched_prefix);
                self.emit_conflict(context, call_id, tool_id, confirm_args, false, Some(reason.clone()), prefix);
                self.emit_answered(context, call_id, tool_id, AgentAnswerCode::Reject);
                return Err(AiToolError::Denied(reason));
            }
            Some(hit) if hit.decision == RULE_DECISION_ALLOW => {
                info!("[ai] 命令规则放行 tool_id={} call_id={} rule_id={} prefix={}", tool_id, call_id, hit.rule_id, hit.matched_prefix);
                self.emit_conflict(context, call_id, tool_id, confirm_args, true, None, prefix);
                return Ok(());
            }
            Some(hit) => Some(Self::rule_reason("command rule requires confirmation", &hit)),
            None => None,
        };

        if self.resolve_confirm_mode(&context.req_id) == ConfirmMode::Silent && !(self.silent_asks_user(&context.req_id) && Self::hits_danger_word(&subject, &extra_danger)) {
            return self.guard_by_danger_word(context, call_id, tool_id, confirm_args, &subject, &extra_danger, prefix);
        }

        if self.is_remembered(tool_id, &subject) {
            info!("[ai] 命中已记住的放行 tool_id={} call_id={}", tool_id, call_id);
            self.emit_conflict(context, call_id, tool_id, confirm_args, true, None, prefix);
            return Ok(());
        }

        if self.is_session_allowed(&context.req_id, &session_key) && !Self::hits_danger_word(&subject, &extra_danger) {
            info!("[ai] 命中本会话同前缀放行 tool_id={} call_id={} prefix={:?}", tool_id, call_id, prefix);
            self.emit_conflict(context, call_id, tool_id, confirm_args, true, None, prefix);
            return Ok(());
        }

        let pending = Arc::new(ToolConflictPending {
            tool_id: tool_id.to_string(),
            chat_id: self.resolve_chat_id(&context.req_id),
            session_key,
            command_tokens,
            subject,
            inbox: Arc::new(ToolConflictInbox::new()),
        });
        self.pendings.insert(call_id.to_string(), Arc::clone(&pending));
        let decided = self.decide(context, call_id, &pending, confirm_args, rule_reason, prefix, &extra_danger).await;
        self.pendings.remove(call_id);
        decided
    }

    pub fn record_auto_allowed<A: Serialize>(&self, context: &AiRunContext, call_id: &str, tool_id: &str, args: &A) -> Result<(), AiToolError> {
        let args = serde_json::to_value(args).map_err(|error| AiToolError::Execution(error.to_string()))?;
        info!("[ai] 白名单自动放行 tool_id={} call_id={}", tool_id, call_id);
        self.emit_conflict(context, call_id, tool_id, args, true, None, None);
        Ok(())
    }

    pub fn forget_session(&self, chat_id: &str) {
        self.session_allows.forget(chat_id);
        self.approved.remove(chat_id);
    }

    fn record_approved(&self, pending: &ToolConflictPending) {
        let Some(chat_id) = pending.chat_id.as_deref() else {
            return;
        };
        let mut log = self.approved.entry(chat_id.to_string()).or_default();
        log.push(format!("{}: {}", pending.tool_id, pending.subject.command_text));
        if log.len() > APPROVED_LOG_MAX {
            let overflow = log.len() - APPROVED_LOG_MAX;
            log.drain(..overflow);
        }
    }

    fn approved_log(&self, req_id: &str) -> Vec<String> {
        self.resolve_chat_id(req_id).and_then(|chat_id| self.approved.get(&chat_id).map(|log| log.clone())).unwrap_or_default()
    }

    fn guard_by_danger_word(
        &self,
        context: &AiRunContext,
        call_id: &str,
        tool_id: &str,
        args: Value,
        subject: &ConflictSubject,
        extra_danger: &[String],
        prefix: Option<String>,
    ) -> Result<(), AiToolError> {
        let Some(word) = Self::danger_word_hit(subject, extra_danger) else {
            self.emit_conflict(context, call_id, tool_id, args, true, None, prefix);
            return Ok(());
        };
        let reason = format!("this step matched the dangerous-operation keyword \"{}\"; silent background execution only allows read-only actions, so it was skipped", word);
        warn!("[ai] 危险关键字拦截 tool_id={} call_id={} word={}", tool_id, call_id, word);
        self.emit_conflict(context, call_id, tool_id, args, false, Some(reason.clone()), prefix);
        self.emit_answered(context, call_id, tool_id, AgentAnswerCode::Reject);
        Err(AiToolError::Denied(reason))
    }

    fn hits_danger_word(subject: &ConflictSubject, extra_danger: &[String]) -> bool {
        Self::danger_word_hit(subject, extra_danger).is_some()
    }

    fn danger_word_hit(subject: &ConflictSubject, extra_danger: &[String]) -> Option<String> {
        subject.shell_commands.iter().find_map(|command| danger_word::match_danger(command, extra_danger))
    }

    fn batch_prefix(prefix_rule: &CommandPrefixRule, subject: &ConflictSubject) -> Option<String> {
        let mut prefixes: Vec<String> = Vec::new();
        for command in &subject.shell_commands {
            let prefix = prefix_rule.prefix_of(command);
            if !prefixes.contains(&prefix) {
                prefixes.push(prefix);
            }
        }
        if prefixes.is_empty() {
            return None;
        }
        Some(prefixes.join(" + "))
    }

    fn is_read_only_tool(&self, req_id: &str, tool_id: &str) -> bool {
        let Some(entry) = self.chat_session_cache_manager.get(req_id) else {
            return false;
        };
        entry.tools.iter().any(|tool| tool.tool_id().as_str() == tool_id && tool.read_only())
    }

    fn is_session_allowed(&self, req_id: &str, session_key: &str) -> bool {
        self.resolve_chat_id(req_id).is_some_and(|chat_id| self.session_allows.is_allowed(&chat_id, session_key))
    }

    pub async fn resolve(&self, call_id: &str, code: AgentAnswerCode) -> Result<()> {
        let Some(pending) = self.pendings.get(call_id).map(|entry| entry.value().clone()) else {
            bail!("unknown tool conflict call_id: {}", call_id);
        };
        if code == AgentAnswerCode::AllowRemember {
            if let Err(error) = self.remember(&pending).await {
                warn!("[ai] 记住放行失败 call_id={} error={:#}", call_id, error);
            }
        }
        if code == AgentAnswerCode::AllowSessionPrefix {
            if let Some(chat_id) = pending.chat_id.as_deref() {
                self.session_allows.remember(chat_id, &pending.session_key);
            }
        }
        if code.is_allow() {
            self.record_approved(&pending);
        }
        pending.push_answer(call_id, code).await;
        if code == AgentAnswerCode::AllowRemember {
            self.fan_out_remembered(call_id, &pending.match_key(), |entry| entry.match_key()).await;
        }
        if code == AgentAnswerCode::AllowSessionPrefix {
            let chat_id = pending.chat_id.clone();
            self.fan_out_remembered(call_id, &pending.session_key, move |entry| if entry.chat_id == chat_id { entry.session_key.clone() } else { String::new() }).await;
        }
        Ok(())
    }

    async fn fan_out_remembered(&self, source_call_id: &str, match_key: &str, key_of: impl Fn(&ToolConflictPending) -> String) {
        let targets: Vec<(String, Arc<ToolConflictPending>)> =
            self.pendings.iter().filter(|entry| entry.key().as_str() != source_call_id && key_of(entry.value()) == match_key).map(|entry| (entry.key().clone(), entry.value().clone())).collect();
        for (call_id, pending) in targets {
            pending.push_answer(&call_id, AgentAnswerCode::Allow).await;
            info!("[ai] 记住允许已广播放行 call_id={}", call_id);
        }
    }

    async fn remember(&self, pending: &ToolConflictPending) -> Result<()> {
        let subject = &pending.subject;
        for device_id in subject.allow_device_ids() {
            if pending.command_tokens.is_empty() {
                self.command_allow_service.remember(&device_id, &pending.tool_id, &subject.command_hash, &subject.command_text).await?;
            } else {
                self.command_rule_service.remember_allow(pending.command_tokens.clone(), device_id).await?;
            }
        }
        info!("[ai] 已记住放行 tool_id={} hash={} tokens={:?}", pending.tool_id, subject.command_hash, pending.command_tokens);
        Ok(())
    }

    fn rule_reason(head: &str, hit: &RuleHit) -> String {
        if hit.justification.is_empty() {
            return format!("{}: `{}`", head, hit.matched_prefix);
        }
        format!("{}: `{}` — {}", head, hit.matched_prefix, hit.justification)
    }

    fn is_remembered(&self, tool_id: &str, subject: &ConflictSubject) -> bool {
        subject.allow_device_ids().iter().all(|device_id| self.command_allow_service.is_allowed(device_id, tool_id, &subject.command_hash))
    }

    fn resolve_confirm_mode(&self, req_id: &str) -> ConfirmMode {
        match self.chat_session_cache_manager.confirm_mode(req_id) {
            Some(mode) => mode,
            None => ConfirmMode::Manual,
        }
    }

    fn silent_asks_user(&self, req_id: &str) -> bool {
        self.chat_session_cache_manager.get(req_id).is_some_and(|entry| entry.context.origin.attended())
    }

    async fn audit(&self, context: &AiRunContext, call_id: &str, tool_id: &str, args: &Value) -> AuditOutcome {
        let approved = self.approved_log(&context.req_id);
        match self.flow_command_audit.audit(context, tool_id, args, &approved).await {
            Ok(verdict) => {
                if verdict.allow {
                    return AuditOutcome::Allow(if verdict.checked.is_empty() { None } else { Some(verdict.checked) });
                }
                warn!("[ai] 自动审核未通过 tool_id={} call_id={} reason={} checked={}", tool_id, call_id, verdict.reason, verdict.checked);
                AuditOutcome::Escalate(verdict.reason)
            }
            Err(error) => {
                warn!("[ai] 自动审核执行失败 tool_id={} call_id={} error={:#}", tool_id, call_id, error);
                AuditOutcome::Escalate(format!("auto audit failed: {}", error))
            }
        }
    }
}
