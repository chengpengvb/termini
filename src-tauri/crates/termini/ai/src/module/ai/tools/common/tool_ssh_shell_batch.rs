use std::sync::Arc;
use serde_json::json;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use crate::base::base_app::BaseApp;
use serde::{Deserialize, Serialize};
use crate::module::ai::dto::AiToolId;
use crate::module::ai::dto::AiRunContext;
use summer::plugin::component::ComponentRef;
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
use crate::service::service_config_ssh::ConfigSshService;
use crate::module::ai::tools::base::{ToolBaseOutWrap, WrapItem};
use crate::module::ssh::client_session_factory::ClientSessionFactory;
use crate::module::ai::tools::common::remote::device_batch::{self, MAX_DEVICE_IDS};
use crate::module::ssh_shell::ssh_shell_exec_core::{resolve_ssh_shell_context, ssh_shell_open_exec_close, SSH_SHELL_EXEC_TIMEOUT};

const BATCH_CONCURRENCY: usize = 8;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ToolSshShellBatchArgs {
    pub device_ids: Vec<String>,

    pub command: String,

    #[serde(default)]
    pub title: Option<String>,

    #[serde(default, rename = "__internal_call_id")]
    pub internal_call_id: Option<String>,
}

#[base_app(sort = 770)]
#[derive(Clone, Service)]
pub struct ToolSshShellBatch {
    #[inject(component)]
    tool_manager: ToolManager,

    #[inject(component)]
    device_service: DeviceService,

    #[inject(component)]
    config_ssh_service: ConfigSshService,

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

    ssh_client_session_factory: ComponentRef<ClientSessionFactory>,
}

#[async_trait]
impl BaseApp for ToolSshShellBatch {
    fn name(&self) -> &'static str {
        "ToolSshShellBatch"
    }

    async fn start(&self, _ctx: &crate::base::base_app::AppContext) -> anyhow::Result<()> {
        self.tool_manager.register(Arc::new(self.clone()));
        Ok(())
    }
}

impl ToolBase for ToolSshShellBatch {
    fn tool_id(&self) -> AiToolId {
        AiToolId::ToolSshShellBatch
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

impl Tool for ToolSshShellBatch {
    const NAME: &'static str = AiToolId::ToolSshShellBatch.as_str();

    type Args = ToolSshShellBatchArgs;
    type Output = String;
    type Error = AiToolError;

    fn description(&self) -> String {
        format!("在一台或多台已登记设备上通过 SSH 执行同一条非交互命令（支持多行脚本），整批一次确认，按机返回 exit_code 与回显，单机连接/执行失败不影响其余设备。目标必须是已登记设备的 device_id（由引用注入中的 group/设备展开，最多 {} 台，不接受 group_id），它们不是本应用所在宿主机，名称或 host 再像本机也不得改用本地执行工具；设备的终端与系统类型以设备自身为准。各机命令不同时分多次调用。命令必须能在有限时间内自行结束，禁止不会自动退出的命令。首次连接会做 SSH 登录，可能因网络、认证、主机密钥失败；某台打开失败后短时间内不要再对它重试，除非用户明确要求或已变更网络与认证配置。", MAX_DEVICE_IDS)
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
                    "description": "已登记设备 id 列表（一台也传数组）；由引用目标展开，不要传 group_id；会去重保序"
                },
                "command": {
                    "type": "string",
                    "description": "在所有目标设备上执行的同一条非交互命令或脚本；必须能在有限时间内自行结束"
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
        let confirm_args = ToolSshShellBatchArgs { device_ids: device_ids.clone(), command: args.command.clone(), title: args.title.clone(), internal_call_id: None };
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
                let ssh_context = match resolve_ssh_shell_context(&device_id, &tool.device_service, &tool.config_ssh_service).await {
                    Ok(ssh_context) => ssh_context,
                    Err(error) => return device_batch::error_item(device_id, String::new(), String::new(), error),
                };
                let name = ssh_context.device.read().name.clone();
                let host = ssh_context.device.read().host.clone();
                if let Err(error) = tool.agent_remote_client_pool.get_or_open_ssh_shell(&run.req_id, &device_id).await {
                    return device_batch::error_item(device_id, name, host, error.detail().to_string());
                }
                let on_chunk = tool.tool_call_chunk_push.build_device_chunk_callback(&run, &call_id, AiToolId::ToolSshShellBatch.as_str(), &device_id);
                let exec = ssh_shell_open_exec_close(command, SSH_SHELL_EXEC_TIMEOUT, &ssh_context, Some(on_chunk), tool.ssh_client_session_factory.clone());
                tokio::select! {
                    biased;
                    res = exec => match res {
                        Ok((exit_code, output)) => device_batch::ok_item(device_id, name, host, Some(exit_code), output),
                        Err(error) => device_batch::error_item(device_id, name, host, error),
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
