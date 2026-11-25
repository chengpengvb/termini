use std::sync::Arc;
use serde_json::json;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use crate::base::base_app::BaseApp;
use serde::{Deserialize, Serialize};
use crate::module::ai::dto::AiToolId;
use crate::module::ai::dto::AiRunContext;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use crate::module::ai::shared::ToolCallDonePush;
use rig::tool::{DynamicTool, Tool, ToolContext};
use crate::module::ai::tools::tool_base::ToolBase;
use crate::module::ai::shared::ToolConflictManager;
use crate::module::ai::tools::common::edit_replace;
use crate::module::ai::shared::AgentRemoteClientPool;
use crate::module::common::dto::file_entry::FileKind;
use crate::module::ai::tools::tool_manager::ToolManager;
use crate::module::ai::tools::ai_tool_error::AiToolError;
use crate::module::ai::tools::base::{ToolBaseOutWrap, WrapItem};
use crate::service::service_ai_file_change::AiFileChangeService;
use crate::web::ws::handle::r#impl::sftp::ClientTransferSftpDyn;
use crate::model::resp::resp_sftp::RespSftpTransferProgressHandler;

const MAX_FILE_BYTES: u64 = 2 * 1024 * 1024;
const MAX_BATCH_EDITS: usize = 20;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct SftpEditSpec {
    pub path: String,
    pub old_string: String,
    pub new_string: String,

    #[serde(default)]
    pub replace_all: bool,

    #[serde(default)]
    pub device_id: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_line: Option<usize>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ToolSftpSearchReplaceArgs {
    #[serde(default)]
    pub replace_all: bool,

    #[serde(default)]
    pub path: Option<String>,

    #[serde(default)]
    pub device_id: Option<String>,

    #[serde(default)]
    pub old_string: Option<String>,

    #[serde(default)]
    pub new_string: Option<String>,

    #[serde(default)]
    pub edits: Option<Vec<SftpEditSpec>>,

    #[serde(default, rename = "__internal_call_id")]
    pub internal_call_id: Option<String>,
}

struct FilePlan {
    path: String,
    created: bool,
    label: String,
    after: String,
    applied: usize,
    device_id: String,
    before: Option<String>,
}

#[base_app(sort = 769)]
#[derive(Clone, Service)]
pub struct ToolSftpSearchReplace {
    #[inject(component)]
    tool_manager: ToolManager,

    #[inject(component)]
    tool_conflict_manager: ToolConflictManager,

    #[inject(component)]
    agent_remote_client_pool: AgentRemoteClientPool,

    #[inject(component)]
    ai_file_change_service: AiFileChangeService,
}

#[async_trait]
impl BaseApp for ToolSftpSearchReplace {
    fn name(&self) -> &'static str {
        "ToolSftpSearchReplace"
    }

    async fn start(&self, _ctx: &crate::base::base_app::AppContext) -> anyhow::Result<()> {
        self.tool_manager.register(Arc::new(self.clone()));
        Ok(())
    }
}

impl ToolBase for ToolSftpSearchReplace {
    fn tool_id(&self) -> AiToolId {
        AiToolId::ToolSftpSearchReplace
    }

    fn on_stop(&self, req_id: &str) {
        self.agent_remote_client_pool.release(req_id);
    }

    fn as_rig_tool(&self, done_push: Option<ToolCallDonePush>) -> DynamicTool {
        ToolBaseOutWrap::wrap(self.clone(), self.max_result_tokens(), done_push)
    }
}

impl ToolSftpSearchReplace {
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

    fn resolve_device_id(top: Option<&str>, item: Option<&str>, path: &str) -> Result<String, AiToolError> {
        let item = item.map(str::trim).filter(|value| !value.is_empty());
        let top = top.map(str::trim).filter(|value| !value.is_empty());
        item.or(top).map(str::to_string).ok_or_else(|| AiToolError::Execution(format!("device_id is missing for {} (set it on the item or at top level)", path)))
    }

    fn noop_progress_handler() -> RespSftpTransferProgressHandler {
        Arc::new(|_| Box::pin(async { Ok(()) }))
    }

    async fn read_text(client: &ClientTransferSftpDyn, path: &str) -> Result<String, AiToolError> {
        let entry = client.stat(path, &FileKind::File).await.map_err(|error| AiToolError::Execution(error.to_string()))?;
        if entry.kind != FileKind::File {
            return Err(AiToolError::Execution(format!("not a file: {}", path)));
        }
        let Some(size) = entry.size else {
            return Err(AiToolError::Execution(format!("file size unavailable: {}", path)));
        };
        if size > MAX_FILE_BYTES {
            return Err(AiToolError::Execution(format!("file too large (max {} bytes): {}", MAX_FILE_BYTES, path)));
        }
        let mut reader = client.open_read(path).await.map_err(|error| AiToolError::Execution(error.to_string()))?;
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await.map_err(|error| AiToolError::Execution(error.to_string()))?;
        String::from_utf8(bytes).map_err(|error| AiToolError::Execution(format!("file is not valid UTF-8: {} ({})", path, error)))
    }

