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
use crate::module::ai::tools::tool_base::ToolBase;
use crate::module::ai::shared::ToolConflictManager;
use crate::module::ai::tools::common::edit_replace;
use crate::util::agents_md_util::discover_agents_md;
use crate::module::ai::shared::ChatSessionCacheManager;
use crate::module::ai::tools::tool_manager::ToolManager;
use crate::module::ai::tools::ai_tool_error::AiToolError;
use crate::service::service_ai_file_change::AiFileChangeService;
use crate::module::ai::tools::base::{PathGuard, ToolBaseOutWrap, WrapItem};

const MAX_FILE_BYTES: u64 = 2 * 1024 * 1024;

const MAX_BATCH_EDITS: usize = 20;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct FileEditSpec {
    #[serde(default)]
    pub path: String,
    pub old_string: String,
    pub new_string: String,

    #[serde(default)]
    pub replace_all: bool,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_line: Option<usize>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ToolSearchReplaceArgs {
    #[serde(default)]
    pub replace_all: bool,

    #[serde(default)]
    pub root: Option<String>,

    #[serde(default)]
    pub path: Option<String>,

    #[serde(default)]
    pub old_string: Option<String>,

    #[serde(default)]
    pub new_string: Option<String>,

    #[serde(default)]
    pub edits: Option<Vec<FileEditSpec>>,

    #[serde(default, rename = "__internal_call_id")]
    pub internal_call_id: Option<String>,
}

struct FilePlan {
    label: String,
    path: PathBuf,
    before: Option<String>,
    after: String,
    applied: usize,
    created: bool,
}

#[base_app(sort = 765)]
#[derive(Clone, Service)]
pub struct ToolSearchReplace {
    #[inject(component)]
    tool_manager: ToolManager,

    #[inject(component)]
    tool_conflict_manager: ToolConflictManager,

    #[inject(component)]
    ai_file_change_service: AiFileChangeService,

    #[inject(component)]
    chat_session_cache_manager: ChatSessionCacheManager,
}

#[async_trait]
impl BaseApp for ToolSearchReplace {
    fn name(&self) -> &'static str {
        "ToolSearchReplace"
    }

    async fn start(&self, _ctx: &crate::base::base_app::AppContext) -> Result<()> {
        self.tool_manager.register(Arc::new(self.clone()));
        Ok(())
    }
}

impl ToolBase for ToolSearchReplace {
    fn tool_id(&self) -> AiToolId {
        AiToolId::ToolSearchReplace
    }

    fn as_rig_tool(&self, done_push: Option<ToolCallDonePush>) -> DynamicTool {
        ToolBaseOutWrap::wrap(self.clone(), self.max_result_tokens(), done_push)
    }
}

impl ToolSearchReplace {
    fn resolve_path(root: Option<&str>, path: &str) -> Result<PathBuf, AiToolError> {
        let trimmed = path.trim();
        if trimmed.is_empty() {
            return Err(AiToolError::Execution("path is empty".to_string()));
        }
        let raw = PathBuf::from(trimmed);
        if raw.is_absolute() {
            return Ok(raw);
        }
        let Some(root) = root.map(str::trim).filter(|item| !item.is_empty()) else {
            return Err(AiToolError::Execution(format!("path must be absolute, ~ is not expanded: {}", trimmed)));
        };
        let base = PathBuf::from(root);
        if !base.is_absolute() {
            return Err(AiToolError::Execution(format!("root must be absolute: {}", root)));
        }
        Ok(base.join(trimmed))
    }

    fn resolve_edits(args: &ToolSearchReplaceArgs) -> Result<Vec<FileEditSpec>, AiToolError> {
        if let Some(edits) = args.edits.as_ref().filter(|edits| !edits.is_empty()) {
            if edits.len() > MAX_BATCH_EDITS {
                return Err(AiToolError::Execution(format!("too many edits: {} exceeds {}", edits.len(), MAX_BATCH_EDITS)));
            }
            let shared = args.path.as_deref().map(str::trim).filter(|item| !item.is_empty());
            let mut resolved = edits.clone();
            for (index, edit) in resolved.iter_mut().enumerate() {
                if !edit.path.trim().is_empty() {
                    continue;
                }
                let Some(shared) = shared else {
                    return Err(AiToolError::Execution(format!("search_replace: edits[{}] 缺 path。每项写自己的 path, 或者整批都改同一个文件时在顶层传一次 path", index)));
                };
                edit.path = shared.to_string();
            }
            return Ok(resolved);
        }
        let Some(path) = args.path.clone().filter(|item| !item.trim().is_empty()) else {
            return Err(AiToolError::Execution("search_replace: 单处改动要传 path, 多处改动传 edits=[...]".to_string()));
        };
        let Some(new_string) = args.new_string.clone() else {
            return Err(AiToolError::Execution("search_replace: 缺 new_string".to_string()));
        };
        Ok(vec![FileEditSpec { path, old_string: args.old_string.clone().unwrap_or_default(), new_string, replace_all: args.replace_all, start_line: None }])
    }

