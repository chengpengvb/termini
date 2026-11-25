use anyhow::Result;
use std::sync::Arc;
use serde_json::json;
use std::path::PathBuf;
use std::time::Instant;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use summer::tracing::{info, warn};
use crate::base::base_app::BaseApp;
use serde::{Deserialize, Serialize};
use crate::module::ai::dto::AiToolId;
use crate::module::ai::dto::AiRunContext;
use crate::module::ai::shared::ToolCallDonePush;
use rig::tool::{DynamicTool, Tool, ToolContext};
use crate::module::ai::shared::ToolCallChunkPush;
use crate::module::ai::shared::ChatSessionCacheManager;
use crate::module::ai::tools::tool_manager::ToolManager;
use crate::module::ai::tools::ai_tool_error::AiToolError;
use crate::module::ai::tools::common::image::image_prepare;
use crate::module::ai::derived::{DerivedKind, DerivedStore};
use crate::module::ai::tools::base::{ToolBaseOutWrap, WrapItem};
use crate::module::ai::tools::tool_base::{ToolBase, ToolDomain};
use crate::module::ai::tools::common::task::session::task_registry::TaskKind;
use crate::module::ai::tools::common::image::vision_transcriber::{TranscribeSource, TranscribeUnit, VisionTranscriber};

const MAX_BATCH_PATHS: usize = 5;
const MAX_FILE_BYTES: u64 = 10 * 1024 * 1024;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ToolImageReadArgs {
    #[serde(default)]
    pub path: Option<String>,

    #[serde(default)]
    pub paths: Option<Vec<String>>,

    #[serde(default)]
    pub refresh: Option<bool>,

    #[serde(default, rename = "__internal_call_id")]
    pub internal_call_id: Option<String>,
}

#[base_app(sort = 766)]
#[derive(Clone, Service)]
pub struct ToolImageRead {
    #[inject(component)]
    tool_manager: ToolManager,

    #[inject(component)]
    derived_store: DerivedStore,

    #[inject(component)]
    vision_transcriber: VisionTranscriber,

    #[inject(component)]
    tool_call_chunk_push: ToolCallChunkPush,

    #[inject(component)]
    chat_session_cache_manager: ChatSessionCacheManager,
}

#[async_trait]
impl BaseApp for ToolImageRead {
    fn name(&self) -> &'static str {
        "ToolImageRead"
    }

    async fn start(&self, _ctx: &crate::base::base_app::AppContext) -> Result<()> {
        self.tool_manager.register(Arc::new(self.clone()));
        Ok(())
    }
}

impl ToolBase for ToolImageRead {
    fn tool_id(&self) -> AiToolId {
        AiToolId::ToolImageRead
    }

    fn deferred(&self) -> bool {
        true
    }

    fn domains(&self) -> &'static [ToolDomain] {
        &[ToolDomain::Main]
    }

    fn as_rig_tool(&self, done_push: Option<ToolCallDonePush>) -> DynamicTool {
        ToolBaseOutWrap::wrap(self.clone(), self.max_result_tokens(), done_push)
    }
}

impl ToolImageRead {
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
        Ok(raw)
    }

    fn resolve_mime(path: &PathBuf) -> Result<&'static str, AiToolError> {
        let Some(ext) = path.extension().and_then(|value| value.to_str()) else {
            return Err(AiToolError::Execution(format!("no image extension: {}", path.display())));
        };
        match ext.to_ascii_lowercase().as_str() {
            "png" => Ok("image/png"),
            "jpg" | "jpeg" => Ok("image/jpeg"),
            "webp" => Ok("image/webp"),
            "gif" => Ok("image/gif"),
            other => Err(AiToolError::Execution(format!("unsupported image type: {}", other))),
        }
    }

    fn resolve_targets(args: &ToolImageReadArgs) -> Result<Vec<String>, AiToolError> {
        if let Some(paths) = args.paths.as_ref().filter(|paths| !paths.is_empty()) {
            if paths.len() > MAX_BATCH_PATHS {
                return Err(AiToolError::Execution(format!(
                    "image_read: paths 传了 {} 张, 一次最多 {} 张。识别一张要跑一次视觉模型, 不要把整份文档每页都转成图片再读: 只抽封面或出问题的那一页, 其余内容用 docx_read/pdf_read/xlsx_read 读文字",
                    paths.len(),
                    MAX_BATCH_PATHS
                )));
            }
            return Ok(paths.clone());
        }
        match args.path.as_deref().map(str::trim).filter(|path| !path.is_empty()) {
            Some(path) => Ok(vec![path.to_string()]),
            None => Err(AiToolError::Execution("path is empty (single image), or pass paths=[...] for batch".to_string())),
        }
    }

    fn push_section_head(&self, run_context: &AiRunContext, call_id: &str, title: &str) {
        if let Err(error) = self.tool_call_chunk_push.push(run_context, call_id, AiToolId::ToolImageRead.as_str(), format!("{}\n", title)) {
            warn!("[ai] 推送分段头失败 call_id={} error={:#}", call_id, error);
        }
    }

    async fn read_one(&self, run_context: &Arc<AiRunContext>, call_id: &str, chat_id: &str, path_text: &str, refresh: bool) -> Result<String, AiToolError> {
        let path = Self::resolve_path(path_text)?;
        let mime = Self::resolve_mime(&path)?;
        let meta = std::fs::metadata(&path).map_err(|error| AiToolError::Execution(error.to_string()))?;
        if meta.len() > MAX_FILE_BYTES {
            return Err(AiToolError::Execution(format!("file too large (max {} bytes): {}", MAX_FILE_BYTES, path.display())));
        }
        let bytes = tokio::fs::read(&path).await.map_err(|error| AiToolError::Execution(format!("read {}: {}", path.display(), error)))?;
        let source_hash = DerivedStore::content_hash(&bytes);
        let key = DerivedStore::file_key(&path);
        let variant = DerivedStore::variant_of(None);
        let derived_root = DerivedStore::root().map_err(|error| AiToolError::Execution(error.to_string()))?;
        if let Some(cached) = self.derived_store.lookup(&key, &variant, Some(&source_hash), refresh) {
            let content = DerivedStore::read_content(&cached).map_err(|error| AiToolError::Execution(error.to_string()))?;
            info!("[ai] 命中派生缓存 path={} derived={} lines={}", path.display(), cached.path.display(), cached.meta.lines);
            self.derived_store.record_material(chat_id, &cached);
            return Ok(format!("{}\n\n{}", cached.header(&derived_root), content));
        }

        let started = Instant::now();
        let source_len = bytes.len() as u64;
        let options = self.vision_transcriber.prepare_options().await?;
        let prepared = tokio::task::spawn_blocking(move || image_prepare::prepare(bytes, mime, options)).await.map_err(|error| AiToolError::Execution(error.to_string()))?;
        let millis = started.elapsed().as_millis();
        let tiles = prepared.images.len();
        let total_bytes = prepared.total_bytes();
        let path_text = path.display().to_string();
        info!("[ai] 图片预处理 path={} {}→{} bytes 片数={} resized={} tiled={} 耗时={}ms", path_text, source_len, total_bytes, tiles, prepared.resized, prepared.tiled, millis);
        let units = vec![TranscribeUnit { label: None, images: prepared.images }];
        let source = TranscribeSource { kind: DerivedKind::Image, task_kind: TaskKind::ImageRead, key, variant, label: None, path, source_hash, source_len, units };
        self.vision_transcriber.transcribe(run_context, call_id, chat_id.to_string(), source).await
    }
}

