use anyhow::Result;
use std::sync::Arc;
use serde_json::json;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use crate::base::base_app::BaseApp;
use serde::{Deserialize, Serialize};
use crate::module::ai::dto::AiToolId;
use crate::web::ws::ws_manager::WsManager;
use crate::module::ai::shared::ToolCallDonePush;
use rig::tool::{DynamicTool, Tool, ToolContext};
use crate::module::ai::tools::tool_base::ToolBase;
use crate::module::ai::tools::tool_manager::ToolManager;
use crate::module::ai::tools::ai_tool_error::AiToolError;
use crate::module::ai::tools::base::{ToolBaseOutWrap, WrapItem};
use crate::model::resp::event::resp_event_entity_notify::RespEventEntityNotify;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ToolEntityNotifyArgs {
    pub entity_type: i32,

    #[serde(default, rename = "__internal_call_id")]
    pub internal_call_id: Option<String>,
}

#[base_app(sort = 777)]
#[derive(Clone, Service)]
pub struct ToolEntityNotify {
    #[inject(component)]
    tool_manager: ToolManager,

    #[inject(component)]
    ws_manager: WsManager,
}

#[async_trait]
impl BaseApp for ToolEntityNotify {
    fn name(&self) -> &'static str {
        "ToolEntityNotify"
    }

    async fn start(&self, _ctx: &crate::base::base_app::AppContext) -> Result<()> {
        self.tool_manager.register(Arc::new(self.clone()));
        Ok(())
    }
}

impl ToolBase for ToolEntityNotify {
    fn tool_id(&self) -> AiToolId {
        AiToolId::ToolEntityNotify
    }

    fn as_rig_tool(&self, done_push: Option<ToolCallDonePush>) -> DynamicTool {
        ToolBaseOutWrap::wrap(self.clone(), self.max_result_tokens(), done_push)
    }
}

impl Tool for ToolEntityNotify {
    const NAME: &'static str = AiToolId::ToolEntityNotify.as_str();

    type Error = AiToolError;
    type Args = ToolEntityNotifyArgs;
    type Output = serde_json::Value;

    fn description(&self) -> String {
        "不写库，只通知前端按传入的 entity_type 数字刷新对应列表。entity_call 等 CRUD 写库成功后前端仍未刷新时调用，或写完后主动配套调用；传入对应 entity_type 数字。".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "entity_type": {
                    "type": "integer",
                    "description": "要刷新的实体类型数字，与 BaseTarget.entity_type 一致；须与刚写库的表对应"
                }
            },
            "required": ["entity_type"]
        })
    }

    async fn call(&self, context: &mut ToolContext, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let Some(_call_id) = args.internal_call_id.clone() else {
            return Err(AiToolError::Execution("internal_call_id of the tool call is missing".to_string()));
        };
        self.ws_manager.broadcast_all(&RespEventEntityNotify::OnChanged(args.entity_type).to_resp());
        let body = json!({ "notified": args.entity_type });
        let rendered = serde_json::to_string(&body).map_err(|error| AiToolError::Execution(error.to_string()))?;
        context.insert_result(vec![WrapItem::wrap_item(None, rendered)]);
        Ok(body)
    }
}
