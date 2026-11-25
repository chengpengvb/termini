use anyhow::Result;
use std::sync::Arc;
use serde_json::json;
use summer::tracing::warn;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use crate::base::base_app::BaseApp;
use serde::{Deserialize, Serialize};
use crate::module::ai::dto::AiToolId;
use crate::module::ai::shared::WakeBus;
use crate::module::ai::dto::AiRunContext;
use crate::module::ai::shared::ToolCallDonePush;
use rig::tool::{DynamicTool, Tool, ToolContext};
use crate::module::ai::tools::tool_base::ToolBase;
use crate::module::ai::shared::ChatSessionCacheManager;
use crate::module::ai::tools::tool_manager::ToolManager;
use crate::module::ai::tools::ai_tool_error::AiToolError;
use crate::module::ai::tools::base::{ToolBaseOutWrap, WrapItem};
use crate::module::ai::tools::common::shell::local_shell_registry::LocalShellRegistry;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ToolShellProcessKillArgs {
    pub process_id: String,

    pub reason: String,
}

#[base_app(sort = 762)]
#[derive(Clone, Service)]
pub struct ToolShellProcessKill {
    #[inject(component)]
    wake_bus: WakeBus,

    #[inject(component)]
    tool_manager: ToolManager,

    #[inject(component)]
    local_shell_registry: LocalShellRegistry,

    #[inject(component)]
    chat_session_cache_manager: ChatSessionCacheManager,
}

#[async_trait]
impl BaseApp for ToolShellProcessKill {
    fn name(&self) -> &'static str {
        "ToolShellProcessKill"
    }

    async fn start(&self, _ctx: &crate::base::base_app::AppContext) -> Result<()> {
        self.tool_manager.register(Arc::new(self.clone()));
        Ok(())
    }
}

impl ToolBase for ToolShellProcessKill {
    fn tool_id(&self) -> AiToolId {
        AiToolId::ToolShellProcessKill
    }

    fn as_rig_tool(&self, done_push: Option<ToolCallDonePush>) -> DynamicTool {
        ToolBaseOutWrap::wrap(self.clone(), self.max_result_tokens(), done_push)
    }
}

impl Tool for ToolShellProcessKill {
    const NAME: &'static str = AiToolId::ToolShellProcessKill.as_str();

    type Args = ToolShellProcessKillArgs;
    type Output = String;
    type Error = AiToolError;

    fn description(&self) -> String {
        "终止某个本地命令进程及其子进程，process_id 来自 shell_process_list 或 shell_exec 转异步/挂载时返回的那个 id。用于提前结束不再需要的异步命令、关掉已经用不上的常驻服务（比如测试做完了要放掉端口）、停掉目标已达成的后台监控，或清掉僵死的遗留进程。它若是唤醒源，终止时一并注销，之后不会再叫醒你。这些进程绑定当前对话、随应用退出统一关闭，挂载的用户也可在界面手动停止。".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "process_id": {
                    "type": "string",
                    "description": "要终止的进程 id"
                },
                "reason": {
                    "type": "string",
                    "description": "终止原因，一句话"
                }
            },
            "required": ["process_id", "reason"]
        })
    }

    async fn call(&self, context: &mut ToolContext, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let run_context = context.require::<Arc<AiRunContext>>().map_err(|error| AiToolError::Execution(error.to_string()))?.clone();
        let Some(session_entry) = self.chat_session_cache_manager.get(&run_context.req_id) else {
            return Err(AiToolError::Execution(format!("unknown req_id: {}", run_context.req_id)));
        };
        let chat_id = session_entry.context.ai_chat_id.clone();
        drop(session_entry);
        let Some(entry) = self.local_shell_registry.get(&chat_id, &args.process_id) else {
            return Err(AiToolError::Execution(format!("process_id={} is not among this chat's local command processes", args.process_id)));
        };
        if entry.process.exit_status().is_some() {
            let body = format!("process_id={} has already exited, state={}", args.process_id, entry.state_text());
            context.insert_result(vec![WrapItem::wrap_item(None, body.clone())]);
            return Ok(body);
        }
        self.wake_bus.stop(&chat_id, &args.process_id);
        entry.process.kill();
        warn!("[ai] 终止本地进程 chat_id={} process_id={} reason={}", chat_id, args.process_id, args.reason);
        let body = format!("已终止 process_id={}（原因：{}）", args.process_id, args.reason);
        context.insert_result(vec![WrapItem::wrap_item(None, body.clone())]);
        Ok(body)
    }
}
