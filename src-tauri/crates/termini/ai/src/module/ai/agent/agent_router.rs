use summer::plugin::Service;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use termini_macros::base_app;
use crate::base::base_app::BaseApp;
use crate::module::ai::provide::ai_agent::AiAgent;
use crate::service::service_ai_chat::AiChatService;
use crate::module::ai::stream::stream_output::StreamOutput;
use crate::module::ai::provide::provide_factory::ProvideFactory;
use crate::module::ai::agent::agent_base::{AgentBase, AgentRole};
use crate::module::ai::dto::{AiRunEmit, RouterDecision, TaskKind};
use crate::module::ai::agent::agent_build::build_agent_no_thinking;
use crate::module::ai::shared::{parse_embedded_json, ChatSessionCacheManager};
use crate::module::ai::agent::prompts::{render, LEAD_ROUTER_SYSTEM, LEAD_ROUTER_TITLE};

const TITLE_FIELD: &str = ", \"title\": string";

#[base_app(sort = 766)]
#[derive(Clone, Service)]
pub struct AgentRouter {
    #[inject(component)]
    ai_chat_service: AiChatService,

    #[inject(component)]
    provide_factory: ProvideFactory,

    #[inject(component)]
    chat_session_cache_manager: ChatSessionCacheManager,
}

#[async_trait]
impl BaseApp for AgentRouter {
    fn name(&self) -> &'static str {
        "AgentRouter"
    }
}

impl AgentBase for AgentRouter {
    fn role(&self) -> AgentRole {
        AgentRole::Router
    }

    fn max_turns(&self) -> usize {
        2
    }

    fn emit(&self) -> AiRunEmit {
        AiRunEmit { text: false, reasoning: false, pause: false, status: false, proxy_error: false, ..AiRunEmit::default() }
    }

    fn tool_concurrency(&self) -> usize {
        1
    }

    fn build_ai_agent(&self, req_id: &str) -> Result<AiAgent> {
        let entry = self.chat_session_cache_manager.get(req_id).ok_or_else(|| anyhow!("unknown req_id: {}", req_id))?;
        let needs_title = self.ai_chat_service.needs_title(&entry.context.ai_chat_id);
        let (title_rule, title_field) = if needs_title { (LEAD_ROUTER_TITLE, TITLE_FIELD) } else { ("", "") };
        let preamble = render(LEAD_ROUTER_SYSTEM, &[("title_rule", title_rule), ("title_field", title_field)]);
        build_agent_no_thinking(&self.provide_factory, &entry.light_provide(), &preamble, Vec::new())
    }
}

impl AgentRouter {
    pub fn parse_decision(raw: &str) -> RouterDecision {
        #[derive(serde::Deserialize)]
        struct RouterOutput {
            #[serde(default)]
            task_kind: TaskKind,
            #[serde(default)]
            goal: Option<String>,
            #[serde(default)]
            title: Option<String>,
            #[serde(default)]
            language: Option<String>,
        }
        match parse_embedded_json::<RouterOutput>(raw) {
            Some(out) => {
                let goal = Self::clean(out.goal);
                let title = Self::clean(out.title);
                let language = Self::clean(out.language);
                RouterDecision { task_kind: out.task_kind, goal, title, language }
            }
            None => RouterDecision { task_kind: TaskKind::MultiStep, goal: None, title: None, language: None },
        }
    }

    fn clean(value: Option<String>) -> Option<String> {
        value.and_then(|text| {
            let trimmed = text.trim().to_string();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed)
            }
        })
    }

    pub fn parse_task_kind(raw: &str) -> TaskKind {
        Self::parse_decision(raw).task_kind
    }
}

impl StreamOutput for RouterDecision {
    fn from_stream_text(raw: &str) -> Self {
        AgentRouter::parse_decision(raw)
    }
}

impl StreamOutput for TaskKind {
    fn from_stream_text(raw: &str) -> Self {
        AgentRouter::parse_task_kind(raw)
    }
}
