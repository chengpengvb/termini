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
use crate::service::service_base::BaseService;
use crate::module::ai::shared::ToolCallDonePush;
use rig::tool::{DynamicTool, Tool, ToolContext};
use crate::module::ai::tools::tool_base::ToolBase;
use crate::module::ai::shared::ToolActivationStore;
use crate::module::ai::shared::ToolConflictManager;
use crate::module::ai::tools::tool_manager::ToolManager;
use crate::module::ai::tools::ai_tool_error::AiToolError;
use crate::service::service_ai_mcp_server::AiMcpServerService;
use crate::module::ai::tools::base::{ToolBaseOutWrap, WrapItem};

fn default_enabled() -> bool {
    true
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ToolMcpUpdateArgs {
    pub id: String,
    pub name: String,
    pub transport: i16,

    #[serde(default = "default_enabled")]
    pub enabled: bool,

    #[serde(default)]
    pub command: Option<String>,

    #[serde(default)]
    pub args: Option<String>,

    #[serde(default)]
    pub env: Option<String>,

    #[serde(default)]
    pub url: Option<String>,

    #[serde(default)]
    pub headers: Option<String>,

    #[serde(default)]
    pub timeout_secs: Option<i32>,

    #[serde(default, rename = "__internal_call_id")]
    pub internal_call_id: Option<String>,
}

#[base_app(sort = 780)]
#[derive(Clone, Service)]
pub struct ToolMcpUpdate {
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
impl BaseApp for ToolMcpUpdate {
    fn name(&self) -> &'static str {
        "ToolMcpUpdate"
    }

    async fn start(&self, _ctx: &crate::base::base_app::AppContext) -> Result<()> {
        self.tool_manager.register(Arc::new(self.clone()));
        Ok(())
    }
}

impl ToolBase for ToolMcpUpdate {
    fn tool_id(&self) -> AiToolId {
        AiToolId::ToolMcpUpdate
    }

    fn deferred(&self) -> bool {
        true
    }

    fn as_rig_tool(&self, done_push: Option<ToolCallDonePush>) -> DynamicTool {
        ToolBaseOutWrap::wrap(self.clone(), self.max_result_tokens(), done_push)
    }
}

impl Tool for ToolMcpUpdate {
    const NAME: &'static str = AiToolId::ToolMcpUpdate.as_str();

    type Args = ToolMcpUpdateArgs;
    type Output = String;
    type Error = AiToolError;

    fn description(&self) -> String {
        "修改本应用一条已有的 MCP 服务。先用 select_all_mcp 查到 id，再传 id 与新的 name / transport / command / args / env / url / headers / enabled 覆盖。仅在用户明确要求修改本应用 MCP 时使用。执行前会请求用户确认。".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "id": {
                    "type": "string",
                    "description": "要修改的 MCP id（用 select_all_mcp 查到）"
                },
                "name": {
                    "type": "string",
                    "description": "MCP 服务名"
                },
                "transport": {
                    "type": "integer",
                    "description": "0=stdio，1=http"
                },
                "enabled": {
                    "type": "boolean",
                    "description": "是否启用，默认 true",
                    "default": true
                },
                "command": {
                    "type": "string",
                    "description": "stdio 启动命令"
                },
                "args": {
                    "type": "string",
                    "description": "stdio 参数，JSON 字符串数组"
                },
                "env": {
                    "type": "string",
                    "description": "stdio 环境变量，JSON 字符串对象"
                },
                "url": {
                    "type": "string",
                    "description": "http 传输的 URL"
                },
                "headers": {
                    "type": "string",
                    "description": "http 请求头，JSON 字符串对象"
                },
                "timeout_secs": {
                    "type": "integer",
                    "description": "工具超时秒数"
                }
            },
            "required": ["id", "name", "transport"]
        })
    }

    async fn call(&self, context: &mut ToolContext, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let run_context = context.require::<Arc<AiRunContext>>().map_err(|error| AiToolError::Execution(error.to_string()))?.clone();
        let Some(call_id) = args.internal_call_id.clone() else {
            return Err(AiToolError::Execution("internal_call_id of the tool call is missing".to_string()));
        };
        let confirm_args = ToolMcpUpdateArgs { internal_call_id: None, ..args.clone() };
        self.tool_conflict_manager.confirm_before_execute(&run_context, &call_id, self.tool_id().as_str(), &confirm_args).await?;
        let existing = self.ai_mcp_server_service.find_by_id(args.id.clone()).await.map_err(|error| AiToolError::Execution(error.to_string()))?;
        let Some(shared) = existing else {
            return Err(AiToolError::Execution(format!("mcp not found: {}", args.id)));
        };
        let mut server = shared.read().clone();
        server.name = args.name.clone();
        server.transport = args.transport;
        server.enabled = args.enabled;
        server.command = args.command;
        server.args = args.args;
        server.env = args.env;
        server.url = args.url;
        server.headers = args.headers;
        server.timeout_secs = args.timeout_secs;
        self.ai_mcp_server_service.update_server(server).await.map_err(|error| AiToolError::Execution(format!("{:#}", error)))?;
        self.tool_activation_store.forget_catalog(&run_context.req_id).await;
        let body = format!("已更新 MCP：{}", args.name);
        context.insert_result(vec![WrapItem::wrap_item(None, body.clone())]);
        Ok(body)
    }
}
