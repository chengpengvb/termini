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
use crate::web::ws::ws_manager::WsManager;
use crate::module::ai::shared::ToolCallDonePush;
use rig::tool::{DynamicTool, Tool, ToolContext};
use crate::module::ai::tools::tool_base::ToolBase;
use crate::service::service_device::DeviceService;
use crate::module::ai::shared::ToolConflictManager;
use crate::module::ai::tools::tool_manager::ToolManager;
use crate::module::ai::tools::ai_tool_error::AiToolError;
use crate::module::ai::tools::base::{ToolBaseOutWrap, WrapItem};
use crate::model::resp::event::resp_event_entity_notify::RespEventEntityNotify;

const PROFILE_MAX_CHARS: usize = 4000;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ToolDeviceProfileUpdateArgs {
    pub device_id: String,
    pub profile: String,

    #[serde(default, rename = "__internal_call_id")]
    pub internal_call_id: Option<String>,
}

#[base_app(sort = 762)]
#[derive(Clone, Service)]
pub struct ToolDeviceProfileUpdate {
    #[inject(component)]
    ws_manager: WsManager,

    #[inject(component)]
    tool_manager: ToolManager,

    #[inject(component)]
    device_service: DeviceService,

    #[inject(component)]
    tool_conflict_manager: ToolConflictManager,
}

#[async_trait]
impl BaseApp for ToolDeviceProfileUpdate {
    fn name(&self) -> &'static str {
        "ToolDeviceProfileUpdate"
    }

    async fn start(&self, _ctx: &crate::base::base_app::AppContext) -> Result<()> {
        self.tool_manager.register(Arc::new(self.clone()));
        Ok(())
    }
}

impl ToolBase for ToolDeviceProfileUpdate {
    fn tool_id(&self) -> AiToolId {
        AiToolId::ToolDeviceProfileUpdate
    }

    fn as_rig_tool(&self, done_push: Option<ToolCallDonePush>) -> DynamicTool {
        ToolBaseOutWrap::wrap(self.clone(), self.max_result_tokens(), done_push)
    }
}

impl Tool for ToolDeviceProfileUpdate {
    const NAME: &'static str = AiToolId::ToolDeviceProfileUpdate.as_str();

    type Args = ToolDeviceProfileUpdateArgs;
    type Output = String;
    type Error = AiToolError;

    fn description(&self) -> String {
        "沉淀或更新设备档案（device.profile）。档案是对这台已登记设备长期有效的认知，会自动带入后续所有涉及它的对话、计划任务与告警诊断。传 device_id 与 profile，profile 整体替换旧档案，所以调用前必须先拿到现有档案（聊天引用注入的 profile 字段，或 select_all_device 查询结果），在旧档案基础上合并提炼：保留仍然有效的内容，改正被实践证伪的，并入新发现；禁止只写本次结论把旧档案冲掉。在设备上完成部署/安装/迁移或域名、端口、证书等配置变更后，必须在收尾汇报前主动调用本工具沉淀新的服务路径、端口、域名、启动方式，不要等用户提醒。只沉淀长期事实：机器角色、关键服务与部署路径、组网与跳板关系、运维约定、已知坑的结论、用户明确交代的指导性信息（如「配置在 /opt/app 下，别去 /etc 找」）。与技能分工：档案记结论性事实，操作步骤与排障过程归技能，技能里已有的内容最多留一句结论，禁止整段复制。不要写入一次性操作过程、瞬时状态（当日磁盘占用、临时进程列表、文件大小、依赖个数）、密码或密钥等任何凭据。本次没有产生新的长期认知就不要调用。宿主机（本应用所在机器）的档案用固定 device_id `000000`，现有内容在上下文的 [Host Profile] 里；它仍然不是远程设备，远程工具不接受这个 id。执行前会把新档案全文展示给用户请求确认。".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "device_id": {
                    "type": "string",
                    "description": "目标设备 id（已登记设备，来自聊天引用或 select_all_device；宿主机固定为 000000）"
                },
                "profile": {
                    "type": "string",
                    "description": "合并提炼后的新档案全文，纯文本，整体替换旧档案，语言遵循 [输出语言]。建议按 角色 / 关键服务与路径 / 运维约定 / 已知坑 组织，上限 4000 字符。"
                }
            },
            "required": ["device_id", "profile"]
        })
    }

    async fn call(&self, context: &mut ToolContext, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let device_id = args.device_id.trim().to_string();
        if device_id.is_empty() {
            return Err(AiToolError::Execution("device_id is empty".to_string()));
        }
        let profile = args.profile.trim().to_string();
        if profile.is_empty() {
            return Err(AiToolError::Execution("profile content is empty; clearing the profile is not allowed".to_string()));
        }
        let chars = profile.chars().count();
        if chars > PROFILE_MAX_CHARS {
            return Err(AiToolError::Execution(format!("the profile should be distilled long-term knowledge, at most {PROFILE_MAX_CHARS} characters, got {chars}")));
        }
        let run_context = context.require::<Arc<AiRunContext>>().map_err(|error| AiToolError::Execution(error.to_string()))?.clone();
        let Some(call_id) = args.internal_call_id.clone() else {
            return Err(AiToolError::Execution("internal_call_id of the tool call is missing".to_string()));
        };
        let confirm_args = ToolDeviceProfileUpdateArgs { device_id: device_id.clone(), profile: profile.clone(), internal_call_id: None };
        self.tool_conflict_manager.confirm_before_execute(&run_context, &call_id, self.tool_id().as_str(), &confirm_args).await?;
        self.device_service.update_profile(device_id.clone(), profile).await.map_err(|error| AiToolError::Execution(error.to_string()))?;
        self.ws_manager.broadcast_all(&RespEventEntityNotify::OnChanged(1).to_resp());
        let body = format!("已沉淀设备 {} 的档案, 共 {} 字符", device_id, chars);
        context.insert_result(vec![WrapItem::wrap_item(None, body.clone())]);
        Ok(body)
    }
}