    async fn plan_edits(root: Option<&str>, edits: &mut [FileEditSpec]) -> Result<Vec<FilePlan>, AiToolError> {
        let mut plans: Vec<FilePlan> = Vec::new();
        for edit in edits.iter_mut() {
            let path = Self::resolve_path(root, &edit.path)?;
            PathGuard::ensure_writable(&path)?;
            if edit.old_string.replace("\r\n", "\n") == edit.new_string.replace("\r\n", "\n") {
                return Err(AiToolError::Execution(format!("new_string must differ from old_string: {}", path.display())));
            }
            let label = path.display().to_string();
            let position = plans.iter().position(|plan| plan.path == path);
            // 整文件覆盖/新建: 直接丢弃该文件之前算出的内容
            if edit.old_string.is_empty() {
                let before = match position {
                    Some(index) => plans[index].before.clone(),
                    None => Self::read_existing(&path).await?,
                };
                let created = before.is_none();
                let plan = FilePlan { label, path: path.clone(), before, after: edit.new_string.clone(), applied: 1, created };
                match position {
                    Some(index) => plans[index] = plan,
                    None => plans.push(plan),
                }
                continue;
            }
            let index = match position {
                Some(index) => index,
                None => {
                    let Some(before) = Self::read_existing(&path).await? else {
                        return Err(AiToolError::Execution(format!("file not found: {}", path.display())));
                    };
                    plans.push(FilePlan { label: label.clone(), path: path.clone(), before: Some(before.clone()), after: before, applied: 0, created: false });
                    plans.len() - 1
                }
            };
            // 回填改动在当前文件内容里的起始行号, 界面据此按真实行号显示 diff
            edit.start_line = edit_replace::first_match_line(&plans[index].after, &edit.old_string);
            let request = edit_replace::ReplaceRequest { old_string: &edit.old_string, new_string: &edit.new_string, replace_all: edit.replace_all };
            let (next, applied) = edit_replace::apply_one(&plans[index].after, &plans[index].label, &request).map_err(AiToolError::Execution)?;
            plans[index].after = next;
            plans[index].applied += applied;
        }
        Ok(plans)
    }

    async fn read_existing(path: &PathBuf) -> Result<Option<String>, AiToolError> {
        let meta = match tokio::fs::metadata(path).await {
            Ok(meta) => meta,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(AiToolError::Execution(format!("{}: {}", path.display(), error))),
        };
        if !meta.is_file() {
            return Err(AiToolError::Execution(format!("not a file: {}", path.display())));
        }
        if meta.len() > MAX_FILE_BYTES {
            return Err(AiToolError::Execution(format!("file too large (max {} bytes): {}", MAX_FILE_BYTES, path.display())));
        }
        match tokio::fs::read_to_string(path).await {
            Ok(text) => Ok(Some(text)),
            Err(error) if error.kind() == std::io::ErrorKind::InvalidData => Err(AiToolError::Execution(format!("file is not valid UTF-8, only UTF-8 is supported: {}", path.display()))),
            Err(error) => Err(AiToolError::Execution(format!("{}: {}", path.display(), error))),
        }
    }

    async fn write_atomic(path: &PathBuf, content: &str, permissions: std::fs::Permissions) -> Result<(), AiToolError> {
        let Some(parent) = path.parent() else {
            return Err(AiToolError::Execution(format!("path has no parent directory: {}", path.display())));
        };
        let Some(name) = path.file_name() else {
            return Err(AiToolError::Execution(format!("path has no file name: {}", path.display())));
        };
        let temp = parent.join(format!(".{}.{}.termix-tmp", name.to_string_lossy(), std::process::id()));
        tokio::fs::write(&temp, content).await.map_err(|error| AiToolError::Execution(format!("{}: {}", temp.display(), error)))?;
        tokio::fs::set_permissions(&temp, permissions).await.map_err(|error| AiToolError::Execution(format!("{}: {}", temp.display(), error)))?;
        if let Err(error) = tokio::fs::rename(&temp, path).await {
            let _ = tokio::fs::remove_file(&temp).await;
            return Err(AiToolError::Execution(format!("{} -> {}: {}", temp.display(), path.display(), error)));
        }
        Ok(())
    }

