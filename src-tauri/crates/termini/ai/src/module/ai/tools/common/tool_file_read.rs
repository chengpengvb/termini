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
use crate::module::ai::shared::ToolCallDonePush;
use rig::tool::{DynamicTool, Tool, ToolContext};
use crate::module::ai::tools::tool_manager::ToolManager;
use crate::module::ai::tools::ai_tool_error::AiToolError;
use crate::model::req::req_chat_reference::REQ_REF_PREFIX;
use crate::module::ai::tools::base::{ToolBaseOutWrap, WrapItem};
use crate::module::ai::tools::tool_base::{ToolBase, ToolDomain};

const MAX_BATCH_FILES: usize = 20;
const MAX_LINES_READ: usize = 1000;
const TAIL_ABSORB_LINES: usize = 80;
const BATCH_DEFAULT_LINES: usize = 400;
const WHOLE_FILE_MAX_LINES: usize = 300;
const MAX_FILE_BYTES: u64 = 2 * 1024 * 1024;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct FileReadSpec {
    pub path: String,

    #[serde(default)]
    pub offset: Option<i64>,

    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ToolFileReadArgs {
    #[serde(default)]
    pub root: Option<String>,

    #[serde(default)]
    pub path: Option<String>,

    #[serde(default)]
    pub offset: Option<i64>,

    #[serde(default)]
    pub limit: Option<usize>,

    #[serde(default)]
    pub files: Option<Vec<FileReadSpec>>,
}

struct ReadSlice {
    body: String,
    start_line: usize,
    taken: usize,
    total: usize,
}

#[base_app(sort = 764)]
#[derive(Clone, Service)]
pub struct ToolFileRead {
    #[inject(component)]
    tool_manager: ToolManager,
}

#[async_trait]
impl BaseApp for ToolFileRead {
    fn name(&self) -> &'static str {
        "ToolFileRead"
    }

    async fn start(&self, _ctx: &crate::base::base_app::AppContext) -> Result<()> {
        self.tool_manager.register(Arc::new(self.clone()));
        Ok(())
    }
}

impl ToolBase for ToolFileRead {
    fn tool_id(&self) -> AiToolId {
        AiToolId::ToolFileRead
    }

    fn domains(&self) -> &'static [ToolDomain] {
        ToolDomain::READ_ONLY
    }

    fn as_rig_tool(&self, done_push: Option<ToolCallDonePush>) -> DynamicTool {
        ToolBaseOutWrap::wrap(self.clone(), self.max_result_tokens(), done_push)
    }
}

impl ToolFileRead {
    fn resolve_path_under(root: Option<&str>, path: &str) -> Result<PathBuf, AiToolError> {
        let trimmed = path.trim();
        if trimmed.is_empty() {
            return Err(AiToolError::Execution("path is empty".to_string()));
        }
        let raw = PathBuf::from(trimmed);
        if raw.is_absolute() {
            return Ok(raw);
        }
        let Some(root) = root.map(str::trim).filter(|root| !root.is_empty()) else {
            return Err(AiToolError::Execution(format!("path must be absolute, or pass root: {}", trimmed)));
        };
        let base = PathBuf::from(root);
        if !base.is_absolute() {
            return Err(AiToolError::Execution(format!("root must be absolute: {}", root)));
        }
        Ok(base.join(trimmed))
    }

    fn format_lines(lines: &[String], start_line: usize) -> String {
        lines.iter().enumerate().map(|(index, line)| format!("{}→{}", start_line + index, line)).collect::<Vec<_>>().join("\n")
    }

    fn unclosed_ref_prefix(text: &str) -> Option<&str> {
        let mut search_from = 0usize;
        while search_from < text.len() {
            let Some(rel) = text[search_from..].find(REQ_REF_PREFIX) else {
                return None;
            };
            let start = search_from + rel;
            let inner_start = start + REQ_REF_PREFIX.len();
            let Some(end_rel) = text[inner_start..].find(']') else {
                return Some(&text[start..]);
            };
            search_from = inner_start + end_rel + 1;
        }
        None
    }

