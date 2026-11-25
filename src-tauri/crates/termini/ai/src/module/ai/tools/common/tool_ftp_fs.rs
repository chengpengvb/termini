use std::sync::Arc;
use serde_json::json;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use crate::base::base_app::BaseApp;
use crate::module::ai::dto::AiToolId;
use crate::module::ai::dto::AiRunContext;
use crate::module::ai::shared::ToolCallDonePush;
use rig::tool::{DynamicTool, Tool, ToolContext};
use crate::module::ai::tools::tool_base::ToolBase;
use crate::module::ai::shared::ToolConflictManager;
use crate::module::ai::shared::AgentRemoteClientPool;
use crate::module::ai::tools::tool_manager::ToolManager;
use crate::web::ws::handle::r#impl::sftp::SessionClient;
use crate::module::ai::tools::ai_tool_error::AiToolError;
use crate::web::ws::handle::r#impl::sftp::RemoteSessionClient;
use crate::module::ai::tools::base::{ToolBaseOutWrap, WrapItem};
use crate::module::ai::tools::common::remote::fs_ops::{self, ToolFsArgs};

#[base_app(sort = 767)]
#[derive(Clone, Service)]
pub struct ToolFtpFs {
    #[inject(component)]
    tool_manager: ToolManager,

    #[inject(component)]
    tool_conflict_manager: ToolConflictManager,

    #[inject(component)]
    agent_remote_client_pool: AgentRemoteClientPool,
}

#[async_trait]
impl BaseApp for ToolFtpFs {
    fn name(&self) -> &'static str {
        "ToolFtpFs"
    }

    async fn start(&self, _ctx: &crate::base::base_app::AppContext) -> anyhow::Result<()> {
        self.tool_manager.register(Arc::new(self.clone()));
        Ok(())
    }
}

impl ToolBase for ToolFtpFs {
    fn tool_id(&self) -> AiToolId {
        AiToolId::ToolFtpFs
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

impl Tool for ToolFtpFs {
    const NAME: &'static str = AiToolId::ToolFtpFs.as_str();

    type Error = AiToolError;
    type Args = ToolFsArgs;
    type Output = String;

    fn description(&self) -> String {
        "经 FTP 在已登记设备上做 list/rename/delete/create。须传 device_id（不能是本机）与 op。list：path 为远端目录绝对路径。rename：from/to 为绝对路径。delete：path 为要删项的绝对路径；同一目录多个用 path（父目录）+ names。create：path 为新建项绝对路径，或 path（父目录）+ name，kind 为 file 或 directory。FTP 不支持 chmod。写操作会先请用户确认。".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "device_id": { "type": "string", "description": "已登记 FTP 设备 id，不能是本机" },
                "op": {
                    "type": "string",
                    "enum": ["list", "rename", "delete", "create"],
                    "description": "list 列目录；rename 重命名；delete 删除；create 新建文件或目录"
                },
                "path": { "type": "string", "description": "远端绝对路径。list 为目录；delete 为单项目标或与 names 搭配的父目录；create 为新建项路径或与 name 搭配的父目录" },
                "name": { "type": "string", "description": "create 时的新文件/目录名，与 path 父目录搭配" },
                "names": { "type": "array", "items": { "type": "string" }, "description": "delete 同一目录下的多个名字，与 path 父目录搭配" },
                "from": { "type": "string", "description": "rename 源绝对路径" },
                "to": { "type": "string", "description": "rename 目标绝对路径" },
                "kind": { "type": "string", "enum": ["file", "directory"], "description": "create 必填" }
            },
            "required": ["device_id", "op"]
        })
    }

    async fn call(&self, context: &mut ToolContext, args: Self::Args) -> anyhow::Result<Self::Output, Self::Error> {
        let run_context = context.require::<Arc<AiRunContext>>().map_err(|error| AiToolError::Execution(error.to_string()))?.clone();
        let Some(call_id) = args.internal_call_id.clone() else {
            return Err(AiToolError::Execution("internal_call_id of the tool call is missing".to_string()));
        };
        let op = args.op.trim().to_string();
        fs_ops::validate(&op, &args, false)?;
        if fs_ops::is_write(&op) {
            let confirm_args = ToolFsArgs { internal_call_id: None, ..args.clone() };
            self.tool_conflict_manager.confirm_before_execute(&run_context, &call_id, self.tool_id().as_str(), &confirm_args).await?;
        }
        let remote = self.agent_remote_client_pool.get_or_open_ftp(&run_context.req_id, args.device_id.trim()).await?;
        let client = SessionClient::Remote(RemoteSessionClient::Ftp(remote));
        let body = fs_ops::execute(&client, &op, &args).await?;
        let rendered = serde_json::to_string(&body).map_err(|error| AiToolError::Execution(error.to_string()))?;
        context.insert_result(vec![WrapItem::wrap_item(None, rendered)]);
        Ok(String::new())
    }
}
