use anyhow::Result;
use std::sync::Arc;
use serde_json::json;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use crate::base::base_app::BaseApp;
use serde::{Deserialize, Serialize};
use crate::module::ai::dto::AiToolId;
use crate::module::ai::dto::AiRunContext;
use crate::module::ai::shared::ToolCallDonePush;
use rig::tool::{DynamicTool, Tool, ToolContext};
use crate::module::ai::shared::ChatSessionCacheManager;
use crate::module::ai::tools::tool_manager::ToolManager;
use crate::module::ai::tools::ai_tool_error::AiToolError;
use crate::module::ai::tools::base::{ToolBaseOutWrap, WrapItem};
use crate::module::ai::tools::tool_base::{ToolBase, ToolDomain};
use crate::module::ai::shared::{DeferredTool, ToolActivationStore};

const LIMIT_DEFAULT: usize = 8;
const LIMIT_MAX: usize = 20;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ToolToolSearchArgs {
    pub query: String,

    #[serde(default)]
    pub limit: Option<usize>,
}

#[base_app(sort = 762)]
#[derive(Clone, Service)]
pub struct ToolToolSearch {
    #[inject(component)]
    tool_manager: ToolManager,

    #[inject(component)]
    tool_activation_store: ToolActivationStore,

    #[inject(component)]
    chat_session_cache_manager: ChatSessionCacheManager,
}

#[async_trait]
impl BaseApp for ToolToolSearch {
    fn name(&self) -> &'static str {
        "ToolToolSearch"
    }

    async fn start(&self, _ctx: &crate::base::base_app::AppContext) -> Result<()> {
        self.tool_manager.register(Arc::new(self.clone()));
        Ok(())
    }
}

impl ToolBase for ToolToolSearch {
    fn tool_id(&self) -> AiToolId {
        AiToolId::ToolToolSearch
    }

    fn read_only(&self) -> bool {
        true
    }

    fn domains(&self) -> &'static [ToolDomain] {
        &[ToolDomain::Main]
    }

    fn on_stop(&self, req_id: &str) {
        self.tool_activation_store.forget_req(req_id);
    }

    fn as_rig_tool(&self, done_push: Option<ToolCallDonePush>) -> DynamicTool {
        ToolBaseOutWrap::wrap(self.clone(), self.max_result_tokens(), done_push)
    }
}

impl Tool for ToolToolSearch {
    const NAME: &'static str = AiToolId::ToolToolSearch.as_str();

    type Error = AiToolError;
    type Args = ToolToolSearchArgs;
    type Output = String;

    fn description(&self) -> String {
        "按关键词检索当前未直接提供的工具（上下文里「可检索工具」列出的那些，以及 MCP 服务的全部工具）。命中的工具从下一轮起可以直接调用。query 写你要做的事或工具名关键词，中英文都可以；一次可以查多个词，用空格分开。不要猜测未列出的工具名。".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "检索关键词，如「导出 Word」「截图」「代码符号」或工具名片段"
                },
                "limit": {
                    "type": "integer",
                    "description": "最多返回多少个工具，默认 8"
                }
            },
            "required": ["query"]
        })
    }

    async fn call(&self, context: &mut ToolContext, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let query = args.query.trim().to_lowercase();
        if query.is_empty() {
            return Err(AiToolError::Execution("query must not be empty".to_string()));
        }
        let run_context = context.require::<Arc<AiRunContext>>().map_err(|error| AiToolError::Execution(error.to_string()))?;
        let Some(session_entry) = self.chat_session_cache_manager.get(&run_context.req_id) else {
            return Err(AiToolError::Execution(format!("unknown req_id: {}", run_context.req_id)));
        };
        let chat_id = session_entry.context.ai_chat_id.clone();
        drop(session_entry);
        let Some(catalog) = self.tool_activation_store.catalog(&run_context.req_id) else {
            return Err(AiToolError::Execution(format!("tool catalog unavailable for req_id={}", run_context.req_id)));
        };
        let limit = args.limit.unwrap_or(LIMIT_DEFAULT).clamp(1, LIMIT_MAX);
        let terms = Self::terms(&query);
        let mut scored: Vec<(usize, &DeferredTool)> = catalog.deferred.iter().map(|tool| (Self::score(tool, &query, &terms), tool)).filter(|(score, _)| *score > 0).collect();
        scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.name.cmp(&b.1.name)));
        let hits: Vec<&DeferredTool> = scored.into_iter().take(limit).map(|(_, tool)| tool).collect();
        let body = if hits.is_empty() {
            format!("no tool matched \"{}\"; available deferred tools: {}", args.query.trim(), catalog.deferred.iter().map(|tool| tool.name.as_str()).collect::<Vec<&str>>().join(", "))
        } else {
            let names: Vec<String> = hits.iter().map(|tool| tool.name.clone()).collect();
            self.tool_activation_store.activate(&chat_id, &names);
            names.join("\n")
        };
        context.insert_result(vec![WrapItem::wrap_item(None, body.clone())]);
        Ok(body)
    }
}

impl ToolToolSearch {
    fn terms(query: &str) -> Vec<String> {
        query.split(|value: char| value.is_whitespace() || value == ',' || value == '，' || value == '/').map(|term| term.trim().to_string()).filter(|term| !term.is_empty()).collect()
    }

    fn score(tool: &DeferredTool, query: &str, terms: &[String]) -> usize {
        let name = tool.name.to_lowercase();
        let description = tool.description.to_lowercase();
        let mut score = 0usize;
        if name.contains(query) {
            score += 20;
        }
        for term in terms {
            if name.contains(term.as_str()) {
                score += 10;
            }
            if description.contains(term.as_str()) {
                score += 3;
            }
            for character in term.chars().filter(|value| !value.is_ascii()) {
                if description.contains(character) {
                    score += 1;
                }
            }
        }
        score
    }
}
