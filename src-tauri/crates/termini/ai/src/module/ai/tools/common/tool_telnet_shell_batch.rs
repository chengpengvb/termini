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
use crate::module::ai::shared::ToolCallChunkPush;
use crate::module::ai::tools::tool_base::ToolBase;
use crate::service::service_device::DeviceService;
use crate::module::ai::shared::ToolConflictManager;
use crate::module::ai::shared::AgentRemoteClientPool;
use crate::module::ai::shared::ChatSessionCacheManager;
use crate::module::ai::tools::tool_manager::ToolManager;
use crate::module::ai::tools::ai_tool_error::AiToolError;
use crate::module::ai::tools::base::{ToolBaseOutWrap, WrapItem};
use crate::module::ai::tools::common::remote::device_batch::{self, MAX_DEVICE_IDS};
use crate::module::telnet_shell::telnet_shell_exec_core::TELNET_SHELL_EXEC_TIMEOUT;

const BATCH_CONCURRENCY: usize = 4;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ToolTelnetShellBatchArgs {
    pub device_ids: Vec<String>,

    pub command: String,

    #[serde(default)]
    pub title: Option<String>,

    #[serde(default, rename = "__internal_call_id")]
    pub internal_call_id: Option<String>,
}

#[base_app(sort = 770)]
#[derive(Clone, Service)]
pub struct ToolTelnetShellBatch {
    #[inject(component)]
    tool_manager: ToolManager,

    #[inject(component)]
    device_service: DeviceService,

    #[inject(component)]
    tool_call_done_push: ToolCallDonePush,

    #[inject(component)]
    tool_call_chunk_push: ToolCallChunkPush,

    #[inject(component)]
    tool_conflict_manager: ToolConflictManager,

    #[inject(component)]
    agent_remote_client_pool: AgentRemoteClientPool,

    #[inject(component)]
    chat_session_cache_manager: ChatSessionCacheManager,
}

#[async_trait]
impl BaseApp for ToolTelnetShellBatch {
    fn name(&self) -> &'static str {
        "ToolTelnetShellBatch"
    }

    async fn start(&self, _ctx: &crate::base::base_app::AppContext) -> anyhow::Result<()> {
        self.tool_manager.register(Arc::new(self.clone()));
        Ok(())
    }
}

impl ToolBase for ToolTelnetShellBatch {
    fn tool_id(&self) -> AiToolId {
        AiToolId::ToolTelnetShellBatch
    }

    fn on_stop(&self, req_id: &str) {
        self.agent_remote_client_pool.release(req_id);
    }

    fn persist(&self) -> bool {
        true
    }

    fn as_rig_tool(&self, done_push: Option<ToolCallDonePush>) -> DynamicTool {
        ToolBaseOutWrap::wrap(self.clone(), self.max_result_tokens(), done_push)
    }
}

impl Tool for ToolTelnetShellBatch {
    const NAME: &'static str = AiToolId::ToolTelnetShellBatch.as_str();

    type Error = AiToolError;
    type Args = ToolTelnetShellBatchArgs;
    type Output = String;

