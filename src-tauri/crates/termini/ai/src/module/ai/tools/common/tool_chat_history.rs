use anyhow::Result;
use std::sync::Arc;
use serde_json::json;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use crate::base::base_app::BaseApp;
use serde::{Deserialize, Serialize};
use crate::module::ai::shared::chat_item_format;
use crate::module::ai::shared::ToolCallDonePush;
use rig::tool::{DynamicTool, Tool, ToolContext};
use crate::module::ai::tools::tool_base::ToolBase;
use crate::module::ai::dto::{AiRunContext, AiToolId};
use crate::module::ai::shared::ChatSessionCacheManager;
use crate::module::ai::tools::tool_manager::ToolManager;
use crate::module::ai::tools::ai_tool_error::AiToolError;
use crate::service::service_ai_chat_item::AiChatItemService;
use crate::module::ai::tools::base::{ToolBaseOutWrap, WrapItem};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ToolChatHistoryArgs {
    #[serde(default)]
    pub scope: Option<String>,

    #[serde(default)]
    pub limit: Option<usize>,

    #[serde(default)]
    pub query: Option<String>,

    #[serde(default)]
    pub chat_id: Option<String>,

    #[serde(default)]
    pub offset: Option<usize>,

    #[serde(default, rename = "__internal_call_id")]
    pub internal_call_id: Option<String>,
}

const PAGE_DEFAULT_LIMIT: usize = 40;
const PAGE_MAX_LIMIT: usize = 200;

fn tool_description() -> String {
    "查询会话历史（含用户消息、助手终稿、工具调用的入参与返回）。不传 chat_id 查当前会话中早于热记忆(当前上下文)的部分：上下文里已有的内容不会返回、也不需要查，只在需要找回被截断的更早事实时使用，越久远的信息优先 scope=user 回溯用户原话，助手长文与工具输出用 scope=search 按需检索。传 chat_id 可查另一个会话（用户会把要接续的对话 id 发给你）：默认 scope=page 按时间顺序分页返回全部内容，用 offset/limit 翻页、按返回里的 of 总数判断是否读完，读完再动手；也可对该会话用 user/search/recent_ai/all。".to_string()
}

fn tool_parameters() -> serde_json::Value {
    json!({
        "type": "object",
        "properties": {
            "chat_id": {
                "type": "string",
                "description": "要查询的会话 id；不传或等于当前会话时查当前会话的更早历史"
            },
            "scope": {
                "type": "string",
                "description": "user（当前会话默认，只查用户输入，回溯需求首选）| search（关键字检索，找回早前的路径/脚本/包名等事实，须配 query）| recent_ai（助手/工具）| all（用户 + 少量 AI）| page（按时间顺序分页读全部，传了 chat_id 时的默认值）"
            },
            "limit": {
                "type": "integer",
                "description": "最多返回条数；user/all 默认 30，search 默认 8（命中数），recent_ai 默认 5，page 默认 40 上限 200"
            },
            "offset": {
                "type": "integer",
                "description": "scope=page 的起始行（从 0 开始），配合返回里的 showing lines a-b of z 翻页"
            },
            "query": {
                "type": "string",
                "description": "关键字，空格分隔多个词（先取全部命中，无结果再退化为任一命中）；scope=search 时必填"
            }
        }
    })
}

#[base_app(sort = 761)]
#[derive(Clone, Service)]
pub struct ToolChatHistory {
    #[inject(component)]
    tool_manager: ToolManager,

    #[inject(component)]
    ai_chat_item_service: AiChatItemService,

    #[inject(component)]
    tool_call_done_push: ToolCallDonePush,

    #[inject(component)]
    chat_session_cache_manager: ChatSessionCacheManager,
}

#[async_trait]
impl BaseApp for ToolChatHistory {
    fn name(&self) -> &'static str {
        "ToolChatHistory"
    }

    async fn start(&self, _ctx: &crate::base::base_app::AppContext) -> Result<()> {
        self.tool_manager.register(Arc::new(self.clone()));
        Ok(())
    }
}