    fn extend_end_for_ref_token(lines: &[String], start_index: usize, mut end_index: usize) -> Result<usize, AiToolError> {
        let total = lines.len();
        loop {
            let text = lines[start_index..end_index].join("\n");
            let Some(fragment) = Self::unclosed_ref_prefix(&text) else {
                return Ok(end_index);
            };
            if end_index >= total {
                return Err(AiToolError::Execution(format!("unclosed reference missing ]: {}", fragment)));
            }
            end_index += 1;
        }
    }

    async fn read_slice(root: Option<&str>, spec: &FileReadSpec, default_lines: usize) -> Result<ReadSlice, AiToolError> {
        let path = Self::resolve_path_under(root, &spec.path)?;
        if !path.exists() {
            return Err(AiToolError::Execution(format!("file not found: {}", path.display())));
        }
        if !path.is_file() {
            return Err(AiToolError::Execution(format!("not a file: {}", path.display())));
        }
        let meta = std::fs::metadata(&path).map_err(|error| AiToolError::Execution(error.to_string()))?;
        if meta.len() > MAX_FILE_BYTES {
            return Err(AiToolError::Execution(format!("file too large (max {} bytes): {}", MAX_FILE_BYTES, path.display())));
        }
        let content = tokio::fs::read_to_string(&path).await.map_err(|error| AiToolError::Execution(error.to_string()))?;
        let lines: Vec<String> = content.lines().map(|line| line.to_string()).collect();
        let total = lines.len();
        if total == 0 {
            return Ok(ReadSlice { body: String::new(), start_line: 1, taken: 0, total: 0 });
        }
        if total <= WHOLE_FILE_MAX_LINES {
            return Ok(ReadSlice { body: Self::format_lines(&lines, 1), start_line: 1, taken: total, total });
        }
        let start_index = Self::resolve_start(spec.offset, total);
        if start_index >= total {
            return Ok(ReadSlice { body: String::new(), start_line: total + 1, taken: 0, total });
        }
        let mut limit = spec.limit.unwrap_or(default_lines).min(MAX_LINES_READ);
        let remaining = total - start_index;
        if remaining <= limit + TAIL_ABSORB_LINES {
            limit = remaining;
        }
        let end_index = Self::extend_end_for_ref_token(&lines, start_index, (start_index + limit).min(total))?;
        let slice = &lines[start_index..end_index];
        Ok(ReadSlice { body: Self::format_lines(slice, start_index + 1), start_line: start_index + 1, taken: slice.len(), total })
    }

    fn resolve_start(offset: Option<i64>, total: usize) -> usize {
        match offset {
            Some(value) if value < 0 => total.saturating_sub((-value) as usize),
            Some(value) if value <= 1 => 0,
            Some(value) => (value as usize).saturating_sub(1).min(total),
            None => 0,
        }
    }

    async fn read_batch(root: Option<&str>, specs: &[FileReadSpec]) -> Result<Vec<WrapItem>, AiToolError> {
        if specs.is_empty() {
            return Err(AiToolError::Execution("files is empty".to_string()));
        }
        if specs.len() > MAX_BATCH_FILES {
            return Err(AiToolError::Execution(format!("too many files: {} exceeds {}", specs.len(), MAX_BATCH_FILES)));
        }
        let default_lines = if specs.len() == 1 { MAX_LINES_READ } else { BATCH_DEFAULT_LINES };
        let slices = futures::future::join_all(specs.iter().map(|spec| Self::read_slice(root, spec, default_lines))).await;
        let mut items: Vec<WrapItem> = Vec::with_capacity(specs.len());
        for (spec, slice) in specs.iter().zip(slices.into_iter()) {
            items.push(match slice {
                Ok(slice) => Self::wrap_item(&spec.path, &slice),
                Err(error) => WrapItem::wrap_item(Some(format!("===== {} =====", spec.path)), format!("<error> {}", error)),
            });
        }
        Ok(items)
    }

