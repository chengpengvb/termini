use summer::plugin::Service;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use termini_macros::base_app;
use crate::base::base_app::BaseApp;
use crate::module::ai::dto::AiRunEmit;
use crate::module::ai::shared::ToolCallDonePush;
use crate::module::ai::provide::ai_agent::AiAgent;
use crate::module::ai::tools::base::ToolBaseCollect;
use crate::module::ai::tools::tool_base::ToolDomain;
use crate::module::ai::agent::agent_build::build_agent;
use crate::module::ai::shared::ChatSessionCacheManager;
use crate::module::ai::agent::prompts::LEAD_ALERT_SYSTEM;
use crate::module::ai::mcp::mcp_client_pool::McpClientPool;
use crate::module::ai::provide::provide_factory::ProvideFactory;
use crate::module::ai::agent::agent_base::{AgentBase, AgentRole};

#[base_app(sort = 767)]
#[derive(Clone, Service)]
pub struct AgentAlert {
    #[inject(component)]
    mcp_client_pool: McpClientPool,

    #[inject(component)]
    provide_factory: ProvideFactory,

    #[inject(component)]
    tool_call_done_push: ToolCallDonePush,

    #[inject(component)]
    chat_session_cache_manager: ChatSessionCacheManager,
}

#[async_trait]
impl BaseApp for AgentAlert {
    fn name(&self) -> &'static str {
        "AgentAlert"
    }
}

impl AgentBase for AgentAlert {
    fn role(&self) -> AgentRole {
        AgentRole::Alert
    }

    fn max_turns(&self) -> usize {
        50
    }

    fn emit(&self) -> AiRunEmit {
        AiRunEmit { status: false, ..AiRunEmit::default() }
    }

    fn tool_concurrency(&self) -> usize {
        3
    }

    fn build_ai_agent(&self, req_id: &str) -> Result<AiAgent> {
        let entry = self.chat_session_cache_manager.get(req_id).ok_or_else(|| anyhow!("unknown req_id: {}", req_id))?;
        let tools = ToolBaseCollect::collect(&entry.tools, &self.mcp_client_pool, ToolDomain::Alert, &self.tool_call_done_push);
        build_agent(&self.provide_factory, &entry.ai_provide, LEAD_ALERT_SYSTEM, tools)
    }
}
