use anyhow::Result;
use std::sync::Arc;
use serde_json::json;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use crate::model::device::Device;
use crate::base::base_app::BaseApp;
use serde::{Deserialize, Serialize};
use crate::module::ai::dto::AiToolId;
use crate::module::ai::dto::AiRunContext;
use crate::model::base_target::BaseTarget;
use crate::web::ws::ws_manager::WsManager;
use crate::module::ai::shared::ToolCallDonePush;
use rig::tool::{DynamicTool, Tool, ToolContext};
use crate::module::ai::tools::tool_base::ToolBase;
use crate::module::ai::shared::ToolConflictManager;
use crate::module::ai::tools::tool_manager::ToolManager;
use crate::module::ai::tools::ai_tool_error::AiToolError;
use crate::model::resp::resp_device_import::DeviceImportItem;
use crate::model::req::req_device_import::ReqDeviceImportApply;
use crate::service::service_device_import::DeviceImportService;
use crate::module::ai::tools::base::{ToolBaseOutWrap, WrapItem};
use crate::model::resp::event::resp_event_entity_notify::RespEventEntityNotify;

const ITEMS_MAX: usize = 500;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ToolDeviceImportArgs {
    pub items: Vec<DeviceImportItem>,

    #[serde(default, rename = "__internal_call_id")]
    pub internal_call_id: Option<String>,
}

#[base_app(sort = 763)]
#[derive(Clone, Service)]
pub struct ToolDeviceImport {
    #[inject(component)]
    ws_manager: WsManager,

    #[inject(component)]
    tool_manager: ToolManager,

    #[inject(component)]
    tool_conflict_manager: ToolConflictManager,

    #[inject(component)]
    device_import_service: DeviceImportService,
}

#[async_trait]
impl BaseApp for ToolDeviceImport {
    fn name(&self) -> &'static str {
        "ToolDeviceImport"
    }

    async fn start(&self, _ctx: &crate::base::base_app::AppContext) -> Result<()> {
        self.tool_manager.register(Arc::new(self.clone()));
        Ok(())
    }
}

impl ToolBase for ToolDeviceImport {
    fn tool_id(&self) -> AiToolId {
        AiToolId::ToolDeviceImport
    }

    fn as_rig_tool(&self, done_push: Option<ToolCallDonePush>) -> DynamicTool {
        ToolBaseOutWrap::wrap(self.clone(), self.max_result_tokens(), done_push)
    }
}

impl ToolDeviceImport {
    fn require_items(items: Vec<DeviceImportItem>) -> Result<Vec<DeviceImportItem>, AiToolError> {
        if items.is_empty() {
            return Err(AiToolError::Execution("items is empty".to_string()));
        }
        if items.len() > ITEMS_MAX {
            return Err(AiToolError::Execution(format!("too many items: {} exceeds {}", items.len(), ITEMS_MAX)));
        }
        Ok(items)
    }
}

impl Tool for ToolDeviceImport {
    const NAME: &'static str = AiToolId::ToolDeviceImport.as_str();

    type Error = AiToolError;
    type Args = ToolDeviceImportArgs;
    type Output = serde_json::Value;

    fn description(&self) -> String {
        format!("批量把设备写入本应用登记表。只收字段，不解析任何文件或会话源。items 一次传入数组，不要一条条调，一次最多 {} 条，超了直接报错。每项 name、host、port 必填，username、jump_name 选填。host:port 已登记则跳过。jump_name 按已有设备名或 host 关联跳板。会请求确认。密钥和登录密码不走本工具：密钥 insert config_key_pair 再把 id 写到 config_ssh.config_key_pair_id，密码写 config_ssh.base.password，都用 entity_call。", ITEMS_MAX)
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "items": {
                    "type": "array",
                    "maxItems": ITEMS_MAX,
                    "items": {
                        "type": "object",
                        "properties": {
                            "name": { "type": "string" },
                            "host": { "type": "string" },
                            "port": { "type": "integer" },
                            "username": { "type": "string" },
                            "jump_name": { "type": "string" }
                        },
                        "required": ["name", "host", "port"]
                    },
                    "description": format!("要写入的设备字段数组，一次最多 {} 条", ITEMS_MAX)
                }
            },
            "required": ["items"]
        })
    }

    async fn call(&self, context: &mut ToolContext, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let run_context = context.require::<Arc<AiRunContext>>().map_err(|error| AiToolError::Execution(error.to_string()))?.clone();
        let Some(call_id) = args.internal_call_id.clone() else {
            return Err(AiToolError::Execution("internal_call_id of the tool call is missing".to_string()));
        };
        let items = Self::require_items(args.items)?;
        let confirm_args = ToolDeviceImportArgs { items: items.clone(), internal_call_id: None };
        self.tool_conflict_manager.confirm_before_execute(&run_context, &call_id, self.tool_id().as_str(), &confirm_args).await?;
        let result = self.device_import_service.apply(ReqDeviceImportApply { items }).await.map_err(|error| AiToolError::Execution(format!("{:#}", error)))?;
        self.ws_manager.broadcast_all(&RespEventEntityNotify::OnChanged(Device::notify_entity_type()).to_resp());
        let body = serde_json::to_value(result).map_err(|error| AiToolError::Execution(error.to_string()))?;
        let rendered = serde_json::to_string(&body).map_err(|error| AiToolError::Execution(error.to_string()))?;
        context.insert_result(vec![WrapItem::wrap_item(None, rendered)]);
        Ok(body)
    }
}
