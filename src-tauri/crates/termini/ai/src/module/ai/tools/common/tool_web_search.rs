use std::sync::Arc;
use serde_json::json;
use std::time::Duration;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use crate::base::base_app::BaseApp;
use serde::{Deserialize, Serialize};
use crate::module::ai::dto::AiToolId;
use crate::module::ai::dto::AiRunContext;
use crate::module::ai::shared::ToolCallDonePush;
use rig::tool::{DynamicTool, Tool, ToolContext};
use crate::module::ai::tools::tool_base::ToolBase;
use crate::module::ai::shared::ToolConflictManager;
use crate::service::service_base_api::BaseApiService;
use crate::module::ai::shared::ChatSessionCacheManager;
use crate::module::ai::tools::tool_manager::ToolManager;
use crate::module::ai::tools::ai_tool_error::AiToolError;
use crate::module::ai::tools::base::{ToolBaseOutWrap, WrapItem};

const LIMIT_MAX_RESULTS: u8 = 10;
const DEFAULT_MAX_RESULTS: u8 = 5;
const REQUEST_TIMEOUT_SECS: u64 = 60;
pub(crate) const MAX_BATCH_QUERIES: usize = 5;
const CANCELLED_RESULT: &str = "exit_code=cancelled";

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ToolWebSearchArgs {
    #[serde(default)]
    pub query: Option<String>,

    #[serde(default)]
    pub queries: Option<Vec<String>>,

    #[serde(default)]
    pub max_results: Option<u8>,

    #[serde(default, rename = "__internal_call_id")]
    pub internal_call_id: Option<String>,
}

#[base_app(sort = 767)]
#[derive(Clone, Service)]
pub struct ToolWebSearch {
    #[inject(component)]
    tool_manager: ToolManager,

    #[inject(component)]
    tool_call_done_push: ToolCallDonePush,

    #[inject(component)]
    tool_conflict_manager: ToolConflictManager,

    #[inject(component)]
    chat_session_cache_manager: ChatSessionCacheManager,
}

#[async_trait]
impl BaseApp for ToolWebSearch {
    fn name(&self) -> &'static str {
        "ToolWebSearch"
    }

    async fn start(&self, _ctx: &crate::base::base_app::AppContext) -> anyhow::Result<()> {
        self.tool_manager.register(Arc::new(self.clone()));
        Ok(())
    }
}

impl ToolBase for ToolWebSearch {
    fn tool_id(&self) -> AiToolId {
        AiToolId::ToolWebSearch
    }

    fn read_only(&self) -> bool {
        true
    }

    fn persist(&self) -> bool {
        true
    }

    fn as_rig_tool(&self, done_push: Option<ToolCallDonePush>) -> DynamicTool {
        ToolBaseOutWrap::wrap(self.clone(), self.max_result_tokens(), done_push)
    }
}

impl ToolWebSearch {
    fn push_cancelled(&self, run_context: &AiRunContext, call_id: &str) -> String {
        let result = CANCELLED_RESULT.to_string();
        self.tool_call_done_push.push_tool_call_done_event(run_context, call_id, result.clone());
        result
    }

    pub(crate) fn resolve_max_results(max_results: Option<u8>) -> u8 {
        let value = max_results.unwrap_or(DEFAULT_MAX_RESULTS);
        if value == 0 {
            return DEFAULT_MAX_RESULTS;
        }
        value.min(LIMIT_MAX_RESULTS)
    }

    pub(crate) fn resolve_queries(args: &ToolWebSearchArgs) -> Result<Vec<String>, AiToolError> {
        if let Some(queries) = args.queries.as_ref().filter(|queries| !queries.is_empty()) {
            let cleaned: Vec<String> = queries.iter().map(|query| query.trim().to_string()).filter(|query| !query.is_empty()).collect();
            if cleaned.len() > MAX_BATCH_QUERIES {
                return Err(AiToolError::Execution(format!("too many queries: {} exceeds {}", cleaned.len(), MAX_BATCH_QUERIES)));
            }
            if !cleaned.is_empty() {
                return Ok(cleaned);
            }
        }
        match args.query.as_deref().map(str::trim).filter(|query| !query.is_empty()) {
            Some(query) => Ok(vec![query.to_string()]),
            None => Err(AiToolError::Execution("query is empty (single search), or pass queries=[...] for batch".to_string())),
        }
    }

