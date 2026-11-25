use anyhow::Result;
use serde::Deserialize;
use summer::tracing::warn;
use rig::tool::DynamicTool;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use crate::base::base_app::BaseApp;
use crate::model::ai_provide::AiProvide;
use crate::module::ai::provide::ai_agent::AiAgent;
use crate::module::ai::shared::parse_embedded_json;
use crate::util::json_util::parse_string_array_prefix;
use crate::module::ai::provide::provide_factory::ProvideFactory;
use crate::module::ai::agent::agent_build::build_agent_no_thinking;
use crate::module::ai::agent::prompts::{render, COMMAND_SUGGEST_SYSTEM, COMMAND_SUGGEST_USER};

const SUGGEST_ITEM_MAX_CHARS: usize = 300;
const SUGGEST_BLOCK_MAX_CHARS: usize = 4000;
const SUGGEST_KEYS: &[&str] = &["suggest_list", "suffix_list"];

#[base_app(sort = 768)]
#[derive(Clone, Service)]
pub struct AgentSuggest {
    #[inject(component)]
    provide_factory: ProvideFactory,
}

#[async_trait]
impl BaseApp for AgentSuggest {
    fn name(&self) -> &'static str {
        "AgentSuggest"
    }
}

#[derive(Debug, Deserialize)]
struct CommandSuggestOutput {
    #[serde(default, alias = "suffix_list")]
    suggest_list: Vec<String>,
}

impl AgentSuggest {
    pub const MAX_TURNS: usize = 4;

    pub fn build_ai_agent(&self, provide: &AiProvide, tools: Vec<DynamicTool>) -> Result<AiAgent> {
        build_agent_no_thinking(&self.provide_factory, provide, COMMAND_SUGGEST_SYSTEM, tools)
    }

    pub fn render_user_prompt(vars: &[(&str, &str)]) -> String {
        render(COMMAND_SUGGEST_USER, vars)
    }

    pub fn parse_list(raw: &str, count: usize) -> Vec<String> {
        let list = Self::extract_list(raw, count);
        if list.is_empty() {
            warn!("[ai] 候选解析失败 raw={}", Self::truncate_tail(raw, 400));
        }
        list
    }

    pub fn extract_list(raw: &str, count: usize) -> Vec<String> {
        let items = match parse_embedded_json::<CommandSuggestOutput>(raw) {
            Some(output) => output.suggest_list,
            None => match parse_string_array_prefix(raw, SUGGEST_KEYS) {
                Some(items) => items,
                None => return vec![],
            },
        };
        Self::take_clean(items, count)
    }

    fn take_clean(items: Vec<String>, count: usize) -> Vec<String> {
        let mut seen: Vec<String> = Vec::new();
        for item in items {
            let cleaned = item.replace('\r', "").trim_end().to_string();
            if cleaned.trim().is_empty() {
                continue;
            }
            let max_chars = if cleaned.contains('\n') { SUGGEST_BLOCK_MAX_CHARS } else { SUGGEST_ITEM_MAX_CHARS };
            if cleaned.chars().count() > max_chars {
                continue;
            }
            if seen.contains(&cleaned) {
                continue;
            }
            seen.push(cleaned);
            if seen.len() >= count {
                break;
            }
        }
        seen
    }

    fn truncate_tail(input: &str, max_chars: usize) -> String {
        let count = input.chars().count();
        if count <= max_chars {
            return input.to_string();
        }
        input.chars().skip(count - max_chars).collect()
    }
}
