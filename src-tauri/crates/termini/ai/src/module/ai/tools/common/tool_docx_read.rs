use anyhow::Result;
use std::sync::Arc;
use serde_json::json;
use std::path::PathBuf;
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
use crate::module::ai::derived::{DerivedKind, DerivedStore};
use crate::module::ai::tools::base::{ToolBaseOutWrap, WrapItem};
use crate::module::ai::tools::tool_base::{ToolBase, ToolDomain};
use crate::module::ai::tools::common::document::pandoc::PandocConverter;

const MAX_LIMIT: usize = 2000;
const DEFAULT_LIMIT: usize = 200;
const MAX_BATCH_FILES: usize = 8;
const MAX_FILE_BYTES: u64 = 32 * 1024 * 1024;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct DocxReadSpec {
    pub path: String,

    #[serde(default)]
    pub offset: Option<usize>,

    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ToolDocxReadArgs {
    #[serde(default)]
    pub path: Option<String>,

    #[serde(default)]
    pub offset: Option<usize>,

    #[serde(default)]
    pub limit: Option<usize>,

    #[serde(default)]
    pub files: Option<Vec<DocxReadSpec>>,

    #[serde(default)]
    pub refresh: Option<bool>,
}

#[base_app(sort = 766)]
#[derive(Clone, Service)]
pub struct ToolDocxRead {
    #[inject(component)]
    tool_manager: ToolManager,

    #[inject(component)]
    derived_store: DerivedStore,

    #[inject(component)]
    pandoc_converter: PandocConverter,

    #[inject(component)]
    chat_session_cache_manager: ChatSessionCacheManager,
}

#[async_trait]
impl BaseApp for ToolDocxRead {
    fn name(&self) -> &'static str {
        "ToolDocxRead"
    }

    async fn start(&self, _ctx: &crate::base::base_app::AppContext) -> Result<()> {
        self.tool_manager.register(Arc::new(self.clone()));
        Ok(())
    }
}

impl ToolBase for ToolDocxRead {
    fn tool_id(&self) -> AiToolId {
        AiToolId::ToolDocxRead
    }

    fn platform_supported(&self) -> bool {
        !cfg!(any(target_os = "ios", target_os = "android"))
    }

    fn deferred(&self) -> bool {
        true
    }

    fn domains(&self) -> &'static [ToolDomain] {
        ToolDomain::MAIN_READ_ONLY
    }

    fn as_rig_tool(&self, done_push: Option<ToolCallDonePush>) -> DynamicTool {
        ToolBaseOutWrap::wrap(self.clone(), self.max_result_tokens(), done_push)
    }
}

impl ToolDocxRead {
    fn resolve_path(path: &str) -> Result<PathBuf, AiToolError> {
        if path.trim().is_empty() {
            return Err(AiToolError::Execution("path is empty".to_string()));
        }
        let raw = PathBuf::from(path);
        if !raw.is_absolute() {
            return Err(AiToolError::Execution(format!("path must be absolute: {}", path)));
        }
        if !raw.exists() {
            return Err(AiToolError::Execution(format!("file not found: {}", raw.display())));
        }
        if !raw.is_file() {
            return Err(AiToolError::Execution(format!("not a file: {}", raw.display())));
        }
        if !raw.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("docx")) {
            return Err(AiToolError::Execution(format!("not a docx file: {}", raw.display())));
        }
        Ok(raw)
    }

    fn window_lines(full: &str, offset: usize, limit: usize) -> String {
        let lines: Vec<&str> = full.lines().collect();
        let total = lines.len();
        if offset >= total {
            return String::new();
        }
        let end = (offset + limit).min(total);
        lines[offset..end].join("\n")
    }

    fn resolve_specs(args: &ToolDocxReadArgs) -> Result<Vec<DocxReadSpec>, AiToolError> {
        if let Some(files) = args.files.as_ref().filter(|files| !files.is_empty()) {
            if files.len() > MAX_BATCH_FILES {
                return Err(AiToolError::Execution(format!("too many files: {} exceeds {}", files.len(), MAX_BATCH_FILES)));
            }
            return Ok(files.clone());
        }
        match args.path.clone().filter(|path| !path.trim().is_empty()) {
            Some(path) => Ok(vec![DocxReadSpec { path, offset: args.offset, limit: args.limit }]),
            None => Err(AiToolError::Execution("path is empty (single file), or pass files=[...] for batch".to_string())),
        }
    }

    async fn read_one(&self, run_context: &Arc<AiRunContext>, spec: &DocxReadSpec, refresh: bool) -> Result<String, AiToolError> {
        let path = Self::resolve_path(&spec.path)?;
        let meta = std::fs::metadata(&path).map_err(|error| AiToolError::Execution(error.to_string()))?;
        if meta.len() > MAX_FILE_BYTES {
            return Err(AiToolError::Execution(format!("file too large (max {} bytes): {}", MAX_FILE_BYTES, path.display())));
        }
        let offset = spec.offset.unwrap_or(0);
        let limit = spec.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
        let media_dir = DerivedStore::dir_for(&DerivedStore::file_key(&path)).map_err(|error| AiToolError::Execution(error.to_string()))?;
        let converter = self.pandoc_converter.clone();
        let source = path.clone();
        let (derived, full) = self
            .derived_store
            .ensure_file_document_with(DerivedKind::Docx, &path, refresh, || async move { converter.docx_to_markdown(&source, &media_dir).await.map_err(|error| error.to_string()) })
            .await
            .map_err(|error| AiToolError::Execution(error.to_string()))?;
        if let Some(chat_id) = self.chat_session_cache_manager.chat_id(&run_context.req_id) {
            self.derived_store.record_material(&chat_id, &derived);
        }
        let derived_root = DerivedStore::root().map_err(|error| AiToolError::Execution(error.to_string()))?;
        let text = Self::window_lines(&full, offset, limit);
        Ok(format!("{}\n\n{}", derived.header(&derived_root), text))
    }
}