    pub(crate) async fn search_many(queries: Vec<String>, max_results: u8) -> Result<String, AiToolError> {
        if Self::api_token().is_none() {
            return Err(AiToolError::Execution("not logged in, web search is unavailable".to_string()));
        }
        let payload = if queries.len() == 1 { json!({ "query": queries[0], "maxResults": max_results }) } else { json!({ "queries": queries, "maxResults": max_results }) };
        let request = Self::api_post("/api/webSearchApi/search").timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS)).json(&payload);
        Self::api_data::<String>(request).await.map_err(|error| AiToolError::Execution(format!("cloud search failed: {:#}", error)))
    }

    pub(crate) fn wrap_search(queries: &[String], result: String) -> Result<Vec<WrapItem>, AiToolError> {
        if queries.len() == 1 {
            return Ok(vec![WrapItem::wrap_item(Some(format!("===== {} =====", queries[0])), result)]);
        }
        let mut items: Vec<WrapItem> = Vec::with_capacity(queries.len());
        let mut cursor = 0usize;
        for (index, query) in queries.iter().enumerate() {
            let header = format!("===== {} =====", query);
            let rest = &result[cursor..];
            let Some(rel) = rest.find(&header) else {
                return Err(AiToolError::Execution(format!("search result missing section for query: {}", query)));
            };
            if rel != 0 {
                return Err(AiToolError::Execution(format!("search result misaligned before query: {}", query)));
            }
            let mut start = cursor + header.len();
            if result[start..].starts_with('\n') {
                start += 1;
            }
            let end = if index + 1 < queries.len() {
                let next = format!("===== {} =====", queries[index + 1]);
                let Some(rel) = result[start..].find(&next) else {
                    return Err(AiToolError::Execution(format!("search result missing section for query: {}", queries[index + 1])));
                };
                start + rel
            } else {
                result.len()
            };
            items.push(WrapItem::wrap_item(Some(header), result[start..end].to_string()));
            cursor = end;
        }
        Ok(items)
    }
}

impl BaseApiService for ToolWebSearch {}

impl Tool for ToolWebSearch {
    const NAME: &'static str = AiToolId::ToolWebSearch.as_str();

    type Error = AiToolError;
    type Args = ToolWebSearchArgs;
    type Output = String;

    fn description(&self) -> String {
        "联网检索。按关键词找，返回若干条标题、网址和摘要。摘要已足以回答绝大多数问题，据此直接作答并附上网址即可；不要为了补全摘要而逐条去抓这些网页。要核实多个点时一次传 queries 数组（最多 5 条，并发执行，按 ===== 查询 ===== 分段返回），不要一条条来回搜。只用于时效性信息、不确定的事实或用户点名要来源的场合；古典文学、历史、通用技术概念等稳定知识直接作答，不要搜。".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "单条检索关键词；用陈述性的关键词组合，不要写成整句提问。与 queries 二选一"
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

    async fn call(&self, context: &mut ToolContext, args: Self::Args) -> anyhow::Result<Self::Output, Self::Error> {
        let run_context = context.require::<Arc<AiRunContext>>().map_err(|error| AiToolError::Execution(error.to_string()))?.clone();
        let Some(call_id) = args.internal_call_id.clone() else {
            return Err(AiToolError::Execution("internal_call_id of the tool call is missing".to_string()));
        };
        let queries = Self::resolve_queries(&args)?;
        let session_cancel = run_context.cancel_token.clone();
        let Some(tool_cancel) = self.chat_session_cache_manager.get_tool_cancel(&run_context.req_id, &run_context.run_id, &call_id) else {
            return Err(AiToolError::Execution(format!("tool cancel not found: req_id={} run_id={} call_id={}", run_context.req_id, run_context.run_id, call_id)));
        };
        let batch = queries.len() > 1;
        let confirm_args = ToolWebSearchArgs {
            query: if batch { None } else { Some(queries[0].clone()) },
            queries: if batch { Some(queries.clone()) } else { None },
            max_results: args.max_results,
            internal_call_id: None,
        };
        self.tool_conflict_manager.confirm_before_execute(&run_context, &call_id, self.tool_id().as_str(), &confirm_args).await?;
        let max_results = Self::resolve_max_results(args.max_results);
        tokio::select! {
            biased;
            _ = tool_cancel.cancelled() => {
                let result = self.push_cancelled(&run_context, &call_id);
                context.insert_result(vec![WrapItem::wrap_item(None, result.clone())]);
                Ok(result)
            }
            _ = session_cancel.cancelled() => {
                let result = self.push_cancelled(&run_context, &call_id);
                context.insert_result(vec![WrapItem::wrap_item(None, result.clone())]);
                Ok(result)
            }
            res = Self::search_many(queries.clone(), max_results) => {
                let items = Self::wrap_search(&queries, res?)?;
                let mut result = String::new();
                for item in &items {
                    if let Some(title) = &item.title {
                        result.push_str(title);
                        result.push('\n');
                    }
                    result.push_str(&item.body);
                    result.push('\n');
                }
                self.tool_call_done_push.push_tool_call_done_event(&run_context, &call_id, result);
                context.insert_result(items);
                Ok(String::new())
            }
        }
    }
}
