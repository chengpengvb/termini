use std::sync::Arc;
use serde_json::json;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use crate::base::base_app::BaseApp;
use serde::{Deserialize, Serialize};
use crate::module::ai::dto::AiToolId;
use crate::module::ai::shared::WakeBus;
use crate::module::ai::dto::AiRunContext;
use crate::module::ai::shared::ShellDaemonPush;
use crate::module::ai::flow::FlowShellSupervise;
use crate::module::ai::shared::ToolCallDonePush;
use rig::tool::{DynamicTool, Tool, ToolContext};
use crate::module::ai::shared::ToolCallChunkPush;
use crate::module::ai::shared::ToolSupervisePush;
use crate::module::ai::tools::tool_base::ToolBase;
use crate::service::service_config::ConfigService;
use crate::module::ai::shared::ToolConflictManager;
use crate::module::ai::shared::ChatSessionCacheManager;
use crate::module::ai::tools::tool_manager::ToolManager;
use crate::module::ai::tools::ai_tool_error::AiToolError;
use crate::module::ai::tools::base::{ToolBaseOutWrap, WrapItem};
use crate::module::shell::platform::shell_discover::native_shell_platform;
use crate::module::ai::tools::common::shell::local_shell_tty::LocalShellTty;
use crate::module::ai::tools::common::shell::tool_local_shell::execute::StepContext;
use crate::module::ai::tools::common::shell::local_shell_registry::LocalShellRegistry;

mod finish;
mod execute;

pub const MAX_BATCH_COMMANDS: usize = 20;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ShellCommandSpec {
    pub command: String,

    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub resident: bool,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wake_pattern: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ToolLocalShellArgs {
    #[serde(default)]
    pub tty: bool,

    #[serde(default)]
    pub resident: bool,

    #[serde(default)]
    pub command: Option<String>,

    #[serde(default)]
    pub cwd: Option<String>,

    #[serde(default)]
    pub title: Option<String>,

    #[serde(default)]
    pub wake_pattern: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commands: Option<Vec<ShellCommandSpec>>,

    #[serde(default, rename = "__internal_call_id")]
    pub internal_call_id: Option<String>,
}

#[base_app(sort = 762)]
#[derive(Clone, Service)]
pub struct ToolLocalShell {
    #[inject(component)]
    wake_bus: WakeBus,

    #[inject(component)]
    tool_manager: ToolManager,

    #[inject(component)]
    config_service: ConfigService,

    #[inject(component)]
    local_shell_tty: LocalShellTty,

    #[inject(component)]
    shell_daemon_push: ShellDaemonPush,

    #[inject(component)]
    tool_call_done_push: ToolCallDonePush,

    #[inject(component)]
    tool_supervise_push: ToolSupervisePush,

    #[inject(component)]
    tool_call_chunk_push: ToolCallChunkPush,

    #[inject(component)]
    local_shell_registry: LocalShellRegistry,

    #[inject(component)]
    flow_shell_supervise: FlowShellSupervise,

    #[inject(component)]
    tool_conflict_manager: ToolConflictManager,

    #[inject(component)]
    chat_session_cache_manager: ChatSessionCacheManager,
}

#[async_trait]
impl BaseApp for ToolLocalShell {
    fn name(&self) -> &'static str {
        "ToolLocalShell"
    }

    async fn start(&self, _ctx: &crate::base::base_app::AppContext) -> anyhow::Result<()> {
        self.tool_manager.register(Arc::new(self.clone()));
        Ok(())
    }
}

impl ToolBase for ToolLocalShell {
    fn tool_id(&self) -> AiToolId {
        AiToolId::ToolLocalShell
    }

    fn on_stop(&self, req_id: &str) {
        let Some(entry) = self.chat_session_cache_manager.get(req_id) else {
            return;
        };
        let chat_id = entry.context.ai_chat_id.clone();
        drop(entry);
        self.local_shell_registry.release_transient(&chat_id);
    }

    fn persist(&self) -> bool {
        true
    }

    fn as_rig_tool(&self, done_push: Option<ToolCallDonePush>) -> DynamicTool {
        ToolBaseOutWrap::wrap(self.clone(), self.max_result_tokens(), done_push)
    }
}

impl Tool for ToolLocalShell {
    const NAME: &'static str = AiToolId::ToolLocalShell.as_str();

    type Args = ToolLocalShellArgs;
    type Output = String;
    type Error = AiToolError;

