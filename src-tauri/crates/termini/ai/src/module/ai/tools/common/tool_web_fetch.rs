use std::sync::Arc;
use serde_json::json;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use crate::base::base_app::BaseApp;
use serde::{Deserialize, Serialize};
use crate::module::ai::dto::AiToolId;
use tokio_util::sync::CancellationToken;
use crate::module::ai::dto::AiRunContext;
use crate::module::ai::shared::ToolCallDonePush;
use rig::tool::{DynamicTool, Tool, ToolContext};
use crate::module::ai::tools::tool_base::ToolBase;
use crate::service::service_config::ConfigService;
use crate::module::ai::shared::ToolConflictManager;
use crate::module::ai::shared::ChatSessionCacheManager;
use crate::module::ai::tools::tool_manager::ToolManager;
use crate::module::ai::tools::ai_tool_error::AiToolError;
use crate::module::ai::tools::base::{ToolBaseOutWrap, WrapItem};
use crate::module::ai::derived::{DerivedKind, DerivedStore, WEB_TTL_SECS};
use crate::module::ai::tools::common::web::fetch_url::{fetch_url, is_trusted_url, present, resolve_url};

const MAX_BATCH_URLS: usize = 8;
const CANCELLED_RESULT: &str = "exit_code=cancelled";

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ToolWebFetchArgs {
    #[serde(default)]
    pub raw: Option<bool>,

    #[serde(default)]
    pub url: Option<String>,

    #[serde(default)]
    pub offset: Option<usize>,

    #[serde(default)]
    pub limit: Option<usize>,

    #[serde(default)]
    pub refresh: Option<bool>,

    #[serde(default)]
    pub urls: Option<Vec<String>>,

    #[serde(default, rename = "__internal_call_id")]
    pub internal_call_id: Option<String>,
}

enum FetchOutcome {
    Done(String),
    Cancelled,
}

#[base_app(sort = 766)]
#[derive(Clone, Service)]
pub struct ToolWebFetch {
    #[inject(component)]
    tool_manager: ToolManager,

    #[inject(component)]
    derived_store: DerivedStore,

    #[inject(component)]
    config_service: ConfigService,

    #[inject(component)]
    tool_call_done_push: ToolCallDonePush,

    #[inject(component)]
    tool_conflict_manager: ToolConflictManager,

    #[inject(component)]
    chat_session_cache_manager: ChatSessionCacheManager,
}

#[async_trait]
impl BaseApp for ToolWebFetch {
    fn name(&self) -> &'static str {
        "ToolWebFetch"
    }

    async fn start(&self, _ctx: &crate::base::base_app::AppContext) -> anyhow::Result<()> {
        self.tool_manager.register(Arc::new(self.clone()));
        Ok(())
    }
}

impl ToolBase for ToolWebFetch {
    fn tool_id(&self) -> AiToolId {
        AiToolId::ToolWebFetch
    }

    fn read_only(&self) -> bool {
        true
    }

    fn as_rig_tool(&self, done_push: Option<ToolCallDonePush>) -> DynamicTool {
        ToolBaseOutWrap::wrap(self.clone(), self.max_result_tokens(), done_push)
    }
}

impl ToolWebFetch {
    fn resolve_targets(args: &ToolWebFetchArgs) -> Result<Vec<String>, AiToolError> {
        if let Some(urls) = args.urls.as_ref().filter(|urls| !urls.is_empty()) {
            if urls.len() > MAX_BATCH_URLS {
                return Err(AiToolError::Execution(format!("too many urls: {} exceeds {}", urls.len(), MAX_BATCH_URLS)));
            }
            return Ok(urls.iter().map(|url| url.trim().to_string()).collect());
        }
        match args.url.as_deref().map(str::trim).filter(|url| !url.is_empty()) {
            Some(url) => Ok(vec![url.to_string()]),
            None => Err(AiToolError::Execution("url is empty (single fetch), or pass urls=[...] for batch".to_string())),
        }
    }

    fn push_cancelled(&self, run_context: &AiRunContext, call_id: &str) -> String {
        let result = CANCELLED_RESULT.to_string();
        self.tool_call_done_push.push_tool_call_done_event(run_context, call_id, result.clone());
        result
    }