    fn wrap_item(path: &str, slice: &ReadSlice) -> WrapItem {
        if slice.total == 0 {
            return WrapItem::wrap_item(Some(format!("===== {} (empty) =====", path)), String::new());
        }
        if slice.taken == 0 {
            return WrapItem::wrap_item(Some(format!("===== {} (offset is past the end of the file, {} lines total) =====", path, slice.total)), String::new());
        }
        let end = slice.start_line + slice.taken - 1;
        let mut body = slice.body.clone();
        if end < slice.total {
            body.push_str(&format!("\n… 该文件还有 {} 行未显示, 需要时用 offset={} 继续", slice.total - end, end + 1));
        }
        WrapItem::wrap_item(Some(format!("===== {} (lines {}-{} of {}) =====", path, slice.start_line, end, slice.total)), body)
    }
}

impl Tool for ToolFileRead {
    const NAME: &'static str = AiToolId::ToolFileRead.as_str();

    type Args = ToolFileReadArgs;
    type Output = String;
    type Error = AiToolError;

    fn description(&self) -> String {
        "读取本应用所在宿主机上的文本文件。**要看多个文件时一次传 files 数组，不要一个个来回调用**：一次最多 20 个，每项可各自指定 offset/limit，并发读取后一并返回，能把几十轮往返压成一轮。同一目录下的多个文件请配合 root：root 传公共根目录，files 里各项只写相对路径，不用把同一段长前缀重复二十遍（各项也仍可直接写绝对路径）。这样正好接 code_semantic 的输出——它第一行给的就是索引根，下面每个文件是相对该根的路径和命中行号，把根填 root、路径填 path、行号填 offset/limit 即可。单个文件可以只传 path。offset 为从 1 起的行号（负数表示从文件末尾倒数），limit 为最多返回行数（单文件默认 1000 行，批量每项默认 400 行）。**不超过 300 行的文件无视 offset/limit 整份返回**，所以普通源码文件不要传小 limit 去分段翻，也不要对已经整份给过的文件再按 offset 取——那只会被拦回来。大文件才按行号取段。整批输出有统一的 token 预算，超了会按文件均分并保留每个文件的头尾，被裁的文件会列出来。若截断处落在 @[ 引用中间，会继续读到对应的 ] 再结束。返回内容每行带 LINE_NUMBER→ 前缀，该前缀不是文件内容的一部分；编辑时请只匹配 → 之后的原文。".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "root": {
                    "type": "string",
                    "description": "公共根目录绝对路径。给了它，files 各项就只写相对路径；可直接填 code_semantic 输出的索引根"
                },
                "files": {
                    "type": "array",
                    "description": "批量读取，一次最多 20 项。多文件优先用它，省掉逐个调用的往返",
                    "maxItems": MAX_BATCH_FILES,
                    "items": {
                        "type": "object",
                        "properties": {
                            "path": {"type": "string", "description": "文件路径。给了 root 就是相对 root 的路径，否则要绝对路径"},
                            "offset": {"type": "integer", "description": "起始行号，从 1 开始；负数表示从末尾倒数"},
                            "limit": {"type": "integer", "description": "最多返回行数，默认 400；不超过 300 行的文件总是整份返回"}
                        },
                        "required": ["path"]
                    }
                },
                "path": {
                    "type": "string",
                    "description": "单文件绝对路径。与 files 二选一"
                },
                "offset": {
                    "type": "integer",
                    "description": "起始行号，从 1 开始；负数表示从文件末尾倒数。不传则从第 1 行开始"
                },
                "limit": {
                    "type": "integer",
                    "description": "最多返回的行数；不传则最多 1000 行"
                }
            }
        })
    }

    async fn call(&self, context: &mut ToolContext, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let root = args.root.as_deref();
        let specs = match args.files.filter(|files| !files.is_empty()) {
            Some(files) => files,
            None => {
                let Some(path) = args.path.filter(|path| !path.trim().is_empty()) else {
                    return Err(AiToolError::Execution("path is empty (single file), or pass files=[...] for batch".to_string()));
                };
                vec![FileReadSpec { path, offset: args.offset, limit: args.limit }]
            }
        };
        context.insert_result(Self::read_batch(root, &specs).await?);
        Ok(String::new())
    }
}
