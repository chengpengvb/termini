use anyhow::Result;
use std::sync::Arc;
use serde_json::json;
use summer::tracing::info;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use std::path::{Path, PathBuf};
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
use crate::module::ai::tools::common::document::external_tool::ExternalTool;
use crate::module::ai::tools::common::task::session::task_registry::TaskKind;
use crate::module::ai::tools::common::document::tool_installer::ToolInstaller;
use crate::module::ai::tools::common::image::vision_transcriber::{TranscribeSource, VisionTranscriber};
use crate::module::ai::tools::common::pdf::pdfium_text::{extract_text_via_pdfium, render_pages_via_pdfium, resolve_pdf_path, window_from_full, FULL_EXTRACT_MAX_PAGES};

const MAX_BATCH_FILES: usize = 8;
const MAX_PAGES_READ: usize = 50;
const MAX_VISION_PAGES: usize = 40;
const DEFAULT_PAGES_READ: usize = 10;
const LAYOUT_QUESTION: &str = "视觉版面转写";
const MAX_FILE_BYTES: u64 = 32 * 1024 * 1024;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct PdfReadSpec {
    pub path: String,

    #[serde(default)]
    pub offset: Option<usize>,

    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ToolPdfReadArgs {
    #[serde(default)]
    pub path: Option<String>,

    #[serde(default)]
    pub offset: Option<usize>,

    #[serde(default)]
    pub limit: Option<usize>,

    #[serde(default)]
    pub refresh: Option<bool>,

    #[serde(default)]
    pub layout: Option<bool>,

    #[serde(default)]
    pub files: Option<Vec<PdfReadSpec>>,

    #[serde(default, rename = "__internal_call_id")]
    pub internal_call_id: Option<String>,
}

#[base_app(sort = 766)]
#[derive(Clone, Service)]
pub struct ToolPdfRead {
    #[inject(component)]
    tool_manager: ToolManager,

    #[inject(component)]
    derived_store: DerivedStore,

    #[inject(component)]
    tool_installer: ToolInstaller,

    #[inject(component)]
    vision_transcriber: VisionTranscriber,

    #[inject(component)]
    chat_session_cache_manager: ChatSessionCacheManager,
}

#[async_trait]
impl BaseApp for ToolPdfRead {
    fn name(&self) -> &'static str {
        "ToolPdfRead"
    }

    async fn start(&self, _ctx: &crate::base::base_app::AppContext) -> Result<()> {
        self.tool_manager.register(Arc::new(self.clone()));
        Ok(())
    }
}

impl ToolBase for ToolPdfRead {
    fn tool_id(&self) -> AiToolId {
        AiToolId::ToolPdfRead
    }

    fn deferred(&self) -> bool {
        true
    }

    fn platform_supported(&self) -> bool {
        !cfg!(any(target_os = "ios", target_os = "android"))
    }

    fn domains(&self) -> &'static [ToolDomain] {
        ToolDomain::MAIN_READ_ONLY
    }

    fn as_rig_tool(&self, done_push: Option<ToolCallDonePush>) -> DynamicTool {
        ToolBaseOutWrap::wrap(self.clone(), self.max_result_tokens(), done_push)
    }
}

impl ToolPdfRead {
    async fn read_layout(&self, run_context: &Arc<AiRunContext>, call_id: &str, chat_id: String, dll_path: PathBuf, path: PathBuf, refresh: bool) -> Result<String, AiToolError> {
        let bytes = tokio::fs::read(&path).await.map_err(|error| AiToolError::Execution(format!("read {}: {}", path.display(), error)))?;
        let source_hash = DerivedStore::content_hash(&bytes);
        let source_len = bytes.len() as u64;
        drop(bytes);
        let key = DerivedStore::file_key(&path);
        let variant = DerivedStore::variant_of(Some(LAYOUT_QUESTION));
        let derived_root = DerivedStore::root().map_err(|error| AiToolError::Execution(error.to_string()))?;
        if let Some(cached) = self.derived_store.lookup(&key, &variant, Some(&source_hash), refresh) {
            let content = DerivedStore::read_content(&cached).map_err(|error| AiToolError::Execution(error.to_string()))?;
            info!("[ai] 命中版面识别缓存 path={} derived={} lines={}", path.display(), cached.path.display(), cached.meta.lines);
            self.derived_store.record_material(&chat_id, &cached);
            return Ok(format!("{}\n\n{}", cached.header(&derived_root), content));
        }
        let options = self.vision_transcriber.prepare_options().await?;
        let render_path = path.clone();
        let (units, total) =
            tokio::task::spawn_blocking(move || render_pages_via_pdfium(&dll_path, &render_path, MAX_VISION_PAGES, options)).await.map_err(|error| AiToolError::Execution(error.to_string()))??;
        info!("[ai] 渲染页面送视觉识别 path={} pages={} rendered={}", path.display(), total, units.len());
        let source =
            TranscribeSource { kind: DerivedKind::Pdf, task_kind: TaskKind::PdfRead, key, variant, label: Some(LAYOUT_QUESTION.to_string()), path: path.clone(), source_hash, source_len, units };
        let mut result = self.vision_transcriber.transcribe(run_context, call_id, chat_id, source).await?;
        if total > MAX_VISION_PAGES {
            result.push_str(&format!("\n\n[pdf_read] 该 PDF 共 {} 页, 版面识别只处理了前 {} 页; 更多页请拆分文件后再读。", total, MAX_VISION_PAGES));
        }
        Ok(result)
    }
}

