use anyhow::Result;
use std::sync::Arc;
use serde_json::json;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use crate::base::base_app::BaseApp;
use crate::model::ai_skill::AiSkill;
use serde::{Deserialize, Serialize};
use crate::module::ai::dto::AiToolId;
use crate::model::resp::resp_page::QueryPage;
use crate::service::service_base::BaseService;
use crate::module::ai::shared::ToolCallDonePush;
use rig::tool::{DynamicTool, Tool, ToolContext};
use crate::util::token_util::estimate_text_tokens;
use crate::service::service_ai_skill::AiSkillService;
use crate::module::ai::tools::tool_manager::ToolManager;
use crate::module::ai::tools::ai_tool_error::AiToolError;
use crate::module::ai::tools::tool_base::{ToolBase, ToolDomain};
use crate::module::ai::tools::base::{ToolBaseOutWrap, WrapItem, TOOL_OUTPUT_MAX_TOKENS};

fn default_page_size() -> usize {
    5
}

const PAGE_SIZE_MAX: usize = 20;
const MAX_CONTENT_LINES: usize = 1000;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ToolSkillListArgs {
    #[serde(default)]
    pub offset: Option<i64>,

    #[serde(default)]
    pub limit: Option<usize>,

    #[serde(default)]
    pub page_number: usize,

    #[serde(default = "default_page_size")]
    pub page_size: usize,

    #[serde(default)]
    pub keyword: Option<String>,
}

#[base_app(sort = 771)]
#[derive(Clone, Service)]
pub struct ToolSkillList {
    #[inject(component)]
    tool_manager: ToolManager,

    #[inject(component)]
    ai_skill_service: AiSkillService,
}

#[async_trait]
impl BaseApp for ToolSkillList {
    fn name(&self) -> &'static str {
        "ToolSkillList"
    }

    async fn start(&self, _ctx: &crate::base::base_app::AppContext) -> Result<()> {
        self.tool_manager.register(Arc::new(self.clone()));
        Ok(())
    }
}

impl ToolBase for ToolSkillList {
    fn tool_id(&self) -> AiToolId {
        AiToolId::ToolSkillList
    }

    fn dedup_in_run(&self) -> bool {
        true
    }

    fn domains(&self) -> &'static [ToolDomain] {
        ToolDomain::READ_ONLY
    }

    fn as_rig_tool(&self, done_push: Option<ToolCallDonePush>) -> DynamicTool {
        ToolBaseOutWrap::wrap(self.clone(), self.max_result_tokens(), done_push)
    }
}

impl ToolSkillList {
    fn is_exact_id_detail(filtered: &[AiSkill], tokens: &[String]) -> bool {
        if filtered.len() != 1 || tokens.is_empty() {
            return false;
        }
        tokens.iter().any(|token| token == &filtered[0].id)
    }

    fn content_start_index(offset: Option<i64>, total: usize) -> usize {
        match offset {
            Some(value) if value < 0 => {
                let from_end = (-value) as usize;
                total.saturating_sub(from_end)
            }
            Some(0) => 0,
            Some(value) => {
                let start = value as usize;
                if start == 0 {
                    0
                } else {
                    start.saturating_sub(1).min(total)
                }
            }
            None => 0,
        }
    }

    fn page_skill_content(content: &str, offset: Option<i64>, limit: Option<usize>) -> Result<String, AiToolError> {
        if content.is_empty() {
            return Ok(String::new());
        }
        let tokens = estimate_text_tokens(content);
        let paged = offset.is_some() || limit.is_some() || tokens > TOOL_OUTPUT_MAX_TOKENS;
        if !paged {
            return Ok(content.to_string());
        }
        let lines: Vec<&str> = content.lines().collect();
        let total = lines.len();
        if total == 0 {
            return Ok(String::new());
        }
        let start_index = Self::content_start_index(offset, total);
        if start_index >= total {
            return Err(AiToolError::Execution(format!("offset past end of skill content, total lines={}", total)));
        }
        let read_limit = limit.unwrap_or(MAX_CONTENT_LINES).min(MAX_CONTENT_LINES);
        let end_index = (start_index + read_limit).min(total);
        let slice = &lines[start_index..end_index];
        let mut body = slice.join("\n");
        if end_index < total {
            body.push_str(&format!("\n… truncated, showing lines {}-{} of {}", start_index + 1, end_index, total));
        }
        Ok(body)
    }
}