    pub(crate) async fn write_text(client: &ClientTransferSftpDyn, path: &str, content: &str) -> Result<(), AiToolError> {
        let bytes = content.as_bytes();
        if bytes.len() as u64 > MAX_FILE_BYTES {
            return Err(AiToolError::Execution(format!("content too large (max {} bytes): {}", MAX_FILE_BYTES, path)));
        }
        let mut writer = client.open_write(path, None, true, Some(bytes.len() as u64), Self::noop_progress_handler()).await.map_err(|error| AiToolError::Execution(error.to_string()))?;
        writer.write_all(bytes).await.map_err(|error| AiToolError::Execution(error.to_string()))?;
        writer.shutdown().await.map_err(|error| AiToolError::Execution(error.to_string()))?;
        Ok(())
    }

    async fn read_existing(client: &ClientTransferSftpDyn, path: &str) -> Result<Option<String>, AiToolError> {
        match client.stat(path, &FileKind::File).await {
            Ok(_) => Ok(Some(Self::read_text(client, path).await?)),
            Err(_) => Ok(None),
        }
    }

    fn resolve_edits(args: &ToolSftpSearchReplaceArgs) -> Result<Vec<SftpEditSpec>, AiToolError> {
        let top_device = args.device_id.as_deref();
        if let Some(edits) = args.edits.as_ref().filter(|edits| !edits.is_empty()) {
            if edits.len() > MAX_BATCH_EDITS {
                return Err(AiToolError::Execution(format!("too many edits: {} exceeds {}", edits.len(), MAX_BATCH_EDITS)));
            }
            return edits
                .iter()
                .map(|edit| {
                    let path = Self::resolve_remote_path(&edit.path)?;
                    let device_id = Self::resolve_device_id(top_device, edit.device_id.as_deref(), &path)?;
                    Ok(SftpEditSpec { path, device_id: Some(device_id), old_string: edit.old_string.clone(), new_string: edit.new_string.clone(), replace_all: edit.replace_all, start_line: None })
                })
                .collect();
        }
        let device_id = Self::resolve_device_id(top_device, None, args.path.as_deref().unwrap_or(""))?;
        let Some(path) = args.path.as_deref().map(str::trim).filter(|item| !item.is_empty()) else {
            return Err(AiToolError::Execution("path is empty, or pass edits=[...] for batch".to_string()));
        };
        let path = Self::resolve_remote_path(path)?;
        let Some(new_string) = args.new_string.clone() else {
            return Err(AiToolError::Execution("new_string is required".to_string()));
        };
        Ok(vec![SftpEditSpec { path, device_id: Some(device_id), old_string: args.old_string.clone().unwrap_or_default(), new_string, replace_all: args.replace_all, start_line: None }])
    }

    async fn plan_edits(&self, req_id: &str, edits: &mut [SftpEditSpec]) -> Result<Vec<FilePlan>, AiToolError> {
        let mut plans: Vec<FilePlan> = Vec::new();
        for edit in edits.iter_mut() {
            let device_id = edit.device_id.clone().ok_or_else(|| AiToolError::Execution(format!("device_id is missing for {}", edit.path)))?;
            if edit.old_string.replace("\r\n", "\n") == edit.new_string.replace("\r\n", "\n") {
                return Err(AiToolError::Execution(format!("new_string must differ from old_string: {}", edit.path)));
            }
            let client = self.agent_remote_client_pool.get_or_open_sftp(req_id, &device_id).await?;
            let label = format!("{}:{}", device_id, edit.path);
            let position = plans.iter().position(|plan| plan.device_id == device_id && plan.path == edit.path);
            if edit.old_string.is_empty() {
                let before = match position {
                    Some(index) => plans[index].before.clone(),
                    None => Self::read_existing(client.as_ref(), &edit.path).await?,
                };
                let created = before.is_none();
                let plan = FilePlan { label, path: edit.path.clone(), device_id, before, after: edit.new_string.clone(), applied: 1, created };
                match position {
                    Some(index) => plans[index] = plan,
                    None => plans.push(plan),
                }
                continue;
            }
            let index = match position {
                Some(index) => index,
                None => {
                    let before = Self::read_text(client.as_ref(), &edit.path).await?;
                    plans.push(FilePlan { label: label.clone(), path: edit.path.clone(), device_id, before: Some(before.clone()), after: before, applied: 0, created: false });
                    plans.len() - 1
                }
            };
            edit.start_line = edit_replace::first_match_line(&plans[index].after, &edit.old_string);
            let request = edit_replace::ReplaceRequest { old_string: &edit.old_string, new_string: &edit.new_string, replace_all: edit.replace_all };
            let (next, applied) = edit_replace::apply_one(&plans[index].after, &plans[index].label, &request).map_err(AiToolError::Execution)?;
            plans[index].after = next;
            plans[index].applied += applied;
        }
        Ok(plans)
    }