    fn description(&self) -> String {
        if cfg!(target_os = "ios") {
            return "在本机执行命令。本机是 iOS 设备，命令由应用内置的 ios_system 命令环境在应用进程内执行，可用的是内置 BSD 命令实现：pwd、cd、ls、cat、mkdir、cp、mv、rm、grep、sed 等。以下能力在 iOS 沙盒下一律不存在，不要尝试也不要生成对应命令：不能执行任何下载来的二进制，不能安装软件包（没有 apt/brew/npm/pip），没有 sudo / root / 任何提权渠道，不能启动常驻后台服务，没有真实 PTY 所以 vim、top、全屏 TUI、各类 REPL 都无法运行。命令必须是能自行结束的非交互命令。可写范围只有应用的 Documents 目录，默认工作目录是 Documents/Termini/Workspace，cwd 可指定其他目录但不能越出 Documents。Deck 对话生成的脚本和产出在 Workspace/Deck，本地 SFTP 能直接看到并上传到远端，写文件优先落到这个目录。命令结束时返回全部输出和退出码。连续多条一次传 commands 数组（最多 20 条，按顺序串行执行，整批只确认一次，某条非零退出即停止后续）。tty=true 会打开一个命令型终端标签页（逐条执行、有回显和提示符），但它不是真实 TTY，不要用它跑需要全屏控制或持续交互的程序，也不能和 commands 同时用。需要生成 Word、PPT、Excel 时不要用本工具，改用 script_run 并指定 runtime=js。".to_string();
        }
        format!("在本应用所在宿主机执行命令，支持多行脚本。该宿主机不是任何已登记设备，操作已登记设备一律改用远程执行工具。cwd 指定工作目录，不传用默认。终端类型以上下文注入的「操作系统运行情况」为准，命令语法须严格匹配，禁止凭经验假设。**连续多条命令一次传 commands 数组，不要一条一次来回调用**：一次最多 {} 条，按顺序串行执行（上一条结束才跑下一条），整批只请求一次确认，各条的回显按 ===== 序号/总数 ===== 分段返回。某条非零退出、被监督终止或被取消时立即停止，剩余命令不执行且会列出来；已执行的不回滚。每条都是独立进程，cd / export / 激活虚拟环境不会带到下一条，要共享环境就写进同一条的多行脚本里。commands 里每项可单独给 cwd、title、wake_pattern，不给就用顶层的。默认非交互：命令执行期间不能等待终端输入。确实需要交互（会问密码、y/N、安装向导、REPL、需要 TTY 才肯运行的程序）时传 tty=true：命令跑在真实终端里，界面会自动打开对应的终端标签页，本工具返回的是终端当前屏幕的文字快照和 process_id；之后用 {} 按 process_id 写入字符（要回车带 \\n）并拿到写入后的屏幕快照。tty=true 只能单条，不支持 commands。屏幕快照只反映终端此刻显示的内容，判断命令有没有执行完要看提示符是否回来。返回里若标注「用户在终端里输入了…」，说明用户已亲自在这个终端里敲了东西（例如密码），不要再替用户重复输入。需要管理员/root 权限时优先用系统原生授权弹窗提权：Windows 用 Start-Process -Verb RunAs -Wait 提权运行，弹 UAC；macOS 用 osascript -e 'do shell script \"要执行的命令\" with administrator privileges' 弹密码框；Linux 桌面用 pkexec；没有图形授权渠道时才用 tty=true 跑 sudo 并让用户在终端里输入密码。提权命令内嵌的真实命令照常受审核。提权后必须再执行一条只读命令回读结果确认是否真的生效，不要凭提权命令没报错就断定成功。这台机器是用户正在用的电脑，不是跑批的服务器：不要同时发起多路重负载（并发编译、全盘扫描、大批量压缩转码、死循环压测），也不要自己加 -j 把并发开到核数以上，重活一次只跑一条并让它跑完；确实要扫全盘或跑长任务时先缩小目录范围、加上限制条件。耗时任务（编译、打包、拉依赖）和不会自行退出的常驻进程（服务、dev server、watch、tail -f、数据库）都可直接跑，不要自己加 & / nohup / start：非 tty 模式下由监督模型盯实时回显决定如何处置。命令自行结束时返回全部输出和退出码。跑得久的命令有四种去向，由你的声明决定。不声明：能等完就一直等；监督判定还要跑很久时转异步——返回 process_id、进程继续跑（批量里算这条已完成，直接继续下一条），异步进程不进界面任务卡、结束不会叫醒你，只活到本轮结束、收尾时被杀掉，所以要用它结果的步骤必须在本轮做完：先推进不依赖它的事，需要结果时用 {} 传 wait_seconds 挂起等它结束再读输出（等价于 sleep）。要让进程跨轮次活着（服务、dev server、watch、tail -f、数据库）传 resident=true：监督判定它已起来后挂到界面任务卡、用户可停，结束不会叫醒你，看输出用 {}。既要挂载又要被叫醒才传 wake_pattern（正则）：启动后立即挂载、不经监督，输出整行命中正则或进程结束时框架以一条 [后台唤醒] 新消息叫醒你并附上输出，不要轮询；监控脚本自己循环检测，命中条件时打印一行独特的标志文本，正则就匹配那一行；要等唤醒才能做的步骤不要塞进同一批 commands，没有别的事就收尾并说明在等什么。挂载的进程用 {} 停止，用户也可在界面手动停止，停止后不再唤醒，应用退出时统一关闭。非 tty 模式下被判定为卡死或在等输入时会被终止并返回原因。",
            MAX_BATCH_COMMANDS,
            AiToolId::ToolShellProcessWrite.as_str(),
            AiToolId::ToolShellProcessLog.as_str(),
            AiToolId::ToolShellProcessLog.as_str(),
            AiToolId::ToolShellProcessKill.as_str())
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "title": {
                    "type": "string",
                    "description": "这条命令在做什么的短标题，语言遵循 [输出语言]。界面上用它标识这一条，长命令不至于只能看到被截断的开头"
                },
                "cwd": {
                    "type": "string",
                    "description": "可选工作目录；不传则使用进程默认目录。传了 commands 时作为各项的缺省工作目录"
                },
                "commands": {
                    "type": "array",
                    "description": "顺序执行的多条命令，最多 20 条。能预先排好的连续命令优先用它，不要一条一次来回调用；与 command、tty 互斥",
                    "maxItems": MAX_BATCH_COMMANDS,
                    "items": {
                        "type": "object",
                        "properties": {
                            "command": { "type": "string", "description": "这一条要执行的命令或脚本；必须非交互" },
                            "cwd": { "type": "string", "description": "这一条的工作目录；不传用顶层 cwd" },
                            "title": { "type": "string", "description": "这一条在做什么的短标题，语言遵循 [输出语言]" },
                            "resident": { "type": "boolean", "description": "这一条要跨轮次挂载到任务卡时给 true；不给用顶层的" },
                            "wake_pattern": { "type": "string", "description": "这一条要挂载并当唤醒源时的正则；挂上后本条立即转后台并继续下一条" }
                        },
                        "required": ["command"]
                    }
                },
                "command": {
                    "type": "string",
                    "description": "单条要执行的命令或脚本，与 commands 二选一；语法必须严格匹配操作系统运行情况中终端字段标明的真实终端类型，不得套用其他终端的语法习惯；tty 为 false 时必须是非交互式命令；耗时任务和常驻服务直接写原始命令，不要自己改成后台运行的写法"
                },
                "tty": {
                    "type": "boolean",
                    "description": "是否在真实终端里运行以便交互；默认 false，且不能与 commands 同用。为 true 时界面会自动打开一个终端标签页，你看到的是终端屏幕快照而不是流式回显，只在命令确实需要等待输入（REPL、密码、y/N、安装向导）时用"
                },
                "resident": {
                    "type": "boolean",
                    "description": "要让进程跨轮次活着时给 true：监督判定它已起来后挂到界面任务卡（用户可停），结束不叫醒你。不给时跑得久的命令转异步、本轮结束即被杀。传了 wake_pattern 不用再传"
                },
                "wake_pattern": {
                    "type": "string",
                    "description": "可选正则。传了就表示这条命令要挂载到任务卡并当唤醒源：启动后立即挂载、不经监督，输出中任一整行命中该正则、或进程结束时，都会以新消息叫醒你。只给真正表示结束或事件发生的标志行，不要给必然出现的词；tty=true 时不支持"
                }
            }
        })
    }

    async fn call(&self, context: &mut ToolContext, args: Self::Args) -> anyhow::Result<Self::Output, Self::Error> {
        let run_context = context.require::<Arc<AiRunContext>>().map_err(|error| AiToolError::Execution(error.to_string()))?.clone();
        let Some(call_id) = args.internal_call_id.clone() else {
            return Err(AiToolError::Execution("internal_call_id of the tool call is missing".to_string()));
        };
        let session_cancel = run_context.cancel_token.clone();
        let Some(tool_cancel) = self.chat_session_cache_manager.get_tool_cancel(&run_context.req_id, &run_context.run_id, &call_id) else {
            return Err(AiToolError::Execution(format!("tool cancel not found: req_id={} run_id={} call_id={}", run_context.req_id, run_context.run_id, call_id)));
        };

        let Some(config) = self.config_service.select_config().await else {
            return Err(AiToolError::Execution("default config is not found".to_string()));
        };
        let Some(local_shell_id) = config.local_shell_id.clone() else {
            return Err(AiToolError::Execution("local_shell_id is not found".to_string()));
        };
        let Some(shell_descr) = native_shell_platform().find_by_shell_id(&local_shell_id) else {
            return Err(AiToolError::Execution(format!("shell_id={} not found", local_shell_id)));
        };
        let Some(session_entry) = self.chat_session_cache_manager.get(&run_context.req_id) else {
            return Err(AiToolError::Execution(format!("unknown req_id: {}", run_context.req_id)));
        };
        let chat_context = session_entry.context.clone();
        drop(session_entry);

        let specs = Self::resolve_commands(&args)?;
        self.tool_conflict_manager.confirm_before_execute(&run_context, &call_id, self.tool_id().as_str(), &Self::confirm_args(&args, &specs)).await?;

        let step = StepContext { call_id, shell_descr, chat_context, shell_id: local_shell_id, run: run_context, tool_cancel, session_cancel, cpu_percent: config.resolve_shell_cpu_percent() };
        if args.tty {
            let spec = specs.first().expect("resolve_commands yields at least one command");
            return Ok(Self::wrap_out(context, self.run_tty(&step, spec).await?));
        }
        self.run_batch(context, &step, &specs).await
    }
}

