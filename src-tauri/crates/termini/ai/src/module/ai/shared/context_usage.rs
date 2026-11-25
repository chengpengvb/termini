use anyhow::Result;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use crate::base::base_app::BaseApp;
use serde::{Deserialize, Serialize};
use crate::service::service_base::BaseService;
use crate::util::token_util::estimate_text_tokens;
use crate::module::ai::tools::tool_base::ToolDomain;
use crate::model::req::req_chat_context::ConfirmMode;
use crate::module::ai::shared::ChatSessionCacheManager;
use crate::module::ai::tools::tool_manager::ToolManager;
use crate::service::service_ai_provide::AiProvideService;
use crate::module::ai::mcp::mcp_client_pool::McpClientPool;
use crate::service::service_ai_chat_memory::AiChatMemoryService;
use crate::module::ai::agent::prompts::{render, LEAD_MAIN_SYSTEM};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextUsage {
    pub live: bool,
    pub total: usize,
    pub summary: usize,
    pub measured: usize,
    pub mcp_tools: usize,
    pub token_limit: usize,
    pub conversation: usize,
    pub system_prompt: usize,
    pub tool_definitions: usize,
}

#[base_app(sort = 774)]
#[derive(Clone, Service)]
pub struct ContextUsageService {
    #[inject(component)]
    tool_manager: ToolManager,

    #[inject(component)]
    mcp_client_pool: McpClientPool,

    #[inject(component)]
    ai_provide_service: AiProvideService,

    #[inject(component)]
    ai_chat_memory_service: AiChatMemoryService,

    #[inject(component)]
    chat_session_cache_manager: ChatSessionCacheManager,
}

#[async_trait]
impl BaseApp for ContextUsageService {
    fn name(&self) -> &'static str {
        "ContextUsageService"
    }
}

impl ContextUsageService {
    pub async fn usage(&self, chat_id: &str, provide_id: Option<&str>) -> Result<ContextUsage> {
        let measured = self.chat_session_cache_manager.context_tokens(chat_id) as usize;
        let (messages, summary, token_limit, confirm_mode, live, tools) = match self.chat_session_cache_manager.get_req_id_by_chat_id(chat_id).and_then(|req_id| {
            self.chat_session_cache_manager
                .get(&req_id)
                .map(|entry| (entry.messages_without_addon(), entry.summary.lock().clone(), *entry.token_limit.lock(), *entry.confirm_mode.borrow(), true, entry.tools.clone()))
        }) {
            Some(live) => live,
            None => {
                let loaded = self.ai_chat_memory_service.load(chat_id.to_string()).await?;
                let token_limit = match provide_id {
                    Some(id) => match self.ai_provide_service.find_by_id(id.to_string()).await? {
                        Some(shared) => self.ai_provide_service.resolve_token_limit(&shared.read().clone()).unwrap_or(0),
                        None => 0,
                    },
                    None => 0,
                };
                let tools = self.tool_manager.tools().into_iter().filter(|tool| tool.domains().contains(&ToolDomain::Main)).collect();
                (loaded.messages(), loaded.summary, token_limit, ConfirmMode::default(), false, tools)
            }
        };

        let system_prompt = estimate_text_tokens(&render(LEAD_MAIN_SYSTEM, &[("confirm_mode", confirm_mode.prompt_hint())]));
        let tool_definitions: usize = tools
            .iter()
            .filter(|tool| !tool.deferred())
            .map(|tool| {
                let rig = tool.as_rig_tool(None);
                let definition = rig.definition();
                Self::definition_tokens(&definition.name, &definition.description, &definition.parameters)
            })
            .sum();
        let mcp_tools: usize = self
            .mcp_client_pool
            .collect_rig_tools()
            .iter()
            .map(|tool| {
                let definition = tool.definition();
                Self::definition_tokens(&definition.name, &definition.description, &definition.parameters)
            })
            .sum();
        let summary_tokens = estimate_text_tokens(&summary);
        let conversation = AiChatMemoryService::estimate_messages_tokens(&messages);
        let total = system_prompt + tool_definitions + mcp_tools + summary_tokens + conversation;
        Ok(ContextUsage { token_limit, system_prompt, tool_definitions, mcp_tools, summary: summary_tokens, conversation, total, measured, live })
    }

    fn definition_tokens(name: &str, description: &str, parameters: &serde_json::Value) -> usize {
        estimate_text_tokens(name) + estimate_text_tokens(description) + estimate_text_tokens(&parameters.to_string())
    }
}
