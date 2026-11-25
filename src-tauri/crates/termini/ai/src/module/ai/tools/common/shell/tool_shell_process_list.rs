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
use crate::module::ai::tools::common::shell::local_shell_registry::LocalShellRegistry;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ToolShellProcessListArgs {}

#[base_app(sort = 762)]
#[derive(Clone, Service)]
pub struct ToolShellProcessList {
    #[inject(component)]
    tool_manager: ToolManager,

    #[inject(component)]
    local_shell_registry: LocalShellRegistry,

    #[inject(component)]
    chat_session_cache_manager: ChatSessionCacheManager,
}

#[async_trait]
impl BaseApp for ToolShellProcessList {
    fn name(&self) -> &'static str {
        "ToolShellProcessList"
    }

    async fn start(&self, _ctx: &crate::base::base_app::AppContext) -> Result<()> {
        self.tool_manager.register(Arc::new(self.clone()));
        Ok(())
    }
}

impl ToolBase for ToolShellProcessList {
    fn tool_id(&self) -> AiToolId {
        AiToolId::ToolShellProcessList
    }

    fn as_rig_tool(&self, done_push: Option<ToolCallDonePush>) -> DynamicTool {
        ToolBaseOutWrap::wrap(self.clone(), self.max_result_tokens(), done_push)
    }
}

impl Tool for ToolShellProcessList {
    const NAME: &'static str = AiToolId::ToolShellProcessList.as_str();

    type Args = ToolShellProcessListArgs;
    type Output = String;
    type Error = AiToolError;

    fn description(&self) -> String {
        "列出当前对话由 shell_exec 启动、仍由本应用持有的本地进程：正在执行的、本轮转异步的，以及挂载在后台的（含之前轮次留下的）。无参数。每行给出 process_id、状态（running / exited=N / killed / signal）、pid、已运行秒数、距最后一行输出的静默秒数、输出行数、是否常驻、cwd 和命令原文。只在需要确认某个常驻服务还活着、或怀疑后台有遗留进程时才调。这些进程绑定当前对话、用户可在界面手动停止、应用退出时统一关闭；本工具不是系统进程管理器，要查系统里的其它进程仍用 shell_exec 跑 ps / tasklist。".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {}
        })
    }

    async fn call(&self, context: &mut ToolContext, _args: Self::Args) -> Result<Self::Output, Self::Error> {
        let run_context = context.require::<Arc<AiRunContext>>().map_err(|error| AiToolError::Execution(error.to_string()))?;
        let Some(entry) = self.chat_session_cache_manager.get(&run_context.req_id) else {
            return Err(AiToolError::Execution(format!("unknown req_id: {}", run_context.req_id)));
        };
        let chat_id = entry.context.ai_chat_id.clone();
        drop(entry);
        let rows = self.local_shell_registry.list(&chat_id);
        let body = if rows.is_empty() { "this chat has no local command processes".to_string() } else { rows.iter().map(|entry| entry.summary_text()).collect::<Vec<String>>().join("\n") };
        context.insert_result(vec![WrapItem::wrap_item(None, body.clone())]);
        Ok(body)
    }
}