impl ToolLocalShell {
    fn wrap_out(context: &mut ToolContext, body: String) -> String {
        context.insert_result(vec![WrapItem::wrap_item(None, body.clone())]);
        body
    }

    fn resolve_commands(args: &ToolLocalShellArgs) -> Result<Vec<ShellCommandSpec>, AiToolError> {
        let Some(commands) = args.commands.as_ref().filter(|items| !items.is_empty()) else {
            let Some(command) = args.command.clone() else {
                return Err(AiToolError::Execution("command is empty, or pass commands=[...] to run several commands in order".to_string()));
            };
            let single = ShellCommandSpec { command, cwd: args.cwd.clone(), title: args.title.clone(), wake_pattern: args.wake_pattern.clone(), resident: args.resident };
            return Self::normalize_command(&single, args).map(|spec| vec![spec]);
        };
        if args.tty {
            return Err(AiToolError::Execution("tty=true does not support commands=[...]; run an interactive command on its own".to_string()));
        }
        if commands.len() > MAX_BATCH_COMMANDS {
            return Err(AiToolError::Execution(format!("too many commands: {} exceeds {}", commands.len(), MAX_BATCH_COMMANDS)));
        }
        commands.iter().map(|item| Self::normalize_command(item, args)).collect()
    }

    fn normalize_command(item: &ShellCommandSpec, args: &ToolLocalShellArgs) -> Result<ShellCommandSpec, AiToolError> {
        if item.command.trim().is_empty() {
            return Err(AiToolError::Execution("command is empty".to_string()));
        }
        if item.wake_pattern.is_some() && args.tty {
            return Err(AiToolError::Execution("wake_pattern is not supported with tty=true".to_string()));
        }
        Ok(ShellCommandSpec {
            command: item.command.clone(),
            cwd: item.cwd.clone().or_else(|| args.cwd.clone()),
            title: item.title.clone(),
            wake_pattern: item.wake_pattern.clone(),
            resident: item.resident || args.resident,
        })
    }