    fn is_cached(&self, url: &url::Url, raw: bool, refresh: bool) -> bool {
        self.derived_store.lookup(&DerivedStore::web_key(url.as_str(), raw), &DerivedStore::variant_of(None), None, refresh).is_some()
    }

    async fn all_trusted(&self, urls: &[url::Url]) -> bool {
        let Some(config) = self.config_service.select_config().await else {
            return false;
        };
        let trusted_hosts = config.resolve_web_fetch_trusted_hosts();
        urls.iter().all(|url| is_trusted_url(url, &trusted_hosts))
    }

    async fn confirm(&self, run_context: &AiRunContext, call_id: &str, args: &ToolWebFetchArgs, batch: bool, pending: Vec<url::Url>) -> Result<(), AiToolError> {
        if pending.is_empty() {
            return Ok(());
        }
        let confirm_args = ToolWebFetchArgs {
            url: if batch { None } else { args.url.clone() },
            urls: if batch { args.urls.clone() } else { None },
            raw: args.raw,
            offset: args.offset,
            limit: args.limit,
            refresh: None,
            internal_call_id: None,
        };
        if self.all_trusted(&pending).await {
            return self.tool_conflict_manager.record_auto_allowed(run_context, call_id, self.tool_id().as_str(), &confirm_args);
        }
        self.tool_conflict_manager.confirm_before_execute(run_context, call_id, self.tool_id().as_str(), &confirm_args).await
    }

    async fn fetch_one(&self, run_context: &Arc<AiRunContext>, tool_cancel: &CancellationToken, chat_id: Option<&str>, url: url::Url, args: &ToolWebFetchArgs) -> Result<FetchOutcome, AiToolError> {
        let raw = args.raw.unwrap_or(false);
        let variant = DerivedStore::variant_of(None);
        let key = DerivedStore::web_key(url.as_str(), raw);
        let derived_root = DerivedStore::root().map_err(|error| AiToolError::Execution(error.to_string()))?;
        if let Some(cached) = self.derived_store.lookup(&key, &variant, None, args.refresh.unwrap_or(false)) {
            let content = DerivedStore::read_content(&cached).map_err(|error| AiToolError::Execution(error.to_string()))?;
            if let Some(chat_id) = chat_id {
                self.derived_store.record_material(chat_id, &cached);
            }
            let text = present(content, args.offset, args.limit)?;
            return Ok(FetchOutcome::Done(format!("{}\n\n{}", cached.header(&derived_root), text)));
        }
        let source = url.to_string();
        tokio::select! {
            biased;
            _ = tool_cancel.cancelled() => Ok(FetchOutcome::Cancelled),
            _ = run_context.cancel_token.cancelled() => Ok(FetchOutcome::Cancelled),
            res = fetch_url(url, raw) => {
                let body = res?;
                let derived = self
                    .derived_store
                    .save(&key, &variant, DerivedKind::Web, &source, None, None, None, Some(WEB_TTL_SECS), None, &body)
                    .map_err(|error| AiToolError::Execution(format!("failed to save web snapshot: {}", error)))?;
                if let Some(chat_id) = chat_id {
                    self.derived_store.record_material(chat_id, &derived);
                }
                let text = present(body, args.offset, args.limit)?;
                Ok(FetchOutcome::Done(format!("{}\n\n{}", derived.header(&derived_root), text)))
            }
        }
    }
}

impl Tool for ToolWebFetch {
    const NAME: &'static str = AiToolId::ToolWebFetch.as_str();

    type Error = AiToolError;
    type Args = ToolWebFetchArgs;
    type Output = String;