impl ToolBase for ToolChatHistory {
    fn tool_id(&self) -> AiToolId {
        AiToolId::ToolChatHistory
    }

    fn dedup_in_run(&self) -> bool {
        true
    }

    fn deferred(&self) -> bool {
        true
    }

    fn as_rig_tool(&self, done_push: Option<ToolCallDonePush>) -> DynamicTool {
        ToolBaseOutWrap::wrap(self.clone(), self.max_result_tokens(), done_push)
    }
}

impl ToolChatHistory {
    fn resolve_limit(scope: chat_item_format::ChatHistoryScope, limit: Option<usize>) -> usize {
        let fallback = match scope {
            chat_item_format::ChatHistoryScope::Search => chat_item_format::CHAT_HISTORY_DEFAULT_SEARCH_LIMIT,
            chat_item_format::ChatHistoryScope::RecentAi => chat_item_format::CHAT_HISTORY_DEFAULT_RECENT_AI_LIMIT,
            chat_item_format::ChatHistoryScope::User | chat_item_format::ChatHistoryScope::All => chat_item_format::CHAT_HISTORY_DEFAULT_USER_LIMIT,
        };
        limit.unwrap_or(fallback).clamp(1, 100)
    }

    fn push_cancelled(&self, run_context: &AiRunContext, call_id: &str) -> String {
        let result = "cancelled".to_string();
        self.tool_call_done_push.push_tool_call_done_event(run_context, call_id, result.clone());
        result
    }

    pub async fn query_other_chat(ai_chat_item_service: &AiChatItemService, chat_id: String, args: &ToolChatHistoryArgs) -> Result<String, AiToolError> {
        let shared_items = ai_chat_item_service.select_by_chat_id(chat_id.clone()).await.map_err(|err| AiToolError::Execution(err.to_string()))?;
        let items: Vec<_> = shared_items.into_iter().map(|item| item.read().clone()).collect();
        if items.is_empty() {
            return Err(AiToolError::Execution(format!("chat not found or has no history: chat_id={}", chat_id)));
        }
        let text = match args.scope.as_deref() {
            Some("page") | None => chat_item_format::render_chat_history_page(&items, args.offset.unwrap_or(0), args.limit.unwrap_or(PAGE_DEFAULT_LIMIT).clamp(1, PAGE_MAX_LIMIT)),
            Some(raw) => {
                let scope = chat_item_format::ChatHistoryScope::parse(raw);
                chat_item_format::render_chat_history(&items, scope, Self::resolve_limit(scope, args.limit), args.query.as_deref())
            }
        };
        Ok(format!("[chat_history chat_id={}]\n{}", chat_id, text))
    }

    pub async fn query(ai_chat_item_service: &AiChatItemService, chat_id: String, args: &ToolChatHistoryArgs, memory_start_item_id: Option<String>) -> Result<String, AiToolError> {
        if let Some(other) = args.chat_id.as_deref().map(str::trim).filter(|id| !id.is_empty() && *id != chat_id) {
            return Self::query_other_chat(ai_chat_item_service, other.to_string(), args).await;
        }
        let scope = chat_item_format::ChatHistoryScope::parse(args.scope.as_deref().unwrap_or("user"));
        if matches!(scope, chat_item_format::ChatHistoryScope::Search) {
            let query = args.query.as_deref().map(str::trim).unwrap_or("");
            if query.is_empty() {
                return Err(AiToolError::Execution("query is required when scope=search".to_string()));
            }
        }
        let limit = Self::resolve_limit(scope, args.limit);
        let shared_items = ai_chat_item_service.select_by_chat_id(chat_id).await.map_err(|err| AiToolError::Execution(err.to_string()))?;
        let mut items: Vec<_> = shared_items.into_iter().map(|item| item.read().clone()).collect();

        let Some(boundary) = memory_start_item_id.and_then(|id| items.iter().find(|item| item.id == id)).map(|item| item.timestamp) else {
            return Ok(chat_item_format::render_chat_history(&items, scope, limit, args.query.as_deref()));
        };
        let total = items.len();
        items.retain(|item| item.timestamp < boundary);
        if items.is_empty() {
            return Ok("[chat_history] hot memory already covers the entire history of this chat: anything not in the context is not in the history either; do not call this tool again.".to_string());
        }
        let text = chat_item_format::render_chat_history(&items, scope, limit, args.query.as_deref());
        Ok(format!("[chat_history] 仅包含热记忆之前的更早历史({}/{} 条参与检索, 上下文已有的部分不重复返回)\n{}", items.len(), total, text))
    }