    fn description(&self) -> String {
        format!("在一台或多台已登记的 Telnet 设备上执行同一条（或同一段多行，在每台的同一条连接上串行执行）命令，面向不支持 SSH 的交换机、路由器等网络设备；整批一次确认，按机汇总回显，单机连接/登录失败不影响其余设备。目标必须是已配置 Telnet 的已登记设备 device_id（由引用注入中的 group/设备展开，最多 {} 台，不接受 group_id），它们不是本应用所在宿主机；同时支持 SSH 的设备优先 ssh_shell_exec_batch。Telnet 没有结束信号与退出码，靠提示符判定命令结束，成败要自己从回显判断（\"% Invalid input\"、\"Error:\"、\"Unrecognized command\" 即失败）。命令必须非交互：\"--More--\" 分页会自动续翻，但 \"[Y/N]\" 这类确认无法应答会等到超时，请改用带免确认参数的写法；禁止不会自动退出的命令（如不带 count 的 ping）。某台打开失败后短时间内不要再对它重试，除非用户明确要求或已变更网络与认证配置。", MAX_DEVICE_IDS)
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "title": {
                    "type": "string",
                    "description": "这条命令在做什么的短标题，语言遵循 [输出语言]。界面上用它标识这一条，长命令不至于只能看到被截断的开头"
                },
                "device_ids": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "已登记且配置了 Telnet 的设备 id 列表（一台也传数组）；由引用目标展开，不要传 group_id；会去重保序"
                },
                "command": {
                    "type": "string",
                    "description": "在所有目标设备上执行的命令；多行表示多条命令，在每台设备的同一条连接上按顺序串行执行。必须非交互且能自行结束"
                }
            },
            "required": ["device_ids", "command"]
        })
    }

    async fn call(&self, context: &mut ToolContext, args: Self::Args) -> anyhow::Result<Self::Output, Self::Error> {
        let run_context = context.require::<Arc<AiRunContext>>().map_err(|error| AiToolError::Execution(error.to_string()))?.clone();
        let Some(call_id) = args.internal_call_id.clone() else {
            return Err(AiToolError::Execution("internal_call_id of the tool call is missing".to_string()));
        };
        if args.command.trim().is_empty() {
            return Err(AiToolError::Execution("command is empty".to_string()));
        }
        let device_ids = device_batch::normalize_device_ids(args.device_ids)?;
        let session_cancel = run_context.cancel_token.clone();
        let Some(tool_cancel) = self.chat_session_cache_manager.get_tool_cancel(&run_context.req_id, &run_context.run_id, &call_id) else {
            return Err(AiToolError::Execution(format!("tool cancel not found: req_id={} run_id={} call_id={}", run_context.req_id, run_context.run_id, call_id)));
        };
        let confirm_args = ToolTelnetShellBatchArgs { device_ids: device_ids.clone(), command: args.command.clone(), title: args.title.clone(), internal_call_id: None };
        self.tool_conflict_manager.confirm_before_execute(&run_context, &call_id, self.tool_id().as_str(), &confirm_args).await?;

        let tool = self.clone();
        let run = Arc::clone(&run_context);
        let call_id_owned = call_id.clone();
        let command = args.command.clone();
        let tool_cancel_token = tool_cancel.clone();
        let session_cancel_token = session_cancel.clone();
        let items = device_batch::run_device_batch(device_ids, BATCH_CONCURRENCY, &tool_cancel, &session_cancel, move |device_id: String| {
            let tool = tool.clone();
            let run = Arc::clone(&run);
            let call_id = call_id_owned.clone();
            let command = command.clone();
            let tool_cancel_token = tool_cancel_token.clone();
            let session_cancel_token = session_cancel_token.clone();
            async move {
                let (name, host) = match device_batch::device_label(&tool.device_service, &device_id).await {
                    Ok(label) => label,
                    Err(error) => return device_batch::error_item(device_id, String::new(), String::new(), error),
                };
                let client = match tool.agent_remote_client_pool.get_or_open_telnet_shell(&run.req_id, &device_id).await {
                    Ok(client) => client,
                    Err(error) => return device_batch::error_item(device_id, name, host, error.detail().to_string()),
                };
                let on_chunk = tool.tool_call_chunk_push.build_device_chunk_callback(&run, &call_id, AiToolId::ToolTelnetShellBatch.as_str(), &device_id);
                tokio::select! {
                    biased;
                    res = client.exec(command, TELNET_SHELL_EXEC_TIMEOUT, Some(on_chunk)) => match res {
                        Ok(output) => {
                            let output = if output.trim().is_empty() { "command completed, the device produced no output".to_string() } else { output };
                            device_batch::ok_item(device_id, name, host, None, output)
                        }
                        Err(error) => device_batch::error_item(device_id, name, host, error.to_string()),
                    },
                    _ = tool_cancel_token.cancelled() => device_batch::cancelled_item(device_id, name, host),
                    _ = session_cancel_token.cancelled() => device_batch::cancelled_item(device_id, name, host),
                }
            }
        })
        .await;

        let json = serde_json::to_string(&items).map_err(|error| AiToolError::Execution(error.to_string()))?;
        self.tool_call_done_push.push_tool_call_done_event(&run_context, &call_id, json);
        let mut wrap_items: Vec<WrapItem> = Vec::with_capacity(items.len());
        for item in &items {
            let body = serde_json::to_string(item).map_err(|error| AiToolError::Execution(error.to_string()))?;
            wrap_items.push(WrapItem::wrap_item(Some(format!("===== {} =====", item.device_id)), body));
        }
        context.insert_result(wrap_items);
        Ok(String::new())
    }
}
