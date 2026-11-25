use summer::plugin::Service;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use termini_macros::base_app;
use crate::base::base_app::BaseApp;
use crate::module::ai::dto::AiRunEmit;
use crate::module::ai::provide::ai_agent::AiAgent;
use crate::module::ai::shared::ChatSessionCacheManager;
use crate::module::ai::agent::prompts::LEAD_DIRECT_SYSTEM;
use crate::module::ai::provide::provide_factory::ProvideFactory;
use crate::module::ai::agent::agent_base::{AgentBase, AgentRole};
use crate::module::ai::agent::agent_build::build_agent_with_level;

#[base_app(sort = 768)]
#[derive(Clone, Service)]
pub struct AgentDirect {
    #[inject(component)]
    provide_factory: ProvideFactory,

    #[inject(component)]
    chat_session_cache_manager: ChatSessionCacheManager,
}

#[async_trait]
impl BaseApp for AgentDirect {
    fn name(&self) -> &'static str {
        "AgentDirect"
    }
}

impl AgentBase for AgentDirect {
    fn role(&self) -> AgentRole {
        AgentRole::Direct
    }

    fn max_turns(&self) -> usize {
        10
    }

    fn emit(&self) -> AiRunEmit {
        AiRunEmit::default()
    }

    fn tool_concurrency(&self) -> usize {
        2
    }

    fn build_ai_agent(&self, req_id: &str) -> Result<AiAgent> {
        let entry = self.chat_session_cache_manager.get(req_id).ok_or_else(|| anyhow!("unknown req_id: {}", req_id))?;
        build_agent_with_level(&self.provide_factory, &entry.ai_provide, LEAD_DIRECT_SYSTEM, vec![], entry.context.thinking_level.as_deref())
    }
}