impl Tool for ToolDocxRead {
    const NAME: &'static str = AiToolId::ToolDocxRead.as_str();

    type Args = ToolDocxReadArgs;
    type Output = String;
    type Error = AiToolError;

    fn description(&self) -> String {
        "读取本应用所在宿主机上的 Word .docx 文件，用 pandoc 转成 Markdown：标题、列表、表格、粗体/斜体、段内换行都按原文档结构保留，图片抽到派生目录并以相对路径引用。可用 offset（0-based 行号）和 limit（默认 200 行，最大 2000）分段读取。要读多份 docx 时一次传 files 数组（最多 8 个，每项可各自指定 offset/limit，顺序读取，按 ===== 路径 ===== 分段返回，单个失败不影响其余），不要一个个来回调用。首次读取会把整份 Markdown 保存为派生全文（返回开头给出路径），之后翻页、用 code_search/code_semantic 找内容可直接对那个路径操作；文件内容变了自动重新转换，也可传 refresh=true 强制。首次使用会自动下载 pandoc（几十 MB）。".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "单个 .docx 文件绝对路径。与 files 二选一" },
                "offset": { "type": "integer", "description": "起始行号 0-based，默认 0" },
                "limit": { "type": "integer", "description": "最多返回行数，默认 200，最大 2000" },
                "files": {
                    "type": "array",
                    "description": "批量读取时传数组，最多 8 个；与单个参数二选一。refresh 对每个文件同样生效",
                    "maxItems": MAX_BATCH_FILES,
                    "items": {
                        "type": "object",
                        "properties": {
                            "path": { "type": "string", "description": ".docx 文件绝对路径" },
                            "offset": { "type": "integer", "description": "起始行号 0-based，默认 0" },
                            "limit": { "type": "integer", "description": "最多返回行数，默认 200，最大 2000" }
                        },
                        "required": ["path"]
                    }
                },
                "refresh": { "type": "boolean", "description": "true 忽略已保存的全文重新转换；默认 false" }
            }
        })
    }

    async fn call(&self, context: &mut ToolContext, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let run_context = context.require::<Arc<AiRunContext>>().map_err(|error| AiToolError::Execution(error.to_string()))?.clone();
        let refresh = args.refresh.unwrap_or(false);
        let specs = Self::resolve_specs(&args)?;
        let mut items: Vec<WrapItem> = Vec::with_capacity(specs.len());
        for spec in &specs {
            if run_context.cancel_token.is_cancelled() {
                return Err(AiToolError::Cancelled("docx_read batch cancelled".to_string()));
            }
            items.push(match self.read_one(&run_context, spec, refresh).await {
                Ok(body) => WrapItem::wrap_item(Some(format!("===== {} =====", spec.path)), body),
                Err(AiToolError::Cancelled(reason)) => return Err(AiToolError::Cancelled(reason)),
                Err(error) => WrapItem::wrap_item(Some(format!("===== {} =====", spec.path)), format!("<error> {}", error)),
            });
        }
        context.insert_result(items);
        Ok(String::new())
    }
}
