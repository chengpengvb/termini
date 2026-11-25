use anyhow::Result;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use crate::base::base_app::BaseApp;
use crate::model::ai_provide::AiProvide;
use crate::module::ai::provide::ai_agent::AiAgent;
use crate::module::ai::provide::provide_factory::ProvideFactory;
use crate::module::ai::agent::agent_build::build_agent_no_thinking;
use crate::module::ai::agent::prompts::{render, LEAD_FOLD_SYSTEM, LEAD_FOLD_USER};

pub const FOLD_MAX_CHARS: usize = 3000;

#[base_app(sort = 768)]
#[derive(Clone, Service)]
pub struct AgentFold {
    #[inject(component)]
    provide_factory: ProvideFactory,
}

#[async_trait]
impl BaseApp for AgentFold {
    fn name(&self) -> &'static str {
        "AgentFold"
    }
}

impl AgentFold {
    pub fn build_ai_agent(&self, provide: &AiProvide) -> Result<AiAgent> {
        let system = render(LEAD_FOLD_SYSTEM, &[("fold_max_chars", &FOLD_MAX_CHARS.to_string())]);
        build_agent_no_thinking(&self.provide_factory, provide, &system, vec![])
    }

    pub fn render_user_prompt(summary: &str, transcript: &str) -> String {
        render(LEAD_FOLD_USER, &[("summary", summary), ("transcript", transcript)])
    }
}
