use anyhow::Result;
use std::sync::Arc;
use std::path::Path;
use serde_json::json;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use crate::base::base_app::BaseApp;
use serde::{Deserialize, Serialize};
use crate::module::ai::dto::AiToolId;
use crate::module::ai::shared::ToolCallDonePush;
use rig::tool::{DynamicTool, Tool, ToolContext};
use crate::module::ai::derived::{Derived, DerivedStore};
use crate::module::ai::tools::tool_manager::ToolManager;
use crate::module::ai::tools::ai_tool_error::AiToolError;
use crate::module::ai::tools::base::{ToolBaseOutWrap, WrapItem};
use crate::module::ai::tools::tool_base::{ToolBase, ToolDomain};

const MAX_SOURCES: usize = 50;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ToolDerivedListArgs {
    #[serde(default)]
    pub source: Option<String>,

    #[serde(default)]
    pub sources: Vec<String>,
}

#[base_app(sort = 766)]
#[derive(Clone, Service)]
pub struct ToolDerivedList {
    #[inject(component)]
    tool_manager: ToolManager,

    #[inject(component)]
    derived_store: DerivedStore,
}

#[async_trait]
impl BaseApp for ToolDerivedList {
    fn name(&self) -> &'static str {
        "ToolDerivedList"
    }

    async fn start(&self, _ctx: &crate::base::base_app::AppContext) -> Result<()> {
        self.tool_manager.register(Arc::new(self.clone()));
        Ok(())
    }
}

impl ToolBase for ToolDerivedList {
    fn tool_id(&self) -> AiToolId {
        AiToolId::ToolDerivedList
    }

    fn deferred(&self) -> bool {
        true
    }

    fn domains(&self) -> &'static [ToolDomain] {
        ToolDomain::READ_ONLY
    }

    fn as_rig_tool(&self, done_push: Option<ToolCallDonePush>) -> DynamicTool {
        ToolBaseOutWrap::wrap(self.clone(), self.max_result_tokens(), done_push)
    }
}

impl ToolDerivedList {
    fn is_url(source: &str) -> bool {
        let lower = source.trim().to_ascii_lowercase();
        lower.starts_with("http://") || lower.starts_with("https://")
    }

    fn collect_sources(args: &ToolDerivedListArgs) -> Result<Vec<String>, AiToolError> {
        let mut out: Vec<String> = Vec::new();
        for raw in args.source.iter().chain(args.sources.iter()) {
            let trimmed = raw.trim();
            if trimmed.is_empty() || out.iter().any(|item| item == trimmed) {
                continue;
            }
            out.push(trimmed.to_string());
        }
        if out.is_empty() {
            return Err(AiToolError::Execution("source is empty".to_string()));
        }
        if out.len() > MAX_SOURCES {
            return Err(AiToolError::Execution(format!("sources allows at most {} entries, got {}", MAX_SOURCES, out.len())));
        }
        Ok(out)
    }

    fn wrap_source(source: &str, items: &[Derived], stale: usize) -> WrapItem {
        let body = Self::render_source(items, stale);
        if items.is_empty() {
            return WrapItem::wrap_item(Some(format!("===== {} =====", source)), body);
        }
        WrapItem::wrap_item(Some(format!("===== {} ({} 份) =====", source, items.len())), body)
    }

    fn render_source(items: &[Derived], stale: usize) -> String {
        if items.is_empty() {
            let note = if stale > 0 { format!(" ({} stale results predate the file change and no longer apply)", stale) } else { String::new() };
            return format!("no usable saved results{}\n", note);
        }
        let mut out = String::new();
        for (index, item) in items.iter().enumerate() {
            let question = item.meta.question.as_deref().unwrap_or("(完整转写/描述)");
            let model = item.meta.model.as_deref().unwrap_or("-");
            out.push_str(&format!(
                "{}. [{}] 提示词「{}」 → {} ({} 行, 约 {} tokens, 模型 {}, 生成于 {})\n   {}\n",
                index + 1,
                item.meta.kind.label(),
                question,
                item.path.display(),
                item.meta.lines,
                item.meta.tokens,
                model,
                format_time(item.meta.created_ms),
                item.meta.gist
            ));
        }
        if stale > 0 {
            out.push_str(&format!("另有 {} 份是文件改动前的旧结果, 已不适用。\n", stale));
        }
        out
    }

    async fn list_source(&self, source: &str) -> Result<WrapItem, AiToolError> {
        if Self::is_url(source) {
            let mut items = self.derived_store.list_variants(&DerivedStore::web_key(source, false), None);
            items.extend(self.derived_store.list_variants(&DerivedStore::web_key(source, true), None));
            return Ok(Self::wrap_source(source, &items, 0));
        }
        let path = Path::new(source);
        if !path.is_absolute() {
            return Err(AiToolError::Execution(format!("path must be absolute: {}", source)));
        }
        let all = self.derived_store.list_variants(&DerivedStore::file_key(path), None);
        let current_hash = match tokio::fs::read(path).await {
            Ok(bytes) => Some(DerivedStore::content_hash(&bytes)),
            Err(_) => None,
        };
        let (fresh, stale): (Vec<Derived>, Vec<Derived>) = match current_hash.as_deref() {
            Some(hash) => all.into_iter().partition(|item| item.meta.source_hash.as_deref() == Some(hash)),
            None => (all, Vec::new()),
        };
        Ok(Self::wrap_source(source, &fresh, stale.len()))
    }
}

fn format_time(ms: i64) -> String {
    chrono::DateTime::from_timestamp_millis(ms).map(|time| time.with_timezone(&chrono::Local).format("%Y-%m-%d %H:%M").to_string()).unwrap_or_else(|| ms.to_string())
}

impl Tool for ToolDerivedList {
    const NAME: &'static str = AiToolId::ToolDerivedList.as_str();

    type Args = ToolDerivedListArgs;
    type Output = String;
    type Error = AiToolError;

    fn description(&self) -> String {
        format!("查看文件（图片/PDF/Word/Excel，本机绝对路径）或网址之前已经保存过哪些解析、识别结果：每份给出当时的提示词、路径、行数、模型和生成时间。跨对话复用识别结果时先调它——之前识别过的图片不必再花时间重新识别，直接 file_read 列出的路径；只有没有结果、或结果不满足当前问题时再调用对应的读取工具（需要重新识别加 refresh=true）。**多个文件用 sources 一次查完（最多 {} 个），不要每个文件调一次。**", MAX_SOURCES)
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "source": {
                    "type": "string",
                    "description": "单个来源：本机文件绝对路径，或 http(s) 网址"
                },
                "sources": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "多个来源一次查询；与 source 二选一或同时传"
                }
            }
        })
    }

    async fn call(&self, context: &mut ToolContext, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let sources = Self::collect_sources(&args)?;
        let derived_root = DerivedStore::root().map_err(|error| AiToolError::Execution(error.to_string()))?;
        let mut items: Vec<WrapItem> = Vec::with_capacity(sources.len());
        for source in &sources {
            items.push(self.list_source(source).await?);
        }
        let last = items.len() - 1;
        items[last].body.push_str(&format!(
            "\n能用就直接 file_read 对应路径(按 offset/limit 翻页), 或用 code_search root={} 找; 只有内容不够或问题不同时才重新调用读取工具(图片换问题会各存一份, 传 refresh=true 强制重识别)。",
            derived_root.display()
        ));
        context.insert_result(items);
        Ok(String::new())
    }
}