    fn description(&self) -> String {
        "通过 HTTP GET 获取指定 URL 的文本内容。要抓多个网址时一次传 urls 数组（最多 8 个，顺序抓取，按 ===== 网址 ===== 分段返回，单个失败不影响其余），不要一个个来回调用。http 会自动升级为 https。不支持需要登录的私有页面。HTML 页面默认只返回抽取后的正文（标题 + 去掉标签/脚本的文字）；要看网站技术信息（框架、脚本、meta、接口地址）传 raw=true 拿源码。默认返回开头和结尾各一段并省略中间；要看中间就传 offset/limit 按行取（返回里会给出总行数和下一页 offset）。抓到的内容会保存为快照文件（返回开头给出路径，一小时内同一网址直接复用，不再抓取），全文用 file_read 读该路径；要拿最新内容传 refresh=true。只在用户明确要求读某个网址、或必须拿到页面原文才能完成任务时用；禁止为补全 web_search 摘要而逐条抓取搜索结果里的网址——电商与内容平台普遍有反爬，这么做只会连续拿到 403。抓取失败后不要换同一站点的其他网址重试，直接用摘要作答或向用户说明。执行前会请求用户确认（GitHub、MDN、docs.rs 等常见公开文档站点免确认）。".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "要抓取的单个 URL（https）。与 urls 二选一"
                },
                "urls": {
                    "type": "array",
                    "description": "批量抓取时传数组，最多 8 个；与单个参数二选一。raw/offset/limit/refresh 对每个网址同样生效",
                    "maxItems": MAX_BATCH_URLS,
                    "items": { "type": "string" }
                },
                "raw": {
                    "type": "boolean",
                    "description": "true 返回原始 HTML 源码（分析技术栈、脚本、meta、接口时用）；默认 false 只返回抽取的正文"
                },
                "offset": {
                    "type": "integer",
                    "description": "从第几行开始返回（1 起算）。不传则返回开头和结尾各一段并省略中间"
                },
                "limit": {
                    "type": "integer",
                    "description": "从 offset 起最多返回多少行，默认 200；返回超出预算时会提示缩小"
                },
                "refresh": {
                    "type": "boolean",
                    "description": "true 忽略一小时内的快照重新抓取；默认 false"
                }
            }
        })
    }

    async fn call(&self, context: &mut ToolContext, args: Self::Args) -> anyhow::Result<Self::Output, Self::Error> {
        let run_context = context.require::<Arc<AiRunContext>>().map_err(|error| AiToolError::Execution(error.to_string()))?.clone();
        let Some(call_id) = args.internal_call_id.clone() else {
            return Err(AiToolError::Execution("internal_call_id of the tool call is missing".to_string()));
        };
        let Some(tool_cancel) = self.chat_session_cache_manager.get_tool_cancel(&run_context.req_id, &run_context.run_id, &call_id) else {
            return Err(AiToolError::Execution(format!("tool cancel not found: req_id={} run_id={} call_id={}", run_context.req_id, run_context.run_id, call_id)));
        };
        let batch = args.urls.as_ref().is_some_and(|urls| !urls.is_empty());
        let targets = Self::resolve_targets(&args)?;
        let raw = args.raw.unwrap_or(false);
        let refresh = args.refresh.unwrap_or(false);
        let chat_id = self.chat_session_cache_manager.chat_id(&run_context.req_id);

        let pending: Vec<url::Url> = targets.iter().filter_map(|text| resolve_url(text).ok()).filter(|url| !self.is_cached(url, raw, refresh)).collect();
        self.confirm(&run_context, &call_id, &args, batch, pending).await?;

        let mut items: Vec<WrapItem> = Vec::with_capacity(targets.len());
        for text in &targets {
            if tool_cancel.is_cancelled() || run_context.cancel_token.is_cancelled() {
                let result = self.push_cancelled(&run_context, &call_id);
                context.insert_result(vec![WrapItem::wrap_item(None, result.clone())]);
                return Ok(result);
            }
            let outcome = match resolve_url(text) {
                Ok(url) => self.fetch_one(&run_context, &tool_cancel, chat_id.as_deref(), url, &args).await,
                Err(error) => Err(error),
            };
            let body = match outcome {
                Ok(FetchOutcome::Done(body)) => body,
                Ok(FetchOutcome::Cancelled) => {
                    let result = self.push_cancelled(&run_context, &call_id);
                    context.insert_result(vec![WrapItem::wrap_item(None, result.clone())]);
                    return Ok(result);
                }
                Err(error) => format!("<error> {}", error),
            };
            items.push(WrapItem::wrap_item(Some(format!("===== {} =====", text)), body));
        }
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
