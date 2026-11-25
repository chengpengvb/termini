use std::pin::Pin;
use std::sync::Arc;
use serde_json::json;
use parking_lot::Mutex;
use std::future::Future;
use std::time::Duration;
use summer::tracing::warn;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use crate::model::device::Device;
use crate::base::base_app::BaseApp;
use serde::{Deserialize, Serialize};
use crate::module::ai::dto::AiToolId;
use crate::model::dto::shared_ptr::Shared;
use crate::module::ai::shared::ToolCallDonePush;
use rig::tool::{DynamicTool, Tool, ToolContext};
use crate::service::service_config::ConfigService;
use crate::module::shell::client_shell::ClientShell;
use crate::module::ai::tools::tool_manager::ToolManager;
use crate::module::ai::tools::ai_tool_error::AiToolError;
use crate::module::common::context::context_base::WindowSize;
use crate::module::ai::tools::base::{ToolBaseOutWrap, WrapItem};
use crate::module::ai::tools::tool_base::{ToolBase, ToolDomain};
use crate::module::ai::tools::common::task::rule::shell_readonly;
use crate::module::common::dto::shell_chunk_callback::ShellChunkCallback;
use crate::module::shell::platform::shell_discover::native_shell_platform;
use crate::module::common::context::context_shell::{ConfigShell, ContextShell};

const READONLY_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ToolShellReadonlyArgs {
    pub command: String,

    #[serde(default)]
    pub title: Option<String>,

    #[serde(default)]
    pub cwd: Option<String>,
}

#[base_app(sort = 762)]
#[derive(Clone, Service)]
pub struct ToolShellReadonly {
    #[inject(component)]
    tool_manager: ToolManager,

    #[inject(component)]
    config_service: ConfigService,
}

#[async_trait]
impl BaseApp for ToolShellReadonly {
    fn name(&self) -> &'static str {
        "ToolShellReadonly"
    }

    async fn start(&self, _ctx: &crate::base::base_app::AppContext) -> anyhow::Result<()> {
        self.tool_manager.register(Arc::new(self.clone()));
        Ok(())
    }
}

impl ToolBase for ToolShellReadonly {
    fn tool_id(&self) -> AiToolId {
        AiToolId::ToolShellReadonly
    }

    fn domains(&self) -> &'static [ToolDomain] {
        &[ToolDomain::Audit]
    }

    fn persist(&self) -> bool {
        true
    }

    fn as_rig_tool(&self, done_push: Option<ToolCallDonePush>) -> DynamicTool {
        ToolBaseOutWrap::wrap(self.clone(), self.max_result_tokens(), done_push)
    }
}

impl Tool for ToolShellReadonly {
    const NAME: &'static str = AiToolId::ToolShellReadonly.as_str();

    type Args = ToolShellReadonlyArgs;
    type Output = String;
    type Error = AiToolError;

