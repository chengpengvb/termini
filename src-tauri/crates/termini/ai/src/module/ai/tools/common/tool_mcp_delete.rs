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
use crate::module::ai::shared::ToolActivationStore;
use crate::module::ai::shared::ToolConflictManager;
use crate::module::ai::tools::tool_manager::ToolManager;
use crate::module::ai::tools::ai_tool_error::AiToolError;
use crate::service::service_ai_mcp_server::AiMcpServerService;
use crate::module::ai::tools::base::{ToolBaseOutWrap, WrapItem};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ToolMcpDeleteArgs {
    pub id: String,

    #[serde(default, rename = "__internal_call_id")]
    pub internal_call_id: Option<String>,
}

#[base_app(sort = 781)]
#[derive(Clone, Service)]
pub struct ToolMcpDelete {
    #[inject(component)]
    tool_manager: ToolManager,

    #[inject(component)]
    tool_activation_store: ToolActivationStore,

    #[inject(component)]
    ai_mcp_server_service: AiMcpServerService,

    #[inject(component)]
    tool_conflict_manager: ToolConflictManager,
}

#[async_trait]
impl BaseApp for ToolMcpDelete {
    fn name(&self) -> &'static str {
        "ToolMcpDelete"
    }

    async fn start(&self, _ctx: &crate::base::base_app::AppContext) -> Result<()> {
        self.tool_manager.register(Arc::new(self.clone()));
        Ok(())
    }
}

impl ToolBase for ToolMcpDelete {
    fn tool_id(&self) -> AiToolId {
        AiToolId::ToolMcpDelete
    }

    fn deferred(&self) -> bool {
        true
    }

    fn as_rig_tool(&self, done_push: Option<ToolCallDonePush>) -> DynamicTool {
        ToolBaseOutWrap::wrap(self.clone(), self.max_result_tokens(), done_push)
    }
}

impl Tool for ToolMcpDelete {
    const NAME: &'static str = AiToolId::ToolMcpDelete.as_str();

    type Args = ToolMcpDeleteArgs;
    type Output = String;
    type Error = AiToolError;

    fn description(&self) -> String {
        "删除本应用一条已有的 MCP 服务。先用 select_all_mcp 查到 id，再传 id 删除。仅在用户明确要求删除本应用 MCP 时使用。执行前会请求用户确认。".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "id": {
                    "type": "string",
                    "description": "要删除的 MCP id（用 select_all_mcp 查到）"
                }
            },
            "required": ["id"]
        })
    }

    async fn call(&self, context: &mut ToolContext, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let run_context = context.require::<Arc<AiRunContext>>().map_err(|error| AiToolError::Execution(error.to_string()))?.clone();
        let Some(call_id) = args.internal_call_id.clone() else {
            return Err(AiToolError::Execution("internal_call_id of the tool call is missing".to_string()));
        };
        let confirm_args = ToolMcpDeleteArgs { id: args.id.clone(), internal_call_id: None };
        self.tool_conflict_manager.confirm_before_execute(&run_context, &call_id, self.tool_id().as_str(), &confirm_args).await?;
        self.ai_mcp_server_service.delete_server(args.id.clone()).await.map_err(|error| AiToolError::Execution(format!("{:#}", error)))?;
        self.tool_activation_store.forget_catalog(&run_context.req_id).await;
        let body = format!("已删除 MCP：{}", args.id);
        context.insert_result(vec![WrapItem::wrap_item(None, body.clone())]);
        Ok(body)
    }
}
