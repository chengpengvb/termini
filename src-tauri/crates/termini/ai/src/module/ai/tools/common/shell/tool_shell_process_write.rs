use anyhow::Result;
use std::sync::Arc;
use serde_json::json;
use tokio::time::Duration;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use crate::base::base_app::BaseApp;
use serde::{Deserialize, Serialize};
use crate::module::ai::dto::AiToolId;
use crate::util::time_util::now_millis;
use crate::module::ai::dto::AiRunContext;
use crate::module::ai::shared::ToolCallDonePush;
use rig::tool::{DynamicTool, Tool, ToolContext};
use crate::module::ai::tools::tool_base::ToolBase;
use crate::module::ai::shared::ToolConflictManager;
use crate::module::ai::shared::ChatSessionCacheManager;
use crate::module::ai::tools::tool_manager::ToolManager;
use crate::module::ai::tools::ai_tool_error::AiToolError;
use crate::module::ai::tools::base::{ToolBaseOutWrap, WrapItem};
use crate::module::ai::tools::common::shell::local_shell_tty::LocalShellTty;
use crate::module::ai::tools::common::shell::local_shell_registry::LocalShellRegistry;
use crate::module::shell::shell_session_hub::{ctrl_key, enter_key, prepare_text_for_terminal};

const WAIT_MAX_MS: u64 = 30000;
const WAIT_DEFAULT_MS: u64 = 1500;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ToolShellProcessWriteArgs {
    #[serde(default)]
    pub chars: String,

    pub process_id: String,

    #[serde(default)]
    pub enter: Option<bool>,

    #[serde(default)]
    pub ctrl: Option<String>,

    #[serde(default)]
    pub wait_ms: Option<u64>,

    #[serde(default, rename = "__internal_call_id")]
    pub internal_call_id: Option<String>,
}

impl ToolShellProcessWriteArgs {
    fn terminal_input(&self) -> Result<Option<String>, AiToolError> {
        if let Some(ctrl) = self.ctrl.as_deref() {
            let mut letters = ctrl.trim().chars();
            let key = match (letters.next(), letters.next()) {
                (Some(letter), None) => ctrl_key(letter),
                _ => None,
            };
            return key.map(Some).ok_or_else(|| AiToolError::Execution(format!("ctrl must be a single letter a-z, got `{}`", ctrl)));
        }
        let press_enter = self.enter.unwrap_or(!self.chars.is_empty());
        if self.chars.is_empty() && !press_enter {
            return Ok(None);
        }
        let mut input = prepare_text_for_terminal(&self.chars);
        if press_enter {
            input.push_str(&enter_key());
        }
        Ok(Some(input))
    }

    fn describe(&self) -> String {
        if let Some(ctrl) = self.ctrl.as_deref() {
            return format!("Ctrl+{}", ctrl.trim().to_ascii_uppercase());
        }
        if self.chars.is_empty() {
            return "Enter".to_string();
        }
        self.chars.clone()
    }
}

#[derive(Serialize)]
struct ConfirmPayload<'a> {
    process_id: &'a str,
    command: &'a str,
}

#[base_app(sort = 762)]
#[derive(Clone, Service)]
pub struct ToolShellProcessWrite {
    #[inject(component)]
    tool_manager: ToolManager,

    #[inject(component)]
    local_shell_tty: LocalShellTty,

    #[inject(component)]
    tool_conflict_manager: ToolConflictManager,

    #[inject(component)]
    local_shell_registry: LocalShellRegistry,

    #[inject(component)]
    chat_session_cache_manager: ChatSessionCacheManager,
}

#[async_trait]
impl BaseApp for ToolShellProcessWrite {
    fn name(&self) -> &'static str {
        "ToolShellProcessWrite"
    }

    async fn start(&self, _ctx: &crate::base::base_app::AppContext) -> Result<()> {
        self.tool_manager.register(Arc::new(self.clone()));
        Ok(())
    }
}

impl ToolBase for ToolShellProcessWrite {
    fn tool_id(&self) -> AiToolId {
        AiToolId::ToolShellProcessWrite
    }

    fn persist(&self) -> bool {
        true
    }

    fn as_rig_tool(&self, done_push: Option<ToolCallDonePush>) -> DynamicTool {
        ToolBaseOutWrap::wrap(self.clone(), self.max_result_tokens(), done_push)
    }
}