    fn memory_start_item_id(&self, req_id: &str) -> Option<String> {
        self.chat_session_cache_manager.turns(req_id).iter().map(|turn| turn.item_id.as_str()).find(|id| !id.is_empty()).map(str::to_string)
    }
}

#[derive(Clone)]
pub struct ToolChatHistoryBound {
    chat_id: String,
    ai_chat_item_service: AiChatItemService,
}

impl ToolChatHistoryBound {
    pub fn new(chat_id: String, ai_chat_item_service: AiChatItemService) -> Self {
        Self { chat_id, ai_chat_item_service }
    }
}

impl ToolBase for ToolChatHistoryBound {
    fn tool_id(&self) -> AiToolId {
        AiToolId::ToolChatHistory
    }

    fn as_rig_tool(&self, done_push: Option<ToolCallDonePush>) -> DynamicTool {
        ToolBaseOutWrap::wrap(self.clone(), self.max_result_tokens(), done_push)
    }
}

impl Tool for ToolChatHistoryBound {
    const NAME: &'static str = AiToolId::ToolChatHistory.as_str();

    type Args = ToolChatHistoryArgs;
    type Output = String;
    type Error = AiToolError;

    fn description(&self) -> String {
        tool_description()
    }

    fn parameters(&self) -> serde_json::Value {
        tool_parameters()
    }

    async fn call(&self, context: &mut ToolContext, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let body = ToolChatHistory::query(&self.ai_chat_item_service, self.chat_id.clone(), &args, None).await?;
        context.insert_result(vec![WrapItem::wrap_item(None, body.clone())]);
        Ok(body)
    }
}

impl Tool for ToolChatHistory {
    const NAME: &'static str = AiToolId::ToolChatHistory.as_str();

    type Error = AiToolError;
    type Args = ToolChatHistoryArgs;
    type Output = String;

    fn description(&self) -> String {
        tool_description()
    }

    fn parameters(&self) -> serde_json::Value {
        tool_parameters()
    }

    async fn call(&self, context: &mut ToolContext, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let run_context = context.require::<Arc<AiRunContext>>().map_err(|error| AiToolError::Execution(error.to_string()))?;
        let Some(call_id) = args.internal_call_id.clone() else {
            return Err(AiToolError::Execution("internal_call_id of the tool call is missing".to_string()));
        };
        let session_cancel = run_context.cancel_token.clone();
        let Some(tool_cancel) = self.chat_session_cache_manager.get_tool_cancel(&run_context.req_id, &run_context.run_id, &call_id) else {
            return Err(AiToolError::Execution(format!("tool cancel not found: req_id={} run_id={} call_id={}", run_context.req_id, run_context.run_id, call_id)));
        };

        let chat_id = {
            let Some(entry) = self.chat_session_cache_manager.get(&run_context.req_id) else {
                return Err(AiToolError::Execution(format!("unknown req_id: {}", run_context.req_id)));
            };
            entry.context.ai_chat_id.clone()
        };

        let memory_start = self.memory_start_item_id(&run_context.req_id);
        let body = tokio::select! {
            biased;
            _ = tool_cancel.cancelled() => self.push_cancelled(&run_context, &call_id),
            _ = session_cancel.cancelled() => self.push_cancelled(&run_context, &call_id),
            res = ToolChatHistory::query(&self.ai_chat_item_service, chat_id, &args, memory_start) => {
                let result = res?;
                self.tool_call_done_push.push_tool_call_done_event(&run_context, &call_id, result.clone());
                result
            }
        };
        context.insert_result(vec![WrapItem::wrap_item(None, body.clone())]);
        Ok(body)
    }
}