impl Tool for ToolImageRead {
    const NAME: &'static str = AiToolId::ToolImageRead.as_str();

    type Args = ToolImageReadArgs;
    type Output = String;
    type Error = AiToolError;

    fn description(&self) -> String {
        format!("读取本应用所在宿主机上的图片文件，由服务端配置的视觉模型逐行识别：保留图片中的换行和空行，非文字区域在原位置写成「[图: 描述]」。结果放在 text 代码块中，不做 Markdown 解释、总结或问答。**一次最多 {} 张，超了直接报错**：识别一张要跑一次视觉模型，几秒到一分钟不等，它不是便宜的批量工具。刚生成的 Word/PDF/Excel/PPT 一律不要把每一页都转成图片再识别，也不要为此另写导出脚本：只抽封面或出问题的那一页，其余内容用 docx_read/pdf_read/xlsx_read 读文字。确实要读多张时一次传 paths 数组（顺序识别，按 ===== 路径 ===== 分段返回，单张失败不影响其余）。支持 png/jpg/jpeg/webp/gif，仅接受绝对路径，文件最大 10MB。完整结果保存为派生 Markdown，返回头部给出路径且正文只返回受控窗口；更多内容用 file_read 的 offset/limit，需要跨文件搜索用 code_search/code_semantic。同一张未变化的图片直接复用结果，refresh=true 强制重识别。长图逐片处理且每片完成即保存断点；失败或取消后再次调用会从断点继续。超过约 90 秒转后台并返回 task_id，用 task_log/task_kill/task_wait 监督。重复输出会自动终止，重新调用即可继续。用户随消息上传的图片路径位于 [Attached Images]；其中标注「已附在消息中」的图片你已能直接查看，只在需要逐行精确转写、保持原始排版或图片文字密集看不清时才用本工具，其余未附带的图片一律通过本工具读取。", MAX_BATCH_PATHS)
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "单张图片文件的绝对路径。与 paths 二选一" },
                "paths": {
                    "type": "array",
                    "description": format!("批量读取时传数组，一次最多 {} 个，超了会被拒；与单个参数二选一", MAX_BATCH_PATHS),
                    "maxItems": MAX_BATCH_PATHS,
                    "items": { "type": "string" }
                },
                "refresh": { "type": "boolean", "description": "true 忽略已完成的缓存结果重新识别；默认 false" }
            }
        })
    }

    async fn call(&self, context: &mut ToolContext, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let run_context = context.require::<Arc<AiRunContext>>().map_err(|error| AiToolError::Execution(error.to_string()))?.clone();
        let Some(call_id) = args.internal_call_id.clone() else {
            return Err(AiToolError::Execution("internal_call_id of the tool call is missing".to_string()));
        };
        let Some(chat_id) = self.chat_session_cache_manager.chat_id(&run_context.req_id) else {
            return Err(AiToolError::Execution(format!("unknown req_id: {}", run_context.req_id)));
        };
        let refresh = args.refresh.unwrap_or(false);
        let targets = Self::resolve_targets(&args)?;
        let mut items: Vec<WrapItem> = Vec::with_capacity(targets.len());
        for path_text in &targets {
            if run_context.cancel_token.is_cancelled() {
                return Err(AiToolError::Cancelled("image_read batch cancelled".to_string()));
            }
            let title = format!("===== {} =====", path_text);
            self.push_section_head(&run_context, &call_id, &title);
            items.push(match self.read_one(&run_context, &call_id, &chat_id, path_text, refresh).await {
                Ok(body) => WrapItem::wrap_item(Some(title), body),
                Err(AiToolError::Cancelled(reason)) => return Err(AiToolError::Cancelled(reason)),
                Err(error) => WrapItem::wrap_item(Some(title), format!("<error> {}", error)),
            });
        }
        context.insert_result(items);
        Ok(String::new())
    }
}
