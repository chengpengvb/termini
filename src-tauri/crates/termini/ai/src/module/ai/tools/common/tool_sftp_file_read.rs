use std::sync::Arc;
use serde_json::json;
use rig::prelude::Tool;
use summer::plugin::Service;
use tokio::io::AsyncReadExt;
use async_trait::async_trait;
use termini_macros::base_app;
use crate::base::base_app::BaseApp;
use serde::{Deserialize, Serialize};
use crate::module::ai::dto::AiToolId;
use crate::module::ai::dto::AiRunContext;
use rig::tool::{DynamicTool, ToolContext};
use crate::module::ai::shared::ToolCallDonePush;
use crate::module::ai::tools::tool_base::ToolBase;
use crate::module::ai::shared::AgentRemoteClientPool;
use crate::module::common::dto::file_entry::FileKind;
use crate::module::ai::tools::tool_manager::ToolManager;
use crate::module::ai::tools::ai_tool_error::AiToolError;
use crate::module::ai::tools::base::{ToolBaseOutWrap, WrapItem};

const MAX_BATCH_FILES: usize = 8;
const MAX_LINES_READ: usize = 1000;
const MAX_FILE_BYTES: u64 = 2 * 1024 * 1024;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct SftpFileReadSpec {
    pub path: String,

    #[serde(default)]
    pub offset: Option<i64>,

    #[serde(default)]
    pub limit: Option<usize>,

    #[serde(default)]
    pub device_id: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ToolSftpFileReadArgs {
    #[serde(default)]
    pub offset: Option<i64>,

    #[serde(default)]
    pub limit: Option<usize>,

    #[serde(default)]
    pub path: Option<String>,

    #[serde(default)]
    pub device_id: Option<String>,

    #[serde(default)]
    pub files: Option<Vec<SftpFileReadSpec>>,

    #[serde(default, rename = "__internal_call_id")]
    pub internal_call_id: Option<String>,
}

struct SftpReadTarget {
    label: String,
    device_id: String,
    spec: SftpFileReadSpec,
}

#[base_app(sort = 768)]
#[derive(Clone, Service)]
pub struct ToolSftpFileRead {
    #[inject(component)]
    tool_manager: ToolManager,

    #[inject(component)]
    agent_remote_client_pool: AgentRemoteClientPool,
}

#[async_trait]
impl BaseApp for ToolSftpFileRead {
    fn name(&self) -> &'static str {
        "ToolSftpFileRead"
    }

    async fn start(&self, _ctx: &crate::base::base_app::AppContext) -> anyhow::Result<()> {
        self.tool_manager.register(Arc::new(self.clone()));
        Ok(())
    }
}

impl ToolBase for ToolSftpFileRead {
    fn tool_id(&self) -> AiToolId {
        AiToolId::ToolSftpFileRead
    }

    fn on_stop(&self, req_id: &str) {
        self.agent_remote_client_pool.release(req_id);
    }

    fn as_rig_tool(&self, done_push: Option<ToolCallDonePush>) -> DynamicTool {
        ToolBaseOutWrap::wrap(self.clone(), self.max_result_tokens(), done_push)
    }
}

impl ToolSftpFileRead {
    fn resolve_remote_path(path: &str) -> Result<String, AiToolError> {
        let trimmed = path.trim();
        if trimmed.is_empty() {
            return Err(AiToolError::Execution("path is empty".to_string()));
        }
        if !trimmed.starts_with('/') {
            return Err(AiToolError::Execution(format!("path must be absolute: {}", path)));
        }
        Ok(trimmed.to_string())
    }

    fn format_lines(lines: &[String], start_line: usize) -> String {
        lines.iter().enumerate().map(|(index, line)| format!("{}→{}", start_line + index, line)).collect::<Vec<_>>().join("\n")
    }

    fn slice_content(content: &str, offset: Option<i64>, limit: Option<usize>) -> Result<String, AiToolError> {
        let lines: Vec<String> = content.lines().map(|line| line.to_string()).collect();
        let total = lines.len();
        if total == 0 {
            return Ok(String::new());
        }
        let start_index = match offset {
            Some(value) if value < 0 => {
                let from_end = (-value) as usize;
                total.saturating_sub(from_end)
            }
            Some(0) => 0,
            Some(value) => {
                let start = value as usize;
                if start == 0 {
                    0
                } else {
                    start.saturating_sub(1).min(total)
                }
            }
            None => 0,
        };
        let read_limit = limit.unwrap_or(MAX_LINES_READ).min(MAX_LINES_READ);
        let end_index = (start_index + read_limit).min(total);
        if start_index >= total {
            return Ok(String::new());
        }
        let slice = &lines[start_index..end_index];
        let mut body = Self::format_lines(slice, start_index + 1);
        if end_index < total {
            body.push_str(&format!("\n… truncated, showing lines {}-{} of {}", start_index + 1, end_index, total));
        }
        Ok(body)
    }

