use anyhow::Result;
use std::sync::Arc;
use serde_json::json;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use crate::base::base_app::BaseApp;
use serde::{Deserialize, Serialize};
use crate::module::ai::dto::AiToolId;
use crate::module::ai::shared::ToolCallDonePush;
use rig::tool::{DynamicTool, Tool, ToolContext};
use crate::module::ai::tools::tool_base::ToolBase;
use crate::module::ai::tools::tool_manager::ToolManager;
use crate::module::ai::tools::ai_tool_error::AiToolError;
use crate::module::ai::tools::base::{ToolBaseOutWrap, WrapItem};
use crate::service::service_ai_crud::{page_entities, LIST_LIMIT_DEFAULT, LIST_LIMIT_MAX};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ToolEntityListArgs {
    #[serde(default)]
    pub keyword: Option<String>,
    #[serde(default)]
    pub offset: Option<usize>,
    #[serde(default)]
    pub limit: Option<usize>,
}

#[base_app(sort = 774)]
#[derive(Clone, Service)]
pub struct ToolEntityList {
    #[inject(component)]
    tool_manager: ToolManager,
}

#[async_trait]
impl BaseApp for ToolEntityList {
    fn name(&self) -> &'static str {
        "ToolEntityList"
    }

    async fn start(&self, _ctx: &crate::base::base_app::AppContext) -> Result<()> {
        self.tool_manager.register(Arc::new(self.clone()));
        Ok(())
    }
}

impl ToolBase for ToolEntityList {
    fn tool_id(&self) -> AiToolId {
        AiToolId::ToolEntityList
    }

    fn read_only(&self) -> bool {
        true
    }

    fn dedup_in_run(&self) -> bool {
        true
    }

    fn as_rig_tool(&self, done_push: Option<ToolCallDonePush>) -> DynamicTool {
        ToolBaseOutWrap::wrap(self.clone(), self.max_result_tokens(), done_push)
    }
}

impl Tool for ToolEntityList {
    const NAME: &'static str = AiToolId::ToolEntityList.as_str();

    type Error = AiToolError;
    type Args = ToolEntityListArgs;
    type Output = serde_json::Value;

    fn description(&self) -> String {
        "列出本应用可读写的全部数据表（通用 CRUD 入口）。只返回表名和字段数，不含行数据。keyword 按表名过滤；offset/limit 翻页，total 大于本页时继续翻。有哪些表以本工具返回为准，不要只认密钥相关表。读字段用 entity_describe，读写行用 entity_call。".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "keyword": {
                    "type": "string",
                    "description": "可选，按表名过滤"
                },
                "offset": {
                    "type": "integer",
                    "description": "从第几条开始，默认 0"
                },
                "limit": {
                    "type": "integer",
                    "description": "本页条数，默认 8，上限 20"
                }
            }
        })
    }

    async fn call(&self, context: &mut ToolContext, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let limit = args.limit.unwrap_or(LIST_LIMIT_DEFAULT).clamp(1, LIST_LIMIT_MAX);
        let page = page_entities(args.offset.unwrap_or(0), limit, args.keyword.as_deref()).map_err(|error| AiToolError::Execution(format!("{:#}", error)))?;
        let body = serde_json::to_string(&page).map_err(|error| AiToolError::Execution(error.to_string()))?;
        context.insert_result(vec![WrapItem::wrap_item(None, body)]);
        Ok(page)
    }
}