    fn description(&self) -> String {
        "在本机执行一条只读命令并返回输出, 不需要审批。用来做代码工具不方便做的事: 列目录清单和文件大小(ls / dir / Get-ChildItem -Recurse -Filter)、统计规模(wc -l、Measure-Object)、看版本历史(git log / blame / show / diff)、看依赖树(mvn dependency:tree、gradle dependencies、cargo tree)、看文件类型和哈希。只放行白名单里的只读命令: rg/grep/find/ls/dir/tree/cat/head/tail/wc/sort/uniq/cut/diff/Get-ChildItem/Get-Content/Select-String/Measure-Object/Select-Object 等, 以及 git 的 log|show|diff|blame|grep|status|ls-files 类只读子命令和构建工具的依赖查看子命令。任何写文件(> >> Out-File Set-Content)、删改(rm mv cp Remove-Item)、安装、联网(curl wget)、提权(sudo)、切目录(cd)、嵌套 shell(bash -c / iex) 都会被拒绝并说明原因, 需要这些操作请写进报告让主代理去做。命令必须非交互, 60 秒内结束, 超时会被终止。语法按上下文里「操作系统运行情况」标明的终端类型写。找代码内容优先用 code_search / code_semantic, 它们更快且结果带行号; 这个工具是补充, 不是替代。".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "title": {
                    "type": "string",
                    "description": "这条命令在做什么的短标题, 语言遵循 [输出语言]"
                },
                "cwd": {
                    "type": "string",
                    "description": "工作目录, 必须是绝对路径且在本次调查范围内; 不传则用进程默认目录"
                },
                "command": {
                    "type": "string",
                    "description": "要执行的只读命令; 语法必须匹配操作系统运行情况中标明的终端类型; 非交互, 不要 cd, 用 cwd 指定目录"
                }
            },
            "required": ["command"]
        })
    }

    async fn call(&self, context: &mut ToolContext, args: Self::Args) -> anyhow::Result<Self::Output, Self::Error> {
        if let Err(reason) = shell_readonly::check_readonly(&args.command) {
            warn!("[ai] 拒绝非只读命令 reason={} command={}", reason, args.command);
            let body =
                format!("not executed: {}. This is a read-only shell that only runs whitelisted read-only commands; if changes are needed, describe them in the report for the main agent.", reason);
            context.insert_result(vec![WrapItem::wrap_item(None, body.clone())]);
            return Ok(body);
        }
        let Some(config) = self.config_service.select_config().await else {
            return Err(AiToolError::Execution("default config is not found".to_string()));
        };
        let cpu_percent = config.resolve_shell_cpu_percent();
        let Some(local_shell_id) = config.local_shell_id else {
            return Err(AiToolError::Execution("local_shell_id is not found".to_string()));
        };
        let Some(shell_descr) = native_shell_platform().find_by_shell_id(&local_shell_id) else {
            return Err(AiToolError::Execution(format!("shell_id={} not found", local_shell_id)));
        };
        let config_shell = ConfigShell { cwd: args.cwd.clone(), shell_descr, env: Vec::new(), cpu_percent };
        let shell_context = ContextShell { size: WindowSize::default(), config: Shared::new(config_shell), device: Shared::new(Device::new_local()) };

        let buffer: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
        let sink = buffer.clone();
        let on_chunk: ShellChunkCallback = Arc::new(move |bytes: Vec<u8>| {
            let sink = sink.clone();
            Box::pin(async move {
                sink.lock().extend_from_slice(&bytes);
            }) as Pin<Box<dyn Future<Output = ()> + Send>>
        });
        let mut process = ClientShell::new().spawn(&shell_context, args.command.clone(), on_chunk).await.map_err(|error| AiToolError::Execution(error.to_string()))?;

        let exit = match tokio::time::timeout(READONLY_TIMEOUT, process.wait_exit()).await {
            Ok(exit) => exit.map_err(|error| AiToolError::Execution(error.to_string()))?,
            Err(_) => {
                process.kill();
                let _ = process.wait_exit().await;
                let output = String::from_utf8_lossy(&buffer.lock()).to_string();
                let body = format!("{}\nexit_code=killed\nthe command did not finish within {} seconds and was terminated. A read-only investigation should not take this long; use a more precise command or switch to code_search.", output, READONLY_TIMEOUT.as_secs());
                context.insert_result(vec![WrapItem::wrap_item(None, body.clone())]);
                return Ok(body);
            }
        };
        let output = String::from_utf8_lossy(&buffer.lock()).to_string();
        let tail = match exit {
            crate::module::shell::client_shell::ShellExit::Code(code) => format!("exit_code={}", code),
            crate::module::shell::client_shell::ShellExit::Signal => "exit_code=signal".to_string(),
        };
        let body = if output.trim().is_empty() { format!("(no output)\n{}", tail) } else { format!("{}\n{}", output, tail) };
        context.insert_result(vec![WrapItem::wrap_item(None, body.clone())]);
        Ok(body)
    }
}
