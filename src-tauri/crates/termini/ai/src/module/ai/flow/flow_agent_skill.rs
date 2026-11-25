use std::sync::Arc;
use dashmap::DashMap;
use serde::Deserialize;
use summer::tracing::warn;
use rig::tool::DynamicTool;
use summer::plugin::Service;
use anyhow::{anyhow, Result};
use termini_macros::base_app;
use crate::base::base_app::BaseApp;
use crate::model::ai_skill::AiSkill;
use crate::model::ai_provide::AiProvide;
use tokio_util::sync::CancellationToken;
use crate::service::service_base::BaseService;
use crate::model::resp::resp_model::ResultCode;
use crate::service::service_config::ConfigService;
use crate::module::ai::tools::tool_manager::ToolManager;
use crate::service::service_ai_provide::AiProvideService;
use crate::service::service_ai_chat_item::AiChatItemService;
use crate::module::ai::provide::provide_factory::ProvideFactory;
use crate::module::ai::tools::base::{ToolBaseOutWrap, TOOL_OUTPUT_MAX_TOKENS};
use crate::module::ai::tools::common::tool_chat_history::ToolChatHistoryBound;
use crate::module::ai::agent::agent_build::{build_agent, prompt_agent_with_turns};
use crate::module::ai::tools::suggest::tool_suggest_web_fetch::ToolSuggestWebFetch;
use crate::module::ai::tools::suggest::tool_suggest_web_search::ToolSuggestWebSearch;
use crate::module::ai::agent::prompts::{output_language_text, render, LEAD_SKILL_EXTRACT_SYSTEM, LEAD_SKILL_EXTRACT_USER};

const MAX_TURNS: usize = 100;

fn create_cancel_tasks() -> Arc<DashMap<String, CancellationToken>> {
    Arc::new(DashMap::new())
}

#[base_app(sort = 770)]
#[derive(Clone, Service)]
pub struct FlowAgentSkill {
    #[inject(component)]
    tool_manager: ToolManager,

    #[inject(component)]
    config_service: ConfigService,

    #[inject(component)]
    provide_factory: ProvideFactory,

    #[inject(component)]
    ai_provide_service: AiProvideService,

    #[inject(component)]
    ai_chat_item_service: AiChatItemService,

    #[inject(func = create_cancel_tasks())]
    cancel_tasks: Arc<DashMap<String, CancellationToken>>,
}

impl BaseApp for FlowAgentSkill {
    fn name(&self) -> &'static str {
        "FlowAgentSkill"
    }
}

#[derive(Debug, Deserialize)]
struct SkillExtractOutput {
    #[serde(default = "default_sort", deserialize_with = "deserialize_sort_flex")]
    sort: i16,
    #[serde(alias = "名称")]
    name: String,

    #[serde(alias = "内容")]
    content: String,

    #[serde(default, alias = "说明")]
    description: String,
}

fn default_sort() -> i16 {
    100
}

fn deserialize_sort_flex<'de, D>(deserializer: D) -> Result<i16, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum SortValue {
        Num(i64),
        Str(String),
        Null,
    }
    match SortValue::deserialize(deserializer)? {
        SortValue::Num(n) => Ok(n.clamp(0, i16::MAX as i64) as i16),
        SortValue::Str(s) => Ok(s.trim().parse::<i16>().unwrap_or(100)),
        SortValue::Null => Ok(100),
    }
}

impl FlowAgentSkill {
    pub fn begin_task(&self, task_id: &str) -> Result<CancellationToken> {
        if task_id.trim().is_empty() {
            return Err(ResultCode::SkillTaskIdEmpty.into());
        }
        if self.cancel_tasks.contains_key(task_id) {
            return Err(ResultCode::SkillTaskRunning.into());
        }
        let token = CancellationToken::new();
        self.cancel_tasks.insert(task_id.to_string(), token.clone());
        Ok(token)
    }

    pub fn cancel_task(&self, task_id: &str) -> Result<()> {
        let Some((_, token)) = self.cancel_tasks.remove(task_id) else {
            return Err(ResultCode::SkillTaskNotFound.into());
        };
        token.cancel();
        Ok(())
    }

    pub fn end_task(&self, task_id: &str) {
        self.cancel_tasks.remove(task_id);
    }

    pub async fn extract(
        &self,
        chat_id: Option<String>,
        ai_provide_id: String,
        key_points: Option<String>,
        name: Option<String>,
        description: Option<String>,
        content: Option<String>,
        cancel: CancellationToken,
    ) -> Result<AiSkill> {
        self.ensure_not_cancelled(&cancel)?;
        let chat_id = normalize_optional_text(chat_id);
        let key_points = normalize_optional_text(key_points);
        let name = normalize_optional_text(name);
        let description = normalize_optional_text(description);
        let content = normalize_optional_text(content);
        if chat_id.is_none() && key_points.is_none() {
            return Err(ResultCode::SkillExtractInputRequired.into());
        }
        let catalog = self.build_tool_catalog();
        let provide = self.resolve_provide(ai_provide_id).await?;
        let tools = self.build_callable_tools(chat_id.clone());
        let language = self.config_service.select_system_language().await;
        let prompt =
            format!("{}\n\n{}", output_language_text(language), build_user_prompt(chat_id.is_some(), key_points.as_deref(), name.as_deref(), description.as_deref(), content.as_deref(), &catalog));
        let agent = build_agent(&self.provide_factory, &provide, LEAD_SKILL_EXTRACT_SYSTEM, tools)?;
        let raw = tokio::select! {
            _ = cancel.cancelled() => {
                return Err(ResultCode::Cancelled.into());
            }
            result = prompt_agent_with_turns(agent, prompt, MAX_TURNS) => result?,
        };
        self.parse_skill(raw)
    }

