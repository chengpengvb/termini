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
use crate::model::ai_mcp_server::AiMcpServer;
use crate::module::ai::shared::ToolCallDonePush;
use rig::tool::{DynamicTool, Tool, ToolContext};
use crate::module::ai::tools::tool_base::ToolBase;
use crate::module::ai::shared::ToolActivationStore;
use crate::module::ai::shared::ToolConflictManager;
use crate::module::ai::tools::tool_manager::ToolManager;
use crate::module::ai::tools::ai_tool_error::AiToolError;
use crate::service::service_ai_mcp_server::AiMcpServerService;
use crate::module::ai::tools::base::{ToolBaseOutWrap, WrapItem};

const DEFAULT_SORT: i16 = 100;

fn default_enabled() -> bool {
    true
}

fn default_sort() -> i16 {
    DEFAULT_SORT
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ToolMcpCreateArgs {
    pub name: String,

    pub transport: i16,

    #[serde(default = "default_sort")]
    pub sort: i16,

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

#[base_app(sort = 779)]
#[derive(Clone, Service)]
pub struct ToolMcpCreate {
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
impl BaseApp for ToolMcpCreate {
    fn name(&self) -> &'static str {
        "ToolMcpCreate"
    }

    async fn start(&self, _ctx: &crate::base::base_app::AppContext) -> Result<()> {
        self.tool_manager.register(Arc::new(self.clone()));
        Ok(())
    }
}

impl ToolBase for ToolMcpCreate {
    fn tool_id(&self) -> AiToolId {
        AiToolId::ToolMcpCreate
    }

    fn deferred(&self) -> bool {
        true
    }

    fn as_rig_tool(&self, done_push: Option<ToolCallDonePush>) -> DynamicTool {
        ToolBaseOutWrap::wrap(self.clone(), self.max_result_tokens(), done_push)
    }
}

impl Tool for ToolMcpCreate {
    const NAME: &'static str = AiToolId::ToolMcpCreate.as_str();

    type Args = ToolMcpCreateArgs;
    type Output = String;
    type Error = AiToolError;

    fn description(&self) -> String {
        "把一条 MCP 服务写入本应用（Termini）的 ai_mcp_server。name 服务名；transport 0=stdio、1=http；stdio 必填 command，http 必填 url；args/env/headers 为 JSON 字符串（args 是字符串数组，env/headers 是字符串对象）。仅在用户明确要求给本应用新增 MCP 时使用，禁止改 Claude/Cursor 配置，禁止用 shell_exec 当安装。执行前会请求用户确认。".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
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
                "sort": {
                    "type": "integer",
                    "description": "排序，默认 100",
                    "default": 100
                },
                "command": {
                    "type": "string",
                    "description": "stdio 启动命令，如 npx / uvx / 可执行文件"
                },
                "args": {
                    "type": "string",
                    "description": "stdio 参数，JSON 字符串数组，例如 [\"-y\",\"@scope/pkg\"]"
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
                    "description": "工具超时秒数，不传则用默认"
                }
            },
            "required": ["name", "transport"]
        })
    }

    async fn call(&self, context: &mut ToolContext, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let run_context = context.require::<Arc<AiRunContext>>().map_err(|error| AiToolError::Execution(error.to_string()))?.clone();
        let Some(call_id) = args.internal_call_id.clone() else {
            return Err(AiToolError::Execution("internal_call_id of the tool call is missing".to_string()));
        };
        let confirm_args = ToolMcpCreateArgs { internal_call_id: None, ..args.clone() };
        self.tool_conflict_manager.confirm_before_execute(&run_context, &call_id, self.tool_id().as_str(), &confirm_args).await?;
        let server = AiMcpServer {
            id: String::new(),
            sort: args.sort,
            name: args.name.clone(),
            enabled: args.enabled,
            transport: args.transport,
            env: args.env,
            url: args.url,
            args: args.args,
            headers: args.headers,
            command: args.command,
            timeout_secs: args.timeout_secs,
        };
        self.ai_mcp_server_service.create_server(server).await.map_err(|error| AiToolError::Execution(format!("{:#}", error)))?;
        self.tool_activation_store.forget_catalog(&run_context.req_id).await;
        let body = format!("已创建 MCP：{}", args.name);
        context.insert_result(vec![WrapItem::wrap_item(None, body.clone())]);
        Ok(body)
    }
}
