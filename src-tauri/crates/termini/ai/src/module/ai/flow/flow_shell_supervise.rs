use std::sync::Arc;
use std::time::Duration;
use rig::tool::ToolContext;
use summer::plugin::Service;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use termini_macros::base_app;
use crate::base::base_app::BaseApp;
use crate::module::ai::dto::AiRunContext;
use crate::module::ai::dto::ShellSuperviseDecision;
use crate::module::ai::shared::ChatSessionCacheManager;
use crate::module::ai::agent::agent_supervise::AgentSupervise;
use crate::module::ai::agent::prompts::output_language_addon_text;
use crate::module::ai::agent::agent_build::prompt_agent_with_extensions;
use crate::module::ai::tools::common::shell::local_shell_registry::ShellProcessEntry;

pub const SUPERVISE_FAIL_MAX: usize = 3;
pub const SUPERVISE_MAX_TURNS: usize = 6;
pub const SUPERVISE_WAIT_MIN_SECS: u64 = 1;
pub const SUPERVISE_WAIT_MAX_SECS: u64 = 300;
pub const SUPERVISE_TOTAL_MAX_SECS: i64 = 1800;
pub const SUPERVISE_PROMPT_TAIL_LINES: usize = 60;
pub const SUPERVISE_FIRST_WAIT: Duration = Duration::from_secs(2);
pub const SUPERVISE_FAIL_RETRY_WAIT: Duration = Duration::from_secs(10);

#[base_app(sort = 771)]
#[derive(Clone, Service)]
pub struct FlowShellSupervise {
    #[inject(component)]
    agent_supervise: AgentSupervise,

    #[inject(component)]
    chat_session_cache_manager: ChatSessionCacheManager,
}

#[async_trait]
impl BaseApp for FlowShellSupervise {
    fn name(&self) -> &'static str {
        "FlowShellSupervise"
    }
}

impl FlowShellSupervise {
    pub async fn decide(&self, context: &Arc<AiRunContext>, entry: &ShellProcessEntry, shell_id: &str, round: usize, since_seq: u64) -> Result<ShellSuperviseDecision> {
        let goal = self.chat_session_cache_manager.scene(&context.req_id).and_then(|scene| scene.goal);
        self.decide_prompt(context, AgentSupervise::build_user_prompt(entry, shell_id, round, since_seq, SUPERVISE_PROMPT_TAIL_LINES, &context.prompt, goal.as_deref())).await
    }

    pub async fn decide_prompt(&self, context: &Arc<AiRunContext>, job_prompt: String) -> Result<ShellSuperviseDecision> {
        let Some(session) = self.chat_session_cache_manager.get(&context.req_id) else {
            return Err(anyhow!("unknown req_id: {}", context.req_id));
        };
        let provide = session.light_provide();
        drop(session);

        let agent = self.agent_supervise.build_ai_agent(&provide)?;
        let Some(language_text) = output_language_addon_text(&context.addon_messages) else {
            return Err(anyhow!("output language addon is missing"));
        };
        let prompt = format!("{}\n\n{}", language_text, job_prompt);
        let mut tool_context = ToolContext::new();
        tool_context.insert(Arc::clone(context));
        let raw = prompt_agent_with_extensions(agent, prompt, SUPERVISE_MAX_TURNS, tool_context).await?;
        let Some(decision) = AgentSupervise::parse_decision(&raw) else {
            return Err(anyhow!("supervise model returned no parseable decision"));
        };
        Ok(decision)
    }
}