impl Tool for ToolSkillList {
    const NAME: &'static str = AiToolId::ToolSkillList.as_str();

    type Error = AiToolError;
    type Args = ToolSkillListArgs;
    type Output = QueryPage<AiSkill>;

    fn description(&self) -> String {
        "查询本应用已保存的技能列表，支持分页。不传 keyword、名称/描述模糊命中、或一次命中多条时只返回 id / name / description，content 为空。仅当 keyword 中含技能 id 且只命中这一条时才返回 content 全文。正文超过输出上限时用 offset（从 1 起的行号）与 limit（最多行数，默认最多 1000 行）分页，看 showing lines X-Y of Z 继续读，读完再执行；不要把未读完的正文当完整技能。见 @[skill:id] 或用户点名技能时先本工具查到 id，再带该 id 作为 keyword 取 content。技能不是设备目标，不能用本工具查设备。分页字段：total 为技能条数（不受正文分页影响），page_number 从 0 开始，page_size 为每页条数，rows 为当前页数据；total > page_size 时递增 page_number 继续查。".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "keyword": {
                    "type": "string",
                    "description": "可选关键字，空格分隔多个词。技能 id 仅精确匹配；技能名与 description 大小写不敏感部分匹配；命中越多越靠前。不传则列出全部（无 content）。只有 keyword 命中唯一技能 id 时才返回 content。"
                },
                "page_number": {
                    "type": "integer",
                    "description": "技能列表页码，从 0 开始，默认 0。",
                    "default": 0
                },
                "page_size": {
                    "type": "integer",
                    "description": "每页技能条数，默认 5，上限 20。total 大于 page_size 时需翻页查询。",
                    "default": 5
                },
                "offset": {
                    "type": "integer",
                    "description": "仅在返回 content 时有效：正文起始行号，从 1 开始；负数表示从末尾倒数。不传则从第 1 行开始"
                },
                "limit": {
                    "type": "integer",
                    "description": "仅在返回 content 时有效：最多返回正文行数，默认最多 1000 行"
                }
            }
        })
    }

    async fn call(&self, context: &mut ToolContext, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let skills = self.ai_skill_service.find_all().await.map_err(|err| AiToolError::Execution(err.to_string()))?;

        let tokens: Vec<String> = match args.keyword.as_deref() {
            Some(text) => {
                let trimmed = text.trim();
                if trimmed.is_empty() {
                    Vec::new()
                } else {
                    trimmed.split_whitespace().map(|token| token.to_string()).collect()
                }
            }
            None => Vec::new(),
        };

        let mut filtered: Vec<AiSkill> = if tokens.is_empty() {
            skills.into_iter().map(|item| item.read().clone()).collect()
        } else {
            let mut scored: Vec<(usize, AiSkill)> = skills
                .into_iter()
                .map(|item| {
                    let skill = item.read().clone();
                    let name = skill.name.to_lowercase();
                    let description = skill.description.to_lowercase();
                    let score = tokens
                        .iter()
                        .filter(|token| {
                            if skill.id.as_str() == token.as_str() {
                                return true;
                            }
                            let lower = token.to_lowercase();
                            name.contains(lower.as_str()) || description.contains(lower.as_str())
                        })
                        .count();
                    (score, skill)
                })
                .filter(|(score, _)| *score > 0)
                .collect();
            scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.name.to_lowercase().cmp(&b.1.name.to_lowercase())));
            scored.into_iter().map(|(_, skill)| skill).collect()
        };

        if !Self::is_exact_id_detail(&filtered, &tokens) {
            let catalog: Vec<AiSkill> = filtered
                .into_iter()
                .map(|mut skill| {
                    skill.content = String::new();
                    skill
                })
                .collect();
            let page = QueryPage::new(args.page_number, args.page_size.clamp(1, PAGE_SIZE_MAX), catalog);
            let body = serde_json::to_string(&page).map_err(|error| AiToolError::Execution(error.to_string()))?;
            context.insert_result(vec![WrapItem::wrap_item(None, body)]);
            return Ok(page);
        }

        let mut skill = filtered.remove(0);
        skill.content = Self::page_skill_content(&skill.content, args.offset, args.limit)?;
        let page = QueryPage::new(0, 1, vec![skill]);
        let body = serde_json::to_string(&page).map_err(|error| AiToolError::Execution(error.to_string()))?;
        context.insert_result(vec![WrapItem::wrap_item(None, body)]);
        Ok(page)
    }
}
