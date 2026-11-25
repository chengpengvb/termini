use serde_json::json;
use crate::module::ai::dto::AiToolId;
use crate::module::ai::shared::ToolCallDonePush;
use rig::tool::{DynamicTool, Tool, ToolContext};
use crate::module::ai::tools::tool_base::ToolBase;
use crate::module::ai::tools::ai_tool_error::AiToolError;
use crate::module::ai::tools::common::tool_web_fetch::ToolWebFetchArgs;
use crate::module::ai::tools::base::{ToolBaseOutWrap, WrapItem, TOOL_OUTPUT_MAX_TOKENS};
use crate::module::ai::tools::common::web::fetch_url::{fetch_url, present, resolve_url};

#[derive(Clone)]
pub struct ToolSuggestWebFetch;

impl ToolBase for ToolSuggestWebFetch {
    fn tool_id(&self) -> AiToolId {
        AiToolId::ToolWebFetch
    }

    fn as_rig_tool(&self, done_push: Option<ToolCallDonePush>) -> DynamicTool {
        ToolBaseOutWrap::wrap(self.clone(), TOOL_OUTPUT_MAX_TOKENS, done_push)
    }
}

impl Tool for ToolSuggestWebFetch {
    const NAME: &'static str = AiToolId::ToolWebFetch.as_str();

    type Args = ToolWebFetchArgs;
    type Output = String;
    type Error = AiToolError;

    fn description(&self) -> String {
        "通过 HTTP GET 获取指定 URL 的文本内容。http 会自动升级为 https。不支持需要登录的私有页面。长页面会被截断。很多站点有反爬虫机制，抓取可能被拦截（如 403/人机验证），失败后不要对同一站点反复换址重试，改用 web_search 摘要或向用户说明。".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "要抓取的 URL（https）"
                }
            },
            "required": ["url"]
        })
    }

    async fn call(&self, context: &mut ToolContext, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let url = resolve_url(args.url.as_deref().unwrap_or_default())?;
        let title = format!("===== {} =====", url);
        let body = fetch_url(url, false).await?;
        let body = present(body, None, None)?;
        context.insert_result(vec![WrapItem::wrap_item(Some(title), body.clone())]);
        Ok(body)
    }
}