impl Tool for ToolPdfRead {
    const NAME: &'static str = AiToolId::ToolPdfRead.as_str();

    type Args = ToolPdfReadArgs;
    type Output = String;
    type Error = AiToolError;

    fn description(&self) -> String {
        "读取本应用所在宿主机上的 PDF 文件。仅接受 .pdf 后缀、绝对路径；文件最大 32MB。要读多份 PDF 时一次传 files 数组（最多 8 个，每项可各自指定 offset/limit，顺序读取，按 ===== 路径 ===== 分段返回，单个失败不影响其余），不要一个个来回调用。默认提取文字层：每页一个 --- page N --- 标记，页内文字放在 text 代码块里逐行原样保留（PDF 里怎么换行就怎么换行）；可用 offset（从 1 起的页码）与 limit（默认 10 页，上限 50 页）分页读取。首次读取会把整份保存为 Markdown 全文（返回开头给出路径），之后翻页、用 code_search/code_semantic 找内容直接对那个路径操作；文件内容变了自动重新解析，也可传 refresh=true 强制。文字层抽不出字（扫描件、图片型 PDF）时会自动把每页渲染成图交给视觉模型按版面 1:1 识别，结果与 image_read 相同格式（每页 --- page N --- 加 text 代码块，同一行内容仍在同一行）；表格、多栏等版面复杂、文字层读出来错乱时，可传 layout=true 主动走视觉识别。视觉识别最多处理前 40 页，每页完成即落盘、失败或打断后重新调用从断点继续；超过约 90 秒会转后台并返回 task_id（task_log 看进度、task_kill 打断、task_wait 等结果）。首次调用会自动下载 pdfium 引擎。".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "单个 PDF 文件的绝对路径。与 files 二选一" },
                "offset": { "type": "integer", "description": "起始页码，从 1 开始；不传则从第 1 页开始（文字层模式）" },
                "limit": { "type": "integer", "description": "最多读取的页数；不传则读 10 页，上限 50 页（文字层模式）" },
                "files": {
                    "type": "array",
                    "description": "批量读取时传数组，最多 8 个；与单个参数二选一。refresh/layout 对每个文件同样生效",
                    "maxItems": MAX_BATCH_FILES,
                    "items": {
                        "type": "object",
                        "properties": {
                            "path": { "type": "string", "description": "PDF 文件的绝对路径" },
                            "offset": { "type": "integer", "description": "起始页码，从 1 开始" },
                            "limit": { "type": "integer", "description": "最多读取的页数，默认 10，上限 50" }
                        },
                        "required": ["path"]
                    }
                },
                "refresh": { "type": "boolean", "description": "true 忽略已保存的全文重新解析；默认 false" },
                "layout": { "type": "boolean", "description": "true 不用文字层，把每页渲染成图交给视觉模型按版面转写；默认 false，扫描件会自动走这条路" }
            }
        })
    }

    async fn call(&self, context: &mut ToolContext, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let run_context = context.require::<Arc<AiRunContext>>().map_err(|error| AiToolError::Execution(error.to_string()))?.clone();
        let Some(chat_id) = self.chat_session_cache_manager.chat_id(&run_context.req_id) else {
            return Err(AiToolError::Execution(format!("unknown req_id: {}", run_context.req_id)));
        };
        let refresh = args.refresh.unwrap_or(false);
        let layout = args.layout.unwrap_or(false);
        let specs = Self::resolve_specs(&args)?;
        let dll_path = self.tool_installer.ensure(ExternalTool::Pdfium).await?;
        let mut items: Vec<WrapItem> = Vec::with_capacity(specs.len());
        for spec in &specs {
            if run_context.cancel_token.is_cancelled() {
                return Err(AiToolError::Cancelled("pdf_read batch cancelled".to_string()));
            }
            items.push(match self.read_one(&run_context, args.internal_call_id.as_deref(), &chat_id, &dll_path, spec, refresh, layout).await {
                Ok(body) => WrapItem::wrap_item(Some(format!("===== {} =====", spec.path)), body),
                Err(AiToolError::Cancelled(reason)) => return Err(AiToolError::Cancelled(reason)),
                Err(error) => WrapItem::wrap_item(Some(format!("===== {} =====", spec.path)), format!("<error> {}", error)),
            });
        }
        context.insert_result(items);
        Ok(String::new())
    }
}