    async fn commit(plan: &FilePlan) -> Result<String, AiToolError> {
        if plan.created {
            if let Some(parent) = plan.path.parent() {
                if !parent.as_os_str().is_empty() {
                    tokio::fs::create_dir_all(parent).await.map_err(|error| AiToolError::Execution(error.to_string()))?;
                }
            }
            tokio::fs::write(&plan.path, &plan.after).await.map_err(|error| AiToolError::Execution(error.to_string()))?;
            return Ok(format!("created file {}", plan.label));
        }
        let permissions = match tokio::fs::metadata(&plan.path).await {
            Ok(meta) => meta.permissions(),
            Err(error) => return Err(AiToolError::Execution(format!("{}: {}", plan.label, error))),
        };
        Self::write_atomic(&plan.path, &plan.after, permissions).await?;
        if plan.applied == 0 {
            return Ok(format!("overwrote {}", plan.label));
        }
        Ok(format!("replaced {} occurrence(s) in {}", plan.applied, plan.label))
    }
}

impl Tool for ToolSearchReplace {
    const NAME: &'static str = AiToolId::ToolSearchReplace.as_str();

    type Args = ToolSearchReplaceArgs;
    type Output = String;
    type Error = AiToolError;

    fn description(&self) -> String {
        "在本应用所在宿主机上精确替换文件中的字符串。**改动多处时一次传 edits 数组，不要一处一次来回调用**：一次最多 20 处，可跨多个文件，同一文件的多处按给定顺序应用、只写一次盘，整批只请求一次用户确认。改动都在内存里算完并全部校验通过才落盘，任何一处 old_string 失配则整批不写入，不会留下改了一半的代码。同目录下的文件可配合 root：root 传公共根目录，edits 里各项只写相对路径。整批都改同一个文件时，顶层传一次 path 即可，edits 里不用每项重复写。编辑前应先用 file_read 读取原文（可用它的 files 批量读）。old_string 必须与文件中内容完全一致（不要包含 file_read 返回的 LINE_NUMBER→ 前缀）。默认 old_string 只能匹配一处；多处匹配时请扩大上下文使其唯一，或将 replace_all 设为 true。old_string 为空时表示新建或整文件覆盖写入。".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "root": {
                    "type": "string",
                    "description": "公共根目录绝对路径。给了它，edits 各项就只写相对路径"
                },
                "edits": {
                    "type": "array",
                    "description": "批量改动，一次最多 20 处，可跨文件。改多处优先用它，省掉逐处调用的往返与逐条确认",
                    "maxItems": MAX_BATCH_EDITS,
                    "items": {
                        "type": "object",
                        "properties": {
                            "path": {"type": "string", "description": "文件路径。给了 root 就是相对 root 的路径，否则要绝对路径；整批都改同一个文件时可以省略，改在顶层传一次 path"},
                            "old_string": {"type": "string", "description": "要被替换的原文；空字符串表示新建文件或整文件覆盖"},
                            "new_string": {"type": "string", "description": "替换后的新文本；必须与 old_string 不同"},
                            "replace_all": {"type": "boolean", "description": "是否替换全部匹配；默认 false，此时 old_string 必须唯一"}
                        },
                        "required": ["old_string", "new_string"]
                    }
                },
                "path": {
                    "type": "string",
                    "description": "单处改动的文件路径；也可以和 edits 一起传，作为 edits 里没写 path 的那些项的共同文件"
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
        let root = args.root.as_deref();
        let plans = Self::plan_edits(root, &mut edits).await?;
        let confirm_args = ToolSearchReplaceArgs { root: args.root.clone(), edits: Some(edits), internal_call_id: None, ..Default::default() };
        self.tool_conflict_manager.confirm_before_execute(&run_context, &call_id, self.tool_id().as_str(), &confirm_args).await?;

        let mut items: Vec<WrapItem> = Vec::with_capacity(plans.len());
        for plan in &plans {
            if let Some((doc_path, content)) = discover_agents_md(&plan.path).await {
                self.chat_session_cache_manager.add_agents_doc(&run_context.req_id, &doc_path, content);
            }
            let body = {
                let _guard = self.tool_manager.lock_path(&ToolManager::local_path_key(&plan.path)).await;
                Self::commit(plan).await?
            };
            self.ai_file_change_service.record(&run_context.req_id, None, plan.label.clone(), plan.before.clone(), plan.after.clone()).await;
            items.push(WrapItem::wrap_item(Some(format!("===== {} =====", plan.label)), body));
        }
        context.insert_result(items);
        Ok(String::new())
    }
}