    fn resolve_targets(args: &ToolSftpFileReadArgs) -> Result<Vec<SftpReadTarget>, AiToolError> {
        let top_device = args.device_id.as_deref().map(str::trim).filter(|value| !value.is_empty());
        if let Some(files) = args.files.as_ref().filter(|files| !files.is_empty()) {
            if files.len() > MAX_BATCH_FILES {
                return Err(AiToolError::Execution(format!("too many files: {} exceeds {}", files.len(), MAX_BATCH_FILES)));
            }
            return files
                .iter()
                .map(|spec| {
                    let Some(device_id) = spec.device_id.as_deref().map(str::trim).filter(|value| !value.is_empty()).or(top_device) else {
                        return Err(AiToolError::Execution(format!("device_id is missing for {} (set it on the item or at top level)", spec.path)));
                    };
                    Ok(SftpReadTarget { label: format!("{}:{}", device_id, spec.path.trim()), device_id: device_id.to_string(), spec: spec.clone() })
                })
                .collect();
        }
        let Some(device_id) = top_device else {
            return Err(AiToolError::Execution("device_id is empty".to_string()));
        };
        let Some(path) = args.path.clone().filter(|path| !path.trim().is_empty()) else {
            return Err(AiToolError::Execution("path is empty (single file), or pass files=[...] for batch".to_string()));
        };
        let label = format!("{}:{}", device_id, path.trim());
        Ok(vec![SftpReadTarget { label, device_id: device_id.to_string(), spec: SftpFileReadSpec { device_id: None, path, offset: args.offset, limit: args.limit } }])
    }

    async fn read_one(&self, run_context: &Arc<AiRunContext>, target: &SftpReadTarget) -> Result<String, AiToolError> {
        let path = Self::resolve_remote_path(&target.spec.path)?;
        let client = self.agent_remote_client_pool.get_or_open_sftp(&run_context.req_id, &target.device_id).await?;
        let entry = client.stat(&path, &FileKind::File).await.map_err(|error| AiToolError::Execution(error.to_string()))?;
        if entry.kind != FileKind::File {
            return Err(AiToolError::Execution(format!("not a file: {}", path)));
        }
        let Some(size) = entry.size else {
            return Err(AiToolError::Execution(format!("file size unavailable: {}", path)));
        };
        if size > MAX_FILE_BYTES {
            return Err(AiToolError::Execution(format!("file too large (max {} bytes): {}", MAX_FILE_BYTES, path)));
        }
        let mut reader = client.open_read(&path).await.map_err(|error| AiToolError::Execution(error.to_string()))?;
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await.map_err(|error| AiToolError::Execution(error.to_string()))?;
        let content = String::from_utf8(bytes).map_err(|error| AiToolError::Execution(format!("file is not valid UTF-8: {} ({})", path, error)))?;
        Self::slice_content(&content, target.spec.offset, target.spec.limit)
    }
}

impl Tool for ToolSftpFileRead {
    const NAME: &'static str = AiToolId::ToolSftpFileRead.as_str();

    type Args = ToolSftpFileReadArgs;
    type Output = String;
    type Error = AiToolError;

    fn description(&self) -> String {
        "经 SFTP 读取已登记设备上的文本文件。须传 device_id（不能是本机）与远端绝对路径 path。要读多个文件时一次传 files 数组（最多 8 个，每项可各自指定 device_id/offset/limit，缺 device_id 则用顶层的；顺序读取，按 ===== 设备:路径 ===== 分段返回，单个失败不影响其余），不要一个个来回调用。可指定 offset（从 1 起的行号，负数表示从文件末尾倒数）与 limit（最多返回行数，默认最多 1000 行）。返回内容每行带 LINE_NUMBER→ 前缀，该前缀不是文件内容的一部分；编辑时请只匹配 → 之后的原文，并用 sftp_search_replace。文件超过 2MB 会失败。".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "device_id": {
                    "type": "string",
                    "description": "已登记设备 id（不能是本机）；批量时作为各项缺省的 device_id"
                },
                "path": {
                    "type": "string",
                    "description": "单个远端文件绝对路径。与 files 二选一"
                },
                "offset": {
                    "type": "integer",
                    "description": "起始行号，从 1 开始；负数表示从文件末尾倒数。不传则从第 1 行开始"
                },
                "limit": {
                    "type": "integer",
                    "description": "最多返回的行数；不传则最多 1000 行"
                },
                "files": {
                    "type": "array",
                    "description": "批量读取时传数组，最多 8 个；与单个参数二选一",
                    "maxItems": MAX_BATCH_FILES,
                    "items": {
                        "type": "object",
                        "properties": {
                            "device_id": { "type": "string", "description": "该项所在设备 id；不传则用顶层 device_id" },
                            "path": { "type": "string", "description": "远端文件绝对路径" },
                            "offset": { "type": "integer", "description": "起始行号，从 1 开始；负数表示从末尾倒数" },
                            "limit": { "type": "integer", "description": "最多返回行数，默认 1000" }
                        },
                        "required": ["path"]
                    }
                }
            }
        })
    }

    async fn call(&self, context: &mut ToolContext, args: Self::Args) -> anyhow::Result<Self::Output, Self::Error> {
        let run_context = context.require::<Arc<AiRunContext>>().map_err(|error| AiToolError::Execution(error.to_string()))?.clone();
        let targets = Self::resolve_targets(&args)?;
        let mut items: Vec<WrapItem> = Vec::with_capacity(targets.len());
        for target in &targets {
            if run_context.cancel_token.is_cancelled() {
                return Err(AiToolError::Cancelled("sftp_file_read batch cancelled".to_string()));
            }
            items.push(match self.read_one(&run_context, target).await {
                Ok(body) => WrapItem::wrap_item(Some(format!("===== {} =====", target.label)), body),
                Err(AiToolError::Cancelled(reason)) => return Err(AiToolError::Cancelled(reason)),
                Err(error) => WrapItem::wrap_item(Some(format!("===== {} =====", target.label)), format!("<error> {}", error)),
            });
        }
        context.insert_result(items);
        Ok(String::new())
    }
}