    fn confirm_args(args: &ToolLocalShellArgs, specs: &[ShellCommandSpec]) -> ToolLocalShellArgs {
        if args.commands.is_none() {
            return args.clone();
        }
        ToolLocalShellArgs { cwd: args.cwd.clone(), title: args.title.clone(), commands: Some(specs.to_vec()), ..Default::default() }
    }

    async fn run_tty(&self, step: &StepContext, spec: &ShellCommandSpec) -> Result<String, AiToolError> {
        let entry = self.local_shell_tty.spawn(step.chat_id(), step.call_id.clone(), step.shell_id.clone(), spec.command.clone(), step.shell_context(spec)).await?;
        let Some(record) = self.local_shell_tty.record(&entry.process_id) else {
            return Err(AiToolError::Execution(format!("terminal session vanished: process_id={}", entry.process_id)));
        };
        let cancel = [step.tool_cancel.clone(), step.session_cancel.clone()];
        let attached = self.local_shell_tty.wait_attached(&record, &cancel).await;
        let snapshot = if attached { self.local_shell_tty.capture(&record, LocalShellTty::settle_wait(), &cancel).await } else { None };
        let mut output = LocalShellTty::render(&record, &entry, snapshot.as_ref(), 0);
        output.push_str(&format!(
            "\n该命令运行在交互终端里，界面上已自动打开对应标签页：process_id={}。后续用 {} 传 process_id 与要输入的原文 chars 继续交互（默认自动回车，不要在 chars 里写 \\n；只传 process_id 是刷新屏幕），用 {} 结束它。",
            entry.process_id,
            AiToolId::ToolShellProcessWrite.as_str(),
            AiToolId::ToolShellProcessKill.as_str()
        ));
        self.tool_call_done_push.push_tool_call_done_event(&step.run, &step.call_id, output.clone());
        Ok(output)
    }
}