    fn ensure_not_cancelled(&self, cancel: &CancellationToken) -> Result<()> {
        if cancel.is_cancelled() {
            return Err(ResultCode::Cancelled.into());
        }
        Ok(())
    }

    async fn resolve_provide(&self, ai_provide_id: String) -> Result<AiProvide> {
        let Some(shared) = self.ai_provide_service.find_by_id(ai_provide_id.clone()).await? else {
            return Err(ResultCode::ProviderNotFound.into());
        };
        let provide = shared.read().clone();
        Ok(provide)
    }

    fn build_tool_catalog(&self) -> String {
        let mut lines: Vec<String> = Vec::new();
        for tool in self.tool_manager.tools() {
            let rig = tool.as_rig_tool(None);
            let definition = rig.definition();
            lines.push(format!("- {}: {}", definition.name, definition.description));
        }
        if lines.is_empty() {
            return "（当前无已注册 builtin 工具）".to_string();
        }
        lines.join("\n")
    }

    fn build_callable_tools(&self, chat_id: Option<String>) -> Vec<DynamicTool> {
        let mut tools: Vec<DynamicTool> = Vec::new();
        if let Some(chat_id) = chat_id {
            tools.push(ToolBaseOutWrap::wrap(ToolChatHistoryBound::new(chat_id, self.ai_chat_item_service.clone()), TOOL_OUTPUT_MAX_TOKENS, None));
        }
        tools.push(ToolBaseOutWrap::wrap(ToolSuggestWebFetch, TOOL_OUTPUT_MAX_TOKENS, None));
        tools.push(ToolBaseOutWrap::wrap(ToolSuggestWebSearch, TOOL_OUTPUT_MAX_TOKENS, None));
        tools
    }

    fn parse_skill(&self, raw: String) -> Result<AiSkill> {
        match parse_skill_output(&raw) {
            Ok(out) => Ok(AiSkill { id: String::new(), sort: out.sort, name: out.name, content: out.content, description: out.description }),
            Err(err) => {
                let preview = truncate_chars(raw.trim(), 800);
                warn!("[ai] 技能提炼结果解析失败: {} raw={}", err, preview);
                Err(ResultCode::SkillExtractParseFailed.into())
            }
        }
    }
}

fn normalize_optional_text(value: Option<String>) -> Option<String> {
    value.and_then(|raw| {
        let trimmed = raw.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    })
}

fn draft_field(value: Option<&str>) -> &str {
    value.map(str::trim).filter(|item| !item.is_empty()).unwrap_or("（无）")
}

fn build_user_prompt(has_chat: bool, key_points: Option<&str>, name: Option<&str>, description: Option<&str>, content: Option<&str>, catalog: &str) -> String {
    let source_note = if has_chat {
        "结合已绑定会话历史、用户需求说明与当前技能草稿（若有）输出一条可复用技能 JSON；局部修订或推倒重来以需求说明为准。"
    } else {
        "结合用户需求说明与当前技能草稿（若有）输出一条可复用技能 JSON；局部修订或推倒重来以需求说明为准。无会话历史可查。"
    };
    let history_note = if has_chat { "chat_history：必用，按需分批检索，不要假设已看到全文。" } else { "chat_history：未绑定会话，不可用；不要尝试调用。" };
    let web_search_note = "web_search / web_fetch：仅当用户要点要求联网、或已有信息不足以确认外部事实时使用；先 search，需要某页原文再用 fetch。";
    let key_points = key_points.map(str::trim).filter(|value| !value.is_empty()).unwrap_or("（无）");
    render(
        LEAD_SKILL_EXTRACT_USER,
        &[
            ("source_note", source_note),
            ("history_note", history_note),
            ("web_search_note", web_search_note),
            ("tool_catalog", catalog),
            ("key_points", key_points),
            ("draft_name", draft_field(name)),
            ("draft_description", draft_field(description)),
            ("draft_content", draft_field(content)),
        ],
    )
}

fn parse_skill_output(raw: &str) -> Result<SkillExtractOutput> {
    let cleaned = strip_markdown_fence(raw);
    if let Ok(out) = serde_json::from_str::<SkillExtractOutput>(cleaned.trim()) {
        return Ok(out);
    }
    let start = cleaned.find('{').ok_or_else(|| anyhow!("no JSON object found in response"))?;
    let end = cleaned.rfind('}').ok_or_else(|| anyhow!("JSON object in response is incomplete"))?;
    if end < start {
        return Err(anyhow!("invalid JSON boundaries in response"));
    }
    let slice = &cleaned[start..=end];
    serde_json::from_str::<SkillExtractOutput>(slice).map_err(|e| anyhow!("{} | json={}", e, truncate_chars(slice, 600)))
}

fn strip_markdown_fence(raw: &str) -> String {
    let trimmed = raw.trim();
    let Some(rest) = trimmed.strip_prefix("```") else {
        return trimmed.to_string();
    };
    let rest = rest.strip_prefix("json").unwrap_or(rest).trim_start();
    let Some(body) = rest.strip_suffix("```") else {
        return trimmed.to_string();
    };
    body.trim().to_string()
}

fn truncate_chars(input: &str, max_chars: usize) -> String {
    let count = input.chars().count();
    if count <= max_chars {
        return input.to_string();
    }
    let truncated: String = input.chars().take(max_chars).collect();
    format!("{}...(truncated)", truncated)
}
