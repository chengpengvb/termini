use summer::plugin::Service;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use termini_macros::base_app;
use std::collections::HashSet;
use crate::base::base_app::BaseApp;
use crate::module::ai::dto::AiRunEmit;
use crate::module::ai::shared::ToolCallDonePush;
use crate::module::ai::provide::ai_agent::AiAgent;
use crate::module::ai::shared::ToolActivationStore;
use crate::module::ai::tools::base::ToolBaseCollect;
use crate::module::ai::tools::tool_base::ToolDomain;
use crate::module::ai::shared::ChatSessionCacheManager;
use crate::module::ai::mcp::mcp_client_pool::McpClientPool;
use crate::module::ai::provide::provide_factory::ProvideFactory;
use crate::module::ai::agent::agent_base::{AgentBase, AgentRole};
use crate::module::ai::agent::prompts::{render, LEAD_MAIN_SYSTEM};
use crate::module::ai::agent::agent_build::build_agent_with_level_handle;

#[base_app(sort = 767)]
#[derive(Clone, Service)]
pub struct AgentMain {
    #[inject(component)]
    mcp_client_pool: McpClientPool,

    #[inject(component)]
    provide_factory: ProvideFactory,

    #[inject(component)]
    tool_call_done_push: ToolCallDonePush,

    #[inject(component)]
    tool_activation_store: ToolActivationStore,

    #[inject(component)]
    chat_session_cache_manager: ChatSessionCacheManager,
}

#[async_trait]
impl BaseApp for AgentMain {
    fn name(&self) -> &'static str {
        "AgentMain"
    }
}

impl AgentBase for AgentMain {
    fn role(&self) -> AgentRole {
        AgentRole::Main
    }

    fn max_turns(&self) -> usize {
        100
    }

    fn emit(&self) -> AiRunEmit {
        AiRunEmit::default()
    }

    fn tool_concurrency(&self) -> usize {
        4
    }

    fn build_ai_agent(&self, req_id: &str) -> Result<AiAgent> {
        let entry = self.chat_session_cache_manager.get(req_id).ok_or_else(|| anyhow!("unknown req_id: {}", req_id))?;
        let preamble = render(LEAD_MAIN_SYSTEM, &[("confirm_mode", entry.confirm_mode.borrow().prompt_hint())]);
        let handle = match self.tool_activation_store.handle(req_id) {
            Some(existing) => existing,
            None => {
                let tools = ToolBaseCollect::collect(&entry.tools, &self.mcp_client_pool, ToolDomain::Main, &self.tool_call_done_push);
                let builtin_names: HashSet<String> = entry.tools.iter().filter(|tool| tool.domains().contains(&ToolDomain::Main)).map(|tool| tool.tool_id().as_str().to_string()).collect();
                self.tool_activation_store.bind_handle(req_id, tools, builtin_names)
            }
        };
        build_agent_with_level_handle(&self.provide_factory, &entry.ai_provide, &preamble, handle, entry.context.thinking_level.as_deref())
    }
}
