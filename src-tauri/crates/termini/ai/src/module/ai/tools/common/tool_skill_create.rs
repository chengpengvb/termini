use anyhow::Result;
use std::sync::Arc;
use serde_json::json;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use crate::base::base_app::BaseApp;
use crate::model::ai_skill::AiSkill;
use serde::{Deserialize, Serialize};
use crate::module::ai::dto::AiToolId;
use crate::module::ai::dto::AiRunContext;
use crate::module::ai::shared::ToolCallDonePush;
use rig::tool::{DynamicTool, Tool, ToolContext};
use crate::module::ai::tools::tool_base::ToolBase;
use crate::module::ai::shared::ToolConflictManager;
use crate::service::service_ai_skill::AiSkillService;
use crate::module::ai::tools::tool_manager::ToolManager;
use crate::module::ai::tools::ai_tool_error::AiToolError;
use crate::module::ai::tools::base::{ToolBaseOutWrap, WrapItem};

const DEFAULT_SORT: i16 = 100;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ToolSkillCreateArgs {
    pub name: String,
    pub content: String,

    #[serde(default)]
    pub description: String,

    #[serde(default, rename = "__internal_call_id")]
    pub internal_call_id: Option<String>,
}

#[base_app(sort = 772)]
#[derive(Clone, Service)]
pub struct ToolSkillCreate {
    #[inject(component)]
    tool_manager: ToolManager,

    #[inject(component)]
    ai_skill_service: AiSkillService,

    #[inject(component)]
    tool_conflict_manager: ToolConflictManager,
}

#[async_trait]
impl BaseApp for ToolSkillCreate {
    fn name(&self) -> &'static str {
        "ToolSkillCreate"
    }

    async fn start(&self, _ctx: &crate::base::base_app::AppContext) -> Result<()> {
        self.tool_manager.register(Arc::new(self.clone()));
        Ok(())
    }
}

impl ToolBase for ToolSkillCreate {
    fn tool_id(&self) -> AiToolId {
        AiToolId::ToolSkillCreate
    }

    fn deferred(&self) -> bool {
        true
    }

    fn as_rig_tool(&self, done_push: Option<ToolCallDonePush>) -> DynamicTool {
        ToolBaseOutWrap::wrap(self.clone(), self.max_result_tokens(), done_push)
    }
}

impl Tool for ToolSkillCreate {
    const NAME: &'static str = AiToolId::ToolSkillCreate.as_str();

    type Args = ToolSkillCreateArgs;
    type Output = String;
    type Error = AiToolError;

    fn description(&self) -> String {
        "把一段可复用的操作流程保存为技能，供以后按 @[skill:id] 引用或用户点名时加载执行。name 技能名，content 完整步骤与命令正文，description 一句话说明何时使用。仅在用户明确要求把当前做法固化成技能、或要新增技能时才用，不要擅自创建。执行前会请求用户确认。".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "技能名，语言遵循 [输出语言]"
                },
                "content": {
                    "type": "string",
                    "description": "技能正文，语言遵循 [输出语言]：只写可复跑的步骤、命令与必要的预期结果；不写背景叙事、根因分析过程和与执行无关的细节，设备的长期事实（路径、端口、角色）归设备档案不归技能"
                },
                "description": {
                    "type": "string",
                    "description": "一句话说明何时使用该技能，语言遵循 [输出语言]"
                }
            },
            "required": ["name", "content"]
        })
    }

    async fn call(&self, context: &mut ToolContext, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let run_context = context.require::<Arc<AiRunContext>>().map_err(|error| AiToolError::Execution(error.to_string()))?.clone();
        let Some(call_id) = args.internal_call_id.clone() else {
            return Err(AiToolError::Execution("internal_call_id of the tool call is missing".to_string()));
        };
        let confirm_args = ToolSkillCreateArgs { name: args.name.clone(), content: args.content.clone(), description: args.description.clone(), internal_call_id: None };
        self.tool_conflict_manager.confirm_before_execute(&run_context, &call_id, self.tool_id().as_str(), &confirm_args).await?;
        let skill = AiSkill { id: String::new(), sort: DEFAULT_SORT, name: args.name.clone(), content: args.content, description: args.description };
        self.ai_skill_service.create_skill(skill).await.map_err(|error| AiToolError::Execution(error.to_string()))?;
        let body = format!("已创建技能：{}", args.name);
        context.insert_result(vec![WrapItem::wrap_item(None, body.clone())]);
        Ok(body)
    }
}
