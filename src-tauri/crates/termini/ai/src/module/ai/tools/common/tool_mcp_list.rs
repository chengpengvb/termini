use anyhow::Result;
use std::sync::Arc;
use serde_json::json;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use crate::base::base_app::BaseApp;
use serde::{Deserialize, Serialize};
use crate::module::ai::dto::AiToolId;
use crate::model::resp::resp_page::QueryPage;
use crate::module::ai::shared::ToolCallDonePush;
use rig::tool::{DynamicTool, Tool, ToolContext};
use crate::module::ai::tools::tool_manager::ToolManager;
use crate::module::ai::tools::ai_tool_error::AiToolError;
use crate::service::service_ai_mcp_server::AiMcpServerService;
use crate::module::ai::tools::base::{ToolBaseOutWrap, WrapItem};
use crate::module::ai::tools::tool_base::{ToolBase, ToolDomain};
use crate::model::resp::resp_ai_mcp_server::RespAiMcpServerRuntime;

fn default_page_size() -> usize {
    5
}

const PAGE_SIZE_MAX: usize = 20;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ToolMcpListArgs {
    #[serde(default)]
    pub page_number: usize,

    #[serde(default = "default_page_size")]
    pub page_size: usize,

    #[serde(default)]
    pub keyword: Option<String>,
}

#[base_app(sort = 778)]
#[derive(Clone, Service)]
pub struct ToolMcpList {
    #[inject(component)]
    tool_manager: ToolManager,

    #[inject(component)]
    ai_mcp_server_service: AiMcpServerService,
}

#[async_trait]
impl BaseApp for ToolMcpList {
    fn name(&self) -> &'static str {
        "ToolMcpList"
    }

    async fn start(&self, _ctx: &crate::base::base_app::AppContext) -> Result<()> {
        self.tool_manager.register(Arc::new(self.clone()));
        Ok(())
    }
}

impl ToolBase for ToolMcpList {
    fn tool_id(&self) -> AiToolId {
        AiToolId::ToolMcpList
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

impl Tool for ToolMcpList {
    const NAME: &'static str = AiToolId::ToolMcpList.as_str();

    type Error = AiToolError;
    type Args = ToolMcpListArgs;
    type Output = QueryPage<RespAiMcpServerRuntime>;

    fn description(&self) -> String {
        "查询本应用已保存的 MCP 服务列表（含运行状态），支持分页。关键字：服务 id 精确匹配，名称模糊匹配（大小写不敏感）；不传则列出全部。每条含 server（PO）、connected、tool_count、last_error。connected=false 时 last_error 是启动失败原因，按该原文用 mcp_update 改配置后再查。创建或修改后必须再调本工具确认是否连上。改前先用本工具查到 id，再用 mcp_create / mcp_update / mcp_delete。分页字段：total 为全量匹配数（不受分页影响），page_number 从 0 开始，page_size 为每页条数，rows 为当前页数据；total > page_size 时递增 page_number 继续查。".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "keyword": {
                    "type": "string",
                    "description": "可选关键字，空格分隔多个词。服务 id 仅精确匹配；名称大小写不敏感部分匹配；命中越多越靠前。不传则列出全部。"
                },
                "page_number": {
                    "type": "integer",
                    "description": "页码，从 0 开始，默认 0。",
                    "default": 0
                },
                "page_size": {
                    "type": "integer",
                    "description": "每页条数，默认 5，上限 20。total 大于 page_size 时需翻页查询。",
                    "default": 5
                }
            }
        })
    }

    async fn call(&self, context: &mut ToolContext, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let servers = self.ai_mcp_server_service.list_runtime().await.map_err(|error| AiToolError::Execution(error.to_string()))?;
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
        let filtered: Vec<RespAiMcpServerRuntime> = if tokens.is_empty() {
            servers
        } else {
            let mut scored: Vec<(usize, RespAiMcpServerRuntime)> = servers
                .into_iter()
                .map(|item| {
                    let name = item.server.name.to_lowercase();
                    let score = tokens
                        .iter()
                        .filter(|token| {
                            if item.server.id.as_str() == token.as_str() {
                                return true;
                            }
                            name.contains(token.to_lowercase().as_str())
                        })
                        .count();
                    (score, item)
                })
                .filter(|(score, _)| *score > 0)
                .collect();
            scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.server.name.to_lowercase().cmp(&b.1.server.name.to_lowercase())));
            scored.into_iter().map(|(_, item)| item).collect()
        };
        let page = QueryPage::new(args.page_number, args.page_size.clamp(1, PAGE_SIZE_MAX), filtered);
        let body = serde_json::to_string(&page).map_err(|error| AiToolError::Execution(error.to_string()))?;
        context.insert_result(vec![WrapItem::wrap_item(None, body)]);
        Ok(page)
    }
}
