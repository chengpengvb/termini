use anyhow::Result;
use std::sync::Arc;
use serde_json::json;
use rig::prelude::Tool;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use crate::base::base_app::BaseApp;
use serde::{Deserialize, Serialize};
use crate::module::ai::dto::AiToolId;
use crate::module::ai::dto::AiRunContext;
use rig::tool::{DynamicTool, ToolContext};
use crate::module::ai::shared::ToolCallDonePush;
use crate::module::ai::shared::ChatSessionCacheManager;
use crate::module::ai::tools::tool_manager::ToolManager;
use crate::module::ai::tools::ai_tool_error::AiToolError;
use crate::module::ai::tools::base::{ToolBaseOutWrap, WrapItem};
use crate::module::ai::tools::tool_base::{ToolBase, ToolDomain};
use crate::module::ai::shared::{ShellDaemonPush, WakeBus, WakeSourceKind};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ToolLoopStopArgs {
    pub loop_id: String,

    pub reason: String,

    #[serde(default, rename = "__internal_call_id")]
    pub internal_call_id: Option<String>,
}

#[base_app(sort = 763)]
#[derive(Clone, Service)]
pub struct ToolLoopStop {
    #[inject(component)]
    wake_bus: WakeBus,

    #[inject(component)]
    tool_manager: ToolManager,

    #[inject(component)]
    shell_daemon_push: ShellDaemonPush,

    #[inject(component)]
    chat_session_cache_manager: ChatSessionCacheManager,
}

#[async_trait]
impl BaseApp for ToolLoopStop {
    fn name(&self) -> &'static str {
        "ToolLoopStop"
    }

    async fn start(&self, _ctx: &crate::base::base_app::AppContext) -> Result<()> {
        self.tool_manager.register(Arc::new(self.clone()));
        Ok(())
    }
}

impl ToolBase for ToolLoopStop {
    fn tool_id(&self) -> AiToolId {
        AiToolId::ToolLoopStop
    }

    fn domains(&self) -> &'static [ToolDomain] {
        &[ToolDomain::Main]
    }

    fn as_rig_tool(&self, done_push: Option<ToolCallDonePush>) -> DynamicTool {
        ToolBaseOutWrap::wrap(self.clone(), self.max_result_tokens(), done_push)
    }
}

impl Tool for ToolLoopStop {
    const NAME: &'static str = AiToolId::ToolLoopStop.as_str();

    type Args = ToolLoopStopArgs;
    type Output = String;
    type Error = AiToolError;

    fn description(&self) -> String {
        "停止一个由 loop_start 启动的后台循环监控并把它从唤醒源注销，之后它不会再叫醒你。loop_id 来自 loop_start 的返回或 [后台唤醒] 消息里的 id。目标已达成、监控条件不再需要时调用；本机后台命令用 shell_process_kill 停，不是本工具。".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "loop_id": {
                    "type": "string",
                    "description": "要停止的循环监控 id"
                },
                "reason": {
                    "type": "string",
                    "description": "停止原因，一句话"
                }
            },
            "required": ["loop_id", "reason"]
        })
    }

    async fn call(&self, context: &mut ToolContext, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let run_context = context.require::<Arc<AiRunContext>>().map_err(|error| AiToolError::Execution(error.to_string()))?;
        let Some(session) = self.chat_session_cache_manager.get(&run_context.req_id) else {
            return Err(AiToolError::Execution(format!("unknown req_id: {}", run_context.req_id)));
        };
        let chat_id = session.context.ai_chat_id.clone();
        drop(session);
        let Some(source) = self.wake_bus.get(&chat_id, &args.loop_id).filter(|source| source.kind == WakeSourceKind::Loop) else {
            return Err(AiToolError::Execution(format!("loop_id={} is not among this chat's running loops", args.loop_id)));
        };
        self.wake_bus.stop(&chat_id, &args.loop_id);
        self.shell_daemon_push.push_daemon_list(&chat_id);
        let body = format!("已停止循环监控「{}」 loop_id={}（原因：{}），之后它不会再唤醒你", source.title, args.loop_id, args.reason);
        context.insert_result(vec![WrapItem::wrap_item(None, body.clone())]);
        Ok(body)
    }
}
