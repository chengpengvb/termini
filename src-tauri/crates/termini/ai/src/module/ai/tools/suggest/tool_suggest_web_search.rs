use serde_json::json;
use crate::module::ai::dto::AiToolId;
use crate::module::ai::shared::ToolCallDonePush;
use rig::tool::{DynamicTool, Tool, ToolContext};
use crate::module::ai::tools::tool_base::ToolBase;
use crate::module::ai::tools::ai_tool_error::AiToolError;
use crate::module::ai::tools::base::{ToolBaseOutWrap, TOOL_OUTPUT_MAX_TOKENS};
use crate::module::ai::tools::common::tool_web_search::{ToolWebSearch, ToolWebSearchArgs, MAX_BATCH_QUERIES};

#[derive(Clone)]
pub struct ToolSuggestWebSearch;

impl ToolBase for ToolSuggestWebSearch {
    fn tool_id(&self) -> AiToolId {
        AiToolId::ToolWebSearch
    }

    fn persist(&self) -> bool {
        true
    }

    fn as_rig_tool(&self, done_push: Option<ToolCallDonePush>) -> DynamicTool {
        ToolBaseOutWrap::wrap(self.clone(), TOOL_OUTPUT_MAX_TOKENS, done_push)
    }
}

impl Tool for ToolSuggestWebSearch {
    const NAME: &'static str = AiToolId::ToolWebSearch.as_str();

    type Error = AiToolError;
    type Args = ToolWebSearchArgs;
    type Output = String;

    fn description(&self) -> String {
        "联网检索。由你根据 instruction 原意判断用户是否明确要求联网检索，不限语言；只有明确要求才调用，其余情况直接凭上下文给候选。要查多个写法或型号时一次传 queries（最多 5 条，并发执行，按 ===== 查询 ===== 分段返回），禁止一条条来回搜。拿到摘要后立刻给出候选，不要再次检索。".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "单条检索关键词；用陈述性的关键词组合（如「设备型号/系统 + 命令目的」），不要写成整句提问。与 queries 二选一"
                },
                "queries": {
                    "type": "array",
                    "description": "批量检索时传数组，最多 5 条，一次并发查完；与 query 二选一",
                    "maxItems": MAX_BATCH_QUERIES,
                    "items": { "type": "string" }
                },
                "max_results": {
                    "type": "integer",
                    "description": "每条查询返回条数，默认 5，最多 10"
                }
            }
        })
    }

    async fn call(&self, context: &mut ToolContext, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let queries = ToolWebSearch::resolve_queries(&args)?;
        let max_results = ToolWebSearch::resolve_max_results(args.max_results);
        let result = ToolWebSearch::search_many(queries.clone(), max_results).await?;
        let items = ToolWebSearch::wrap_search(&queries, result)?;
        context.insert_result(items);
        Ok(String::new())
    }
}