impl Tool for ToolShellProcessWrite {
    const NAME: &'static str = AiToolId::ToolShellProcessWrite.as_str();

    type Args = ToolShellProcessWriteArgs;
    type Output = String;
    type Error = AiToolError;

    fn description(&self) -> String {
        "向 shell_exec 以 tty=true 启动的交互终端输入一行，然后返回终端当前屏幕的文字快照。process_id 必填，来自那次 shell_exec 的返回。chars 是要输入的文本，原样送入终端，不要写 \\n 之类的转义；输入后默认自动按 Enter，不想回车就传 enter=false。要发控制键用 ctrl（如 ctrl=\"c\" 是 Ctrl+C，ctrl=\"d\" 是 Ctrl+D），此时 chars 忽略。一次只输入一条，看到提示符回来再发下一条。chars 留空且不传 enter/ctrl 表示只刷新屏幕。返回里若标注「用户在终端里输入了…」，说明用户已在界面上亲自输入过（比如密码），不要重复替用户输；密码等敏感内容一律留给用户在终端里自己输。".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "process_id": {
                    "type": "string",
                    "description": "交互终端进程 id，必填"
                },
                "chars": {
                    "type": "string",
                    "description": "要输入的文本，原样送入终端，不要包含 \\n 等转义；留空表示只刷新屏幕"
                },
                "enter": {
                    "type": "boolean",
                    "description": "输入后是否按 Enter，默认 true；chars 为空时传 true 表示只按一次 Enter"
                },
                "ctrl": {
                    "type": "string",
                    "description": "发送控制键，单个字母：c=Ctrl+C 中断、d=Ctrl+D 结束输入、z=Ctrl+Z、l=Ctrl+L 清屏；传了 ctrl 就不会输入 chars"
                },
                "wait_ms": {
                    "type": "integer",
                    "description": "输入后等多久再截取屏幕，默认 1500，上限 30000；进程退出会提前返回"
                }
            },
            "required": ["process_id"]
        })
    }

    async fn call(&self, context: &mut ToolContext, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let run_context = context.require::<Arc<AiRunContext>>().map_err(|error| AiToolError::Execution(error.to_string()))?.clone();
        let Some(call_id) = args.internal_call_id.clone() else {
            return Err(AiToolError::Execution("internal_call_id of the tool call is missing".to_string()));
        };
        let Some(session_entry) = self.chat_session_cache_manager.get(&run_context.req_id) else {
            return Err(AiToolError::Execution(format!("unknown req_id: {}", run_context.req_id)));
        };
        let chat_id = session_entry.context.ai_chat_id.clone();
        drop(session_entry);
        let Some(entry) = self.local_shell_registry.get(&chat_id, &args.process_id) else {
            return Err(AiToolError::Execution(format!("process_id={} is not among this chat's local command processes", args.process_id)));
        };
        let Some(record) = self.local_shell_tty.record(&args.process_id) else {
            return Err(AiToolError::Execution(format!("process_id={} was not started with tty=true; only interactive terminals accept input", args.process_id)));
        };
        let since_ms = now_millis();
        if !record.is_running() {
            let body = LocalShellTty::render(&record, &entry, record.snapshot().as_ref(), since_ms);
            context.insert_result(vec![WrapItem::wrap_item(None, body.clone())]);
            return Ok(body);
        }
        if let Some(input) = args.terminal_input()? {
            let command = args.describe();
            let payload = ConfirmPayload { process_id: &args.process_id, command: &command };
            self.tool_conflict_manager.confirm_before_execute(&run_context, &call_id, self.tool_id().as_str(), &payload).await?;
            self.local_shell_tty.write(&record, &input).await?;
        }
        let wait = Duration::from_millis(args.wait_ms.unwrap_or(WAIT_DEFAULT_MS).min(WAIT_MAX_MS));
        let snapshot = self.local_shell_tty.capture(&record, wait, &[run_context.cancel_token.clone()]).await;
        let body = LocalShellTty::render(&record, &entry, snapshot.as_ref(), since_ms);
        context.insert_result(vec![WrapItem::wrap_item(None, body.clone())]);
        Ok(body)
    }
}
