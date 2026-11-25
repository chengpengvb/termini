use anyhow::Result;
use std::sync::Arc;
use serde_json::json;
use parking_lot::RwLock;
use std::time::Duration;
use summer::tracing::info;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use std::path::{Path, PathBuf};
use crate::base::base_app::BaseApp;
use serde::{Deserialize, Serialize};
use crate::module::ai::dto::AiToolId;
use crate::module::ai::dto::AiRunContext;
use crate::module::ai::shared::ToolBridge;
use crate::module::ai::shared::ChatFileStore;
use crate::module::ai::shared::ToolCallDonePush;
use rig::tool::{DynamicTool, Tool, ToolContext};
use crate::module::ai::shared::ToolCallChunkPush;
use crate::module::ai::shared::ToolConflictManager;
use crate::module::ai::shared::ChatSessionCacheManager;
use crate::module::ai::tools::tool_manager::ToolManager;
use crate::module::ai::tools::ai_tool_error::AiToolError;
use crate::module::ai::tools::base::{ToolBaseOutWrap, WrapItem};
use crate::module::ai::tools::tool_base::{ToolBase, ToolDomain};
use crate::module::ai::shared::chat_file_store::CHAT_SCRIPTS_DIR;
use crate::module::ai::tools::common::document::tool_installer::ToolInstaller;
use crate::module::ai::tools::common::tool_script_run::bridge_prelude::BridgePrelude;
use crate::module::ai::tools::common::document::script_runtime::{RuntimeProbe, ScriptRuntime};

mod js_runtime;
mod bridge_prelude;
mod runtime_prepare;

const RUN_TIMEOUT: Duration = Duration::from_secs(600);

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ToolScriptRunArgs {
    #[serde(default)]
    pub tools: bool,

    #[serde(default)]
    pub code: String,

    #[serde(default)]
    pub cwd: Option<String>,

    #[serde(default)]
    pub title: Option<String>,

    #[serde(default)]
    pub args: Option<Vec<String>>,

    #[serde(default)]
    pub deps: Option<Vec<String>>,

    #[serde(default)]
    pub script_path: Option<String>,

    #[serde(default)]
    pub runtime: Option<ScriptRuntime>,

    #[serde(default, rename = "__internal_call_id")]
    pub internal_call_id: Option<String>,
}

fn create_probe() -> Arc<RwLock<Option<RuntimeProbe>>> {
    Arc::new(RwLock::new(None))
}

#[base_app(sort = 767)]
#[derive(Clone, Service)]
pub struct ToolScriptRun {
    #[inject(component)]
    tool_bridge: ToolBridge,

    #[inject(component)]
    tool_manager: ToolManager,

    #[inject(component)]
    tool_installer: ToolInstaller,

    #[inject(component)]
    chat_file_store: ChatFileStore,

    #[inject(component)]
    tool_call_done_push: ToolCallDonePush,

    #[inject(component)]
    tool_call_chunk_push: ToolCallChunkPush,

    #[inject(func = create_probe())]
    probe: Arc<RwLock<Option<RuntimeProbe>>>,

    #[inject(component)]
    tool_conflict_manager: ToolConflictManager,

    #[inject(component)]
    chat_session_cache_manager: ChatSessionCacheManager,
}

#[async_trait]
impl BaseApp for ToolScriptRun {
    fn name(&self) -> &'static str {
        "ToolScriptRun"
    }

    async fn start(&self, _ctx: &crate::base::base_app::AppContext) -> Result<()> {
        self.tool_manager.register(Arc::new(self.clone()));
        let probe = self.probe.clone();
        tokio::spawn(async move {
            let detected = RuntimeProbe::detect().await;
            *probe.write() = Some(detected);
        });
        Ok(())
    }
}

impl ToolBase for ToolScriptRun {
    fn tool_id(&self) -> AiToolId {
        AiToolId::ToolScriptRun
    }

    fn deferred(&self) -> bool {
        true
    }

    fn domains(&self) -> &'static [ToolDomain] {
        &[ToolDomain::Main]
    }

    fn persist(&self) -> bool {
        true
    }

    fn as_rig_tool(&self, done_push: Option<ToolCallDonePush>) -> DynamicTool {
        ToolBaseOutWrap::wrap(self.clone(), self.max_result_tokens(), done_push)
    }
}

