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
pub struct ToolTaskListArgs {
    #[serde(default, rename = "__internal_call_id")]
    pub internal_call_id: Option<String>,
}

#[base_app(sort = 764)]
#[derive(Clone, Service)]
pub struct ToolTaskList {
    #[inject(component)]
    tool_manager: ToolManager,

    #[inject(component)]
    task_registry: TaskRegistry,

    #[inject(component)]
    chat_session_cache_manager: ChatSessionCacheManager,
}

#[async_trait]
impl BaseApp for ToolTaskList {
    fn name(&self) -> &'static str {
        "ToolTaskList"
    }

    async fn start(&self, _ctx: &crate::base::base_app::AppContext) -> Result<()> {
        self.tool_manager.register(Arc::new(self.clone()));
        Ok(())
    }
}

impl ToolBase for ToolTaskList {
    fn tool_id(&self) -> AiToolId {
        AiToolId::ToolTaskList
    }

    fn as_rig_tool(&self, done_push: Option<ToolCallDonePush>) -> DynamicTool {
        ToolBaseOutWrap::wrap(self.clone(), self.max_result_tokens(), done_push)
    }
}

impl Tool for ToolTaskList {
    const NAME: &'static str = AiToolId::ToolTaskList.as_str();

    type Error = AiToolError;
    type Args = ToolTaskListArgs;
    type Output = String;

    fn description(&self) -> String {
        "列出本会话已转后台的任务（子代理调查、图片识别）：task_id、类型、状态(running/done/failed/cancelled)、已运行秒数、静默秒数、进度行数、任务描述首行。只返回汇总不返回正文，正文用 task_log 按需读。".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        json!({ "type": "object", "properties": {} })
    }

    async fn call(&self, context: &mut ToolContext, _args: Self::Args) -> Result<Self::Output, Self::Error> {
        let run_context = context.require::<Arc<AiRunContext>>().map_err(|error| AiToolError::Execution(error.to_string()))?;
        let Some(session) = self.chat_session_cache_manager.get(&run_context.req_id) else {
            return Err(AiToolError::Execution(format!("unknown req_id: {}", run_context.req_id)));
        };
        let chat_id = session.context.ai_chat_id.clone();
        drop(session);

        let rows = self.task_registry.list_detached(&chat_id);
        let body = if rows.is_empty() {
            "[task_list] no background tasks".to_string()
        } else {
            let running = rows.iter().filter(|entry| !entry.state().is_finished()).count();
            let lines: Vec<String> = rows.iter().map(|entry| entry.summary_text()).collect();
            format!("[task_list] 共 {} 个, 其中 {} 个还在跑\n{}", rows.len(), running, lines.join("\n"))
        };
        context.insert_result(vec![WrapItem::wrap_item(None, body.clone())]);
        Ok(body)
    }
}
