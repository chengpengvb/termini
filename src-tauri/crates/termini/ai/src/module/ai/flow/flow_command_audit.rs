use std::sync::Arc;
use serde_json::Value;
use parking_lot::Mutex;
use anyhow::{bail, Result};
use summer::plugin::Service;
use termini_macros::base_app;
use crate::base::base_app::BaseApp;
use crate::module::ai::shared::slim_history;
use crate::service::service_config::ConfigService;
use crate::module::ai::agent::agent_base::AgentBase;
use crate::module::ai::agent::agent_audit::AgentAudit;
use crate::module::ai::shared::ChatSessionCacheManager;
use crate::module::ai::stream::stream_runner::AiStreamRunner;
use crate::module::ai::agent::prompts::output_language_addons;
use crate::module::ai::dto::{AiRunContext, AuditVerdict, SharedMemory};

const AUDIT_HISTORY_MAX: usize = 12;

#[base_app(sort = 769)]
#[derive(Clone, Service)]
pub struct FlowCommandAudit {
    #[inject(component)]
    agent_audit: AgentAudit,

    #[inject(component)]
    config_service: ConfigService,

    #[inject(component)]
    stream_runner: AiStreamRunner,

    #[inject(component)]
    chat_session_cache_manager: ChatSessionCacheManager,
}

impl BaseApp for FlowCommandAudit {
    fn name(&self) -> &'static str {
        "FlowCommandAudit"
    }
}

impl FlowCommandAudit {
    pub async fn audit(&self, context: &AiRunContext, tool_id: &str, args: &Value, approved: &[String]) -> Result<AuditVerdict> {
        let memory: SharedMemory = Arc::new(Mutex::new(slim_history(&context.memory.lock(), AUDIT_HISTORY_MAX)));
        let agent = self.agent_audit.build_ai_agent(&context.req_id)?;
        let policy = self.config_service.select_config().await.and_then(|config| config.audit_policy);
        let addon_messages = output_language_addons(&context.addon_messages);
        if addon_messages.is_empty() {
            bail!("output language addon is missing");
        }

        let emit = self.agent_audit.emit();
        let req_id = context.req_id.clone();
        let builder = self.agent_audit.clone();
        let max_turns = self.agent_audit.max_turns();
        let tool_concurrency = self.agent_audit.tool_concurrency();
        let goal = self.chat_session_cache_manager.scene(&context.req_id).and_then(|scene| scene.goal);
        let prompt = Self::build_prompt(tool_id, args, policy.as_deref(), approved, &context.prompt, goal.as_deref());
        let rebuild_agent = Arc::new(move |id: &str| builder.build_ai_agent(id));
        let run_context =
            Arc::new(AiRunContext::new(req_id, agent, prompt, emit, max_turns, memory, tool_concurrency, context.chat_id.clone(), rebuild_agent, addon_messages, context.cancel_token.clone()));
        self.stream_runner.run_stream::<AuditVerdict>(run_context, self.agent_audit.role()).await
    }

    fn build_prompt(tool_id: &str, args: &Value, policy: Option<&str>, approved: &[String], user_prompt: &str, goal: Option<&str>) -> String {
        let args_text = serde_json::to_string_pretty(args).unwrap_or_else(|_| args.to_string());
        let mut prompt = crate::module::ai::shared::chat_item_format::intent_prefix(user_prompt, goal);
        if let Some(policy) = policy.map(str::trim).filter(|value| !value.is_empty()) {
            prompt.push_str(&format!("[审核策略]\n{}\n\n", policy));
        }
        if !approved.is_empty() {
            prompt.push_str(&format!("[本轮已批准]\n{}\n\n", approved.join("\n")));
        }
        prompt.push_str(&format!("[待审核操作]\n工具: {}\n参数:\n{}", tool_id, args_text));
        prompt
    }
}
