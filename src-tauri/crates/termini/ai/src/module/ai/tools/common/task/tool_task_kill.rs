use anyhow::Result;
use std::sync::Arc;
use serde_json::json;
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
use crate::module::ai::shared::ChatSessionCacheManager;
use crate::module::ai::tools::tool_manager::ToolManager;
use crate::module::ai::tools::ai_tool_error::AiToolError;
use crate::module::ai::tools::base::{ToolBaseOutWrap, WrapItem};
use crate::module::ai::tools::common::task::session::task_registry::TaskRegistry;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ToolTaskKillArgs {
    pub task_id: String,

    #[serde(default, rename = "__internal_call_id")]
    pub internal_call_id: Option<String>,
}

#[base_app(sort = 767)]
#[derive(Clone, Service)]
pub struct ToolTaskKill {
    #[inject(component)]
    tool_manager: ToolManager,

    #[inject(component)]
    task_registry: TaskRegistry,

    #[inject(component)]
    chat_session_cache_manager: ChatSessionCacheManager,
}

#[async_trait]
impl BaseApp for ToolTaskKill {
    fn name(&self) -> &'static str {
        "ToolTaskKill"
    }

    async fn start(&self, _ctx: &crate::base::base_app::AppContext) -> Result<()> {
        self.tool_manager.register(Arc::new(self.clone()));
        Ok(())
    }
}

impl ToolBase for ToolTaskKill {
    fn tool_id(&self) -> AiToolId {
        AiToolId::ToolTaskKill
    }

    fn as_rig_tool(&self, done_push: Option<ToolCallDonePush>) -> DynamicTool {
        ToolBaseOutWrap::wrap(self.clone(), self.max_result_tokens(), done_push)
    }
}

impl Tool for ToolTaskKill {
    const NAME: &'static str = AiToolId::ToolTaskKill.as_str();

    type Error = AiToolError;
    type Args = ToolTaskKillArgs;
    type Output = String;

    fn description(&self) -> String {
        "取消一个后台子代理。已经拿到足够信息、或它明显在原地打转时用；取消后它已产出的进度仍可用 task_log 读到。".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "task_id": {
                    "type": "string",
                    "description": "要取消的后台任务 id"
                }
            },
            "required": ["task_id"]
        })
    }

    async fn call(&self, context: &mut ToolContext, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let run_context = context.require::<Arc<AiRunContext>>().map_err(|error| AiToolError::Execution(error.to_string()))?.clone();
        let Some(session) = self.chat_session_cache_manager.get(&run_context.req_id) else {
            return Err(AiToolError::Execution(format!("unknown req_id: {}", run_context.req_id)));
        };
        let chat_id = session.context.ai_chat_id.clone();
        drop(session);

        let Some(entry) = self.task_registry.get(&chat_id, &args.task_id) else {
            return Err(AiToolError::Execution(format!("task_id={} is not among this chat's background tasks", args.task_id)));
        };
        if entry.state().is_finished() {
            let body = format!("[task_kill] already finished, nothing to cancel\n{}", entry.summary_text());
            context.insert_result(vec![WrapItem::wrap_item(None, body.clone())]);
            return Ok(body);
        }
        entry.cancel.cancel();
        let body = format!("[task_kill] 已请求取消\n{}", entry.summary_text());
        context.insert_result(vec![WrapItem::wrap_item(None, body.clone())]);
        Ok(body)
    }
}