impl Tool for ToolScriptRun {
    const NAME: &'static str = AiToolId::ToolScriptRun.as_str();

    type Args = ToolScriptRunArgs;
    type Output = String;
    type Error = AiToolError;

    fn description(&self) -> String {
        let probe = self.probe.read().clone().unwrap_or_default();
        let managed_deno = Self::managed_deno_installed();
        let available = probe.available(managed_deno);
        let preferred = probe.preferred(managed_deno);
        let runtime_text = format!("本机可用运行时：{}；不传 runtime 默认用 {}", available.iter().map(|item| item.as_str()).collect::<Vec<&str>>().join(" / "), preferred.as_str());
        format!(
            "在本应用所在宿主机执行一段脚本，用于生成排版复杂的 Word、带公式/样式/多表/图表的 Excel、PPT、任意 PDF，以及数据处理、格式转换等一次性任务。生成文档或表格时必须做出正式版式（字体、标题、表头底色、边距、图表），不要默认空白样式交差。{}。各运行时写法：deno — TypeScript/JavaScript，npm 包直接 `import ... from \"npm:docx\"`（exceljs/pptxgenjs/pdf-lib 同理），自动下载缓存，不需要 deps，参数用 Deno.args；node — ESM JavaScript，`import ... from \"docx\"`，需要的 npm 包名写进 deps 由工具先 npm install 到共享目录（已装过的跳过），参数用 process.argv；python — Python 3，需要的 pip 包名写进 deps（python-docx、openpyxl、python-pptx、reportlab 等）由工具先 pip install 到专用 venv（已装过的跳过），参数用 sys.argv；js — 应用内置的 QuickJS 引擎，不需要任何外部运行时，在 iOS / Android 上只有它可用，可直接写顶层 await，命令行参数在 args 数组里，不支持 deps 安装也不支持 tools=true。生成 Word：用全局 docx 对象构造 Document 后调 await saveDocx(\"文件名.docx\", doc)；生成 PPT：用 new PptxGenJS() 构造后调 await savePptx(\"文件名.pptx\", pptx)。这两个是本环境唯一可用的文档落盘方式，返回写入的绝对路径。本环境没有 Node，Packer.toBuffer、Packer.toBlob、pptx.writeFile 一律不可用（会报 nodebuffer is not supported），不要调用。写其他任意文件用 workspace.write(路径, Uint8Array) 或 workspace.writeBase64(路径, base64)，相对路径相对 cwd 解析。输出文件写绝对路径，写完打印路径；stdout/stderr 实时回显并在结束后连同退出码一起返回。脚本必须能自行结束，不要起常驻服务。执行前会请求用户确认。脚本文件跑完会保留并在结果里返回 script_path；要修正就用 search_replace 改这个文件出错的几行、再把该路径传回 script_path 重跑，不必重发整段 code。tools=true 时脚本里可以直接调用你手头的其他工具来编排多步任务：JS 写 `await tools.ssh_shell_exec_batch({{...}})`，Python 写 `tools.ssh_shell_exec_batch({{...}})`，参数就是该工具的参数对象，返回值是该工具的文本结果；适合「对多台设备循环取数 → 汇总 → 写表」这类原本要来回十几轮的流程，一次脚本跑完。每个嵌套调用照常受审批约束。",
            runtime_text
        )
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "title": {
                    "type": "string",
                    "description": "这段脚本在做什么的短标题，语言遵循 [输出语言]，界面上用它标识这一条"
                },
                "runtime": {
                    "type": "string",
                    "enum": ["deno", "node", "python", "js"],
                    "description": "用哪个运行时执行，只能选工具描述里列为可用的；不传用默认"
                },
                "code": {
                    "type": "string",
                    "description": "完整脚本源码，语言与 runtime 匹配；首次运行传这个，按 script_path 重跑时不要传"
                },
                "script_path": {
                    "type": "string",
                    "description": "已生成脚本的绝对路径（上次 script_run 返回的 script_path）；先用 search_replace 改这个文件出错的几行，再传它按路径重跑，不必重发整段 code。与 code 二选一，runtime 传与该文件一致的值"
                },
                "deps": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "node/python 需要先安装的包名（可带版本，如 exceljs@4、python-docx==1.1.2）；deno 不需要"
                },
                "cwd": {
                    "type": "string",
                    "description": if cfg!(target_os = "ios") {
                        "可选工作目录，脚本里的相对路径相对它解析；不传则使用 Documents/Termini/Workspace。生成的文件会出现在本地 SFTP，可直接上传远端"
                    } else {
                        "可选工作目录，脚本里的相对路径相对它解析；不传则使用应用默认目录"
                    }
                },
                "args": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "可选命令行参数"
                },
                "tools": {
                    "type": "boolean",
                    "description": "为 true 时脚本内可通过 tools.<工具名>(参数对象) 调用其他工具编排流程；默认 false"
                }
            },
            "required": []
        })
    }

    async fn call(&self, context: &mut ToolContext, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let run_context = context.require::<Arc<AiRunContext>>().map_err(|error| AiToolError::Execution(error.to_string()))?.clone();
        let Some(call_id) = args.internal_call_id.clone() else {
            return Err(AiToolError::Execution("internal_call_id of the tool call is missing".to_string()));
        };
        let (source_code, existing_script) = self.resolve_source(&args).await?;
        let session_cancel = run_context.cancel_token.clone();
        let Some(tool_cancel) = self.chat_session_cache_manager.get_tool_cancel(&run_context.req_id, &run_context.run_id, &call_id) else {
            return Err(AiToolError::Execution(format!("tool cancel not found: req_id={} run_id={} call_id={}", run_context.req_id, run_context.run_id, call_id)));
        };

        let probe = self.ensure_probe().await;
        let runtime = args.runtime.unwrap_or_else(|| probe.preferred(Self::managed_deno_installed()));
        let confirm_args = ToolScriptRunArgs {
            tools: args.tools,
            cwd: args.cwd.clone(),
            internal_call_id: None,
            runtime: Some(runtime),
            deps: args.deps.clone(),
            args: args.args.clone(),
            code: source_code.clone(),
            title: args.title.clone(),
            script_path: args.script_path.clone(),
        };
        self.tool_conflict_manager.confirm_before_execute(&run_context, &call_id, self.tool_id().as_str(), &confirm_args).await?;

        let (on_chunk, buffer) = self.tool_call_chunk_push.build_shell_chunk_callback(&run_context, &call_id, self.tool_id().as_str());
        let script_dir = self.chat_file_store.ensure_dir_for_req(&run_context.req_id, CHAT_SCRIPTS_DIR).await.map_err(|error| AiToolError::Execution(error.to_string()))?;
        let existing = existing_script.as_deref();
        if runtime == ScriptRuntime::Js {
            if args.tools {
                return Err(AiToolError::Execution("runtime=js does not support tools=true".to_string()));
            }
            let script = match existing {
                Some(path) => path.to_path_buf(),
                None => Self::write_script(&script_dir, &source_code, ScriptRuntime::Js).await?,
            };
            let tail = js_runtime::execute(source_code.clone(), args.cwd.clone(), args.args.clone().unwrap_or_default(), on_chunk, tool_cancel, session_cancel, RUN_TIMEOUT).await;
            let output = buffer.lock().clone();
            let result = self.compose_result(output, tail, &script);
            self.tool_call_done_push.push_tool_call_done_event(&run_context, &call_id, result.clone());
            context.insert_result(vec![WrapItem::wrap_item(None, result.clone())]);
            return Ok(result);
        }
        let deps = args.deps.clone().unwrap_or_default().into_iter().map(|item| item.trim().to_string()).filter(|item| !item.is_empty()).collect::<Vec<String>>();
        let bridge_token = if args.tools { Some(self.tool_bridge.issue(Arc::clone(&run_context), on_chunk.clone())) } else { None };
        let code = if args.tools { BridgePrelude::wrap(runtime, &source_code) } else { source_code.clone() };
        let prepared = match runtime {
            ScriptRuntime::Deno => self.prepare_deno(&probe, &script_dir, &code, existing).await,
            ScriptRuntime::Node => self.prepare_node(&probe, &code, existing, &deps, &on_chunk).await,
            ScriptRuntime::Python => self.prepare_python(&probe, &script_dir, &code, existing, &deps, &on_chunk).await,
            ScriptRuntime::Js => Err(AiToolError::Execution("runtime=js runs in-process and has no prepared command".to_string())),
        };
        let (mut command, script) = match prepared {
            Ok(prepared) => prepared,
            Err(error) => {
                self.revoke_bridge(bridge_token.as_deref());
                return Err(error);
            }
        };
        if let Some(token) = bridge_token.as_deref() {
            let endpoint = match self.tool_bridge.endpoint() {
                Ok(endpoint) => endpoint,
                Err(error) => {
                    self.revoke_bridge(Some(token));
                    return Err(AiToolError::Execution(error.to_string()));
                }
            };
            command.env("TERMINI_TOOL_BRIDGE_URL", endpoint).env("TERMINI_TOOL_BRIDGE_TOKEN", token);
        }
        if let Some(extra) = args.args.as_ref() {
            command.args(extra);
        }
        if let Some(cwd) = args.cwd.as_deref().map(str::trim).filter(|value| !value.is_empty()) {
            command.current_dir(cwd);
        }

        info!("[ai] 启动脚本 req_id={} call_id={} runtime={} tools={} script={}", run_context.req_id, call_id, runtime.as_str(), args.tools, script.display());
        let spawned = command.spawn().map_err(|error| AiToolError::Execution(format!("failed to start {}: {}", runtime.as_str(), error)));
        let mut child = match spawned {
            Ok(child) => child,
            Err(error) => {
                self.revoke_bridge(bridge_token.as_deref());
                return Err(error);
            }
        };
        let stdout = child.stdout.take().map(|pipe| tokio::spawn(Self::pump(pipe, on_chunk.clone())));
        let stderr = child.stderr.take().map(|pipe| tokio::spawn(Self::pump(pipe, on_chunk.clone())));

        let tail = tokio::select! {
            biased;
            _ = tool_cancel.cancelled() => {
                let _ = child.kill().await;
                "exit_code=cancelled".to_string()
            }
            _ = session_cancel.cancelled() => {
                let _ = child.kill().await;
                "exit_code=cancelled".to_string()
            }
            status = tokio::time::timeout(RUN_TIMEOUT, child.wait()) => match status {
                Ok(Ok(status)) => format!("exit_code={} runtime={}", status.code().map(|code| code.to_string()).unwrap_or_else(|| "signal".to_string()), runtime.as_str()),
                Ok(Err(error)) => format!("exit_code=error\nfailed while waiting for the script to exit: {}", error),
                Err(_) => {
                    let _ = child.kill().await;
                    format!("exit_code=killed\nthe script did not finish within {}s and was terminated", RUN_TIMEOUT.as_secs())
                }
            }
        };
        for pump in [stdout, stderr].into_iter().flatten() {
            let _ = pump.await;
        }
        self.revoke_bridge(bridge_token.as_deref());

        let output = buffer.lock().clone();
        let result = self.compose_result(output, tail, &script);
        self.tool_call_done_push.push_tool_call_done_event(&run_context, &call_id, result.clone());
        context.insert_result(vec![WrapItem::wrap_item(None, result.clone())]);
        Ok(result)
    }
}

impl ToolScriptRun {
    async fn resolve_source(&self, args: &ToolScriptRunArgs) -> Result<(String, Option<PathBuf>), AiToolError> {
        let path = args.script_path.as_deref().map(str::trim).filter(|value| !value.is_empty());
        let has_code = !args.code.trim().is_empty();
        match (path, has_code) {
            (Some(_), true) => Err(AiToolError::Execution("pass either code or script_path, not both".to_string())),
            (Some(path), false) => {
                let script = PathBuf::from(path);
                let content = tokio::fs::read_to_string(&script).await.map_err(|error| AiToolError::Execution(format!("failed to read script {}: {}", script.display(), error)))?;
                Ok((content, Some(script)))
            }
            (None, true) => Ok((args.code.clone(), None)),
            (None, false) => Err(AiToolError::Execution("code or script_path is required".to_string())),
        }
    }

    fn compose_result(&self, output: String, tail: String, script: &Path) -> String {
        let body = if output.is_empty() { tail } else { format!("{}\n{}", output, tail) };
        format!("{}\nscript_path={}", body, script.display())
    }

    fn revoke_bridge(&self, token: Option<&str>) {
        if let Some(token) = token {
            self.tool_bridge.revoke(token);
        }
    }
}