    fn commit_message(plan: &FilePlan) -> String {
        if plan.created {
            return format!("created file {}", plan.path);
        }
        if plan.applied == 0 {
            return format!("overwrote {}", plan.path);
        }
        format!("replaced {} occurrence(s) in {}", plan.applied, plan.path)
    }
}

impl Tool for ToolSftpSearchReplace {
    const NAME: &'static str = AiToolId::ToolSftpSearchReplace.as_str();

    type Error = AiToolError;
    type Args = ToolSftpSearchReplaceArgs;
    type Output = String;

    fn description(&self) -> String {
        "经 SFTP 在已登记设备上精确替换文本文件中的字符串。**改动多处时一次传 edits 数组，不要一处一次来回调用**：一次最多 20 处，可跨多个文件、可跨设备，同一文件的多处按给定顺序应用、只写一次，整批只请求一次用户确认。改动都在内存里算完并全部校验通过才写入，任何一处 old_string 失配则整批不写入，不会留下改了一半的代码。须传 device_id（不能是本机）；批量时各项可各自指定 device_id，缺则用顶层的。path 必须是远端绝对路径。编辑前应先用 sftp_file_read 读取原文（可用它的 files 批量读）。old_string 必须与文件中内容完全一致（不要包含 sftp_file_read 返回的 LINE_NUMBER→ 前缀）。默认 old_string 只能匹配一处；多处匹配时请扩大上下文使其唯一，或将 replace_all 设为 true。old_string 为空时表示新建或整文件覆盖写入。".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "device_id": {
                    "type": "string",
                    "description": "已登记设备 id（不能是本机）；批量时作为各项缺省的 device_id"
                },
                "edits": {
                    "type": "array",
                    "description": "批量改动，一次最多 20 处，可跨文件、可跨设备。改多处优先用它，省掉逐处调用的往返与逐条确认",
                    "maxItems": MAX_BATCH_EDITS,
                    "items": {
                        "type": "object",
                        "properties": {
                            "device_id": {"type": "string", "description": "该项所在设备 id；不传则用顶层 device_id"},
                            "path": {"type": "string", "description": "远端文件绝对路径"},
                            "old_string": {"type": "string", "description": "要被替换的原文；空字符串表示新建文件或整文件覆盖"},
                            "new_string": {"type": "string", "description": "替换后的新文本；必须与 old_string 不同"},
                            "replace_all": {"type": "boolean", "description": "是否替换全部匹配；默认 false，此时 old_string 必须唯一"}
                        },
                        "required": ["path", "old_string", "new_string"]
                    }
                },
                "path": {
                    "type": "string",
                    "description": "单处改动的远端文件绝对路径。与 edits 二选一"
                },
                "old_string": {
                    "type": "string",
                    "description": "要被替换的原文；空字符串表示新建文件或整文件覆盖"
                },
                "new_string": {
                    "type": "string",
                    "description": "替换后的新文本；必须与 old_string 不同"
                },
                "replace_all": {
                    "type": "boolean",
                    "description": "是否替换全部匹配；默认 false，此时 old_string 必须唯一"
                }
            }
        })
    }

    async fn call(&self, context: &mut ToolContext, args: Self::Args) -> anyhow::Result<Self::Output, Self::Error> {
        let run_context = context.require::<Arc<AiRunContext>>().map_err(|error| AiToolError::Execution(error.to_string()))?.clone();
        let Some(call_id) = args.internal_call_id.clone() else {
            return Err(AiToolError::Execution("internal_call_id of the tool call is missing".to_string()));
        };
        let mut edits = Self::resolve_edits(&args)?;
        let plans = self.plan_edits(&run_context.req_id, &mut edits).await?;
        let confirm_args = ToolSftpSearchReplaceArgs { device_id: args.device_id.clone(), edits: Some(edits), internal_call_id: None, ..Default::default() };
        self.tool_conflict_manager.confirm_before_execute(&run_context, &call_id, self.tool_id().as_str(), &confirm_args).await?;
        let mut items: Vec<WrapItem> = Vec::with_capacity(plans.len());
        for plan in &plans {
            let body = {
                let _guard = self.tool_manager.lock_path(&format!("{}:{}", plan.device_id, plan.path)).await;
                let client = self.agent_remote_client_pool.get_or_open_sftp(&run_context.req_id, &plan.device_id).await?;
                Self::write_text(client.as_ref(), &plan.path, &plan.after).await?;
                Self::commit_message(plan)
            };
            self.ai_file_change_service.record(&run_context.req_id, Some(plan.device_id.clone()), plan.path.clone(), plan.before.clone(), plan.after.clone()).await;
            items.push(WrapItem::wrap_item(Some(format!("===== {} =====", plan.label)), body));
        }
        context.insert_result(items);
        Ok(String::new())
    }
}