impl ToolPdfRead {
    fn resolve_specs(args: &ToolPdfReadArgs) -> Result<Vec<PdfReadSpec>, AiToolError> {
        if let Some(files) = args.files.as_ref().filter(|files| !files.is_empty()) {
            if files.len() > MAX_BATCH_FILES {
                return Err(AiToolError::Execution(format!("too many files: {} exceeds {}", files.len(), MAX_BATCH_FILES)));
            }
            return Ok(files.clone());
        }
        match args.path.clone().filter(|path| !path.trim().is_empty()) {
            Some(path) => Ok(vec![PdfReadSpec { path, offset: args.offset, limit: args.limit }]),
            None => Err(AiToolError::Execution("path is empty (single file), or pass files=[...] for batch".to_string())),
        }
    }

    #[allow(clippy::too_many_arguments)]
    async fn read_one(&self, run_context: &Arc<AiRunContext>, call_id: Option<&str>, chat_id: &str, dll_path: &Path, spec: &PdfReadSpec, refresh: bool, layout: bool) -> Result<String, AiToolError> {
        let path = resolve_pdf_path(&spec.path)?;
        let meta = std::fs::metadata(&path).map_err(|error| AiToolError::Execution(error.to_string()))?;
        if meta.len() > MAX_FILE_BYTES {
            return Err(AiToolError::Execution(format!("file too large (max {} bytes): {}", MAX_FILE_BYTES, path.display())));
        }
        if layout {
            let Some(call_id) = call_id else {
                return Err(AiToolError::Execution("internal_call_id of the tool call is missing".to_string()));
            };
            return Ok(self.read_layout(run_context, call_id, chat_id.to_string(), dll_path.to_path_buf(), path, refresh).await?);
        }
        let start_page = spec.offset.unwrap_or(1).saturating_sub(1);
        let page_limit = spec.limit.unwrap_or(DEFAULT_PAGES_READ).min(MAX_PAGES_READ);
        let bytes = tokio::fs::read(&path).await.map_err(|error| AiToolError::Execution(format!("read {}: {}", path.display(), error)))?;
        let source_hash = DerivedStore::content_hash(&bytes);
        let source_len = bytes.len() as u64;
        drop(bytes);
        let key = DerivedStore::file_key(&path);
        let variant = DerivedStore::variant_of(None);
        let (derived, full) = match self.derived_store.lookup(&key, &variant, Some(&source_hash), refresh) {
            Some(cached) => {
                let content = DerivedStore::read_content(&cached).map_err(|error| AiToolError::Execution(error.to_string()))?;
                (cached, content)
            }
            None => {
                let probe_dll = dll_path.to_path_buf();
                let probe_path = path.clone();
                let extract = tokio::task::spawn_blocking(move || extract_text_via_pdfium(&probe_dll, &probe_path, 0, FULL_EXTRACT_MAX_PAGES))
                    .await
                    .map_err(|error| AiToolError::Execution(error.to_string()))??;
                if extract.looks_scanned() {
                    let Some(call_id) = call_id else {
                        return Err(AiToolError::Execution("this PDF has no text layer and needs vision transcription, but the internal_call_id of the tool call is missing".to_string()));
                    };
                    info!("[ai] 文字层几乎为空, 改走视觉版面转写 path={} pages={} chars={}", path.display(), extract.pages, extract.chars);
                    return Ok(self.read_layout(run_context, call_id, chat_id.to_string(), dll_path.to_path_buf(), path, refresh).await?);
                }
                let derived = self
                    .derived_store
                    .save(&key, &variant, DerivedKind::Pdf, &path.to_string_lossy(), Some(source_hash), Some(source_len), None, None, None, &extract.text)
                    .map_err(|error| AiToolError::Execution(error.to_string()))?;
                (derived, extract.text)
            }
        };
        self.derived_store.record_material(chat_id, &derived);
        let derived_root = DerivedStore::root().map_err(|error| AiToolError::Execution(error.to_string()))?;
        let window = window_from_full(&full, start_page, page_limit);
        Ok(format!("{}\n\n{}", derived.header(&derived_root), window.text))
    }
}
