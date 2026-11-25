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
use crate::module::ai::shared::ToolConflictManager;
use crate::service::service_ai_skill::AiSkillService;
use crate::module::ai::tools::tool_manager::ToolManager;
use crate::module::ai::tools::ai_tool_error::AiToolError;
use crate::module::ai::tools::base::{ToolBaseOutWrap, WrapItem};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ToolSkillUpdateArgs {
    pub id: String,
    pub name: String,
    pub content: String,

    #[serde(default)]
    pub description: String,

    #[serde(default, rename = "__internal_call_id")]
    pub internal_call_id: Option<String>,
}

#[base_app(sort = 773)]
#[derive(Clone, Service)]
pub struct ToolSkillUpdate {
    #[inject(component)]
    tool_manager: ToolManager,

    #[inject(component)]
    ai_skill_service: AiSkillService,

    #[inject(component)]
    tool_conflict_manager: ToolConflictManager,
}

#[async_trait]
impl BaseApp for ToolSkillUpdate {
    fn name(&self) -> &'static str {
        "ToolSkillUpdate"
    }

    async fn start(&self, _ctx: &crate::base::base_app::AppContext) -> Result<()> {
        self.tool_manager.register(Arc::new(self.clone()));
        Ok(())
    }
}

impl ToolBase for ToolSkillUpdate {
    fn tool_id(&self) -> AiToolId {
        AiToolId::ToolSkillUpdate
    }

    fn deferred(&self) -> bool {
        true
    }

    fn as_rig_tool(&self, done_push: Option<ToolCallDonePush>) -> DynamicTool {
        ToolBaseOutWrap::wrap(self.clone(), self.max_result_tokens(), done_push)
    }
}

impl Tool for ToolSkillUpdate {
    const NAME: &'static str = AiToolId::ToolSkillUpdate.as_str();

    type Args = ToolSkillUpdateArgs;
    type Output = String;
    type Error = AiToolError;

    fn description(&self) -> String {
        "修改一条已保存的技能。先用 select_all_skill 查到目标技能的 id，再传 id 与新的 name / content / description 覆盖。两种情形使用：用户要求修改技能；或执行中证实技能正文有错（命令报错、路径不存在、步骤跑不通），此时先按实际可行的做法完成任务，再把修正写回技能并向用户说明，不要只绕过错误——否则下次会话还会按错的正文重走弯路。执行前会请求用户确认。".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "id": {
                    "type": "string",
                    "description": "要修改的技能 id（用 select_all_skill 查到）"
                },
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
            "required": ["id", "name", "content"]
        })
    }

    async fn call(&self, context: &mut ToolContext, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let run_context = context.require::<Arc<AiRunContext>>().map_err(|error| AiToolError::Execution(error.to_string()))?.clone();
        let Some(call_id) = args.internal_call_id.clone() else {
            return Err(AiToolError::Execution("internal_call_id of the tool call is missing".to_string()));
        };
        let confirm_args = ToolSkillUpdateArgs { id: args.id.clone(), name: args.name.clone(), content: args.content.clone(), description: args.description.clone(), internal_call_id: None };
        self.tool_conflict_manager.confirm_before_execute(&run_context, &call_id, self.tool_id().as_str(), &confirm_args).await?;
        let existing = self.ai_skill_service.find_by_id(args.id.clone()).await.map_err(|error| AiToolError::Execution(error.to_string()))?;
        let Some(shared) = existing else {
            return Err(AiToolError::Execution(format!("skill not found: {}", args.id)));
        };
        let mut skill = shared.read().clone();
        skill.name = args.name.clone();
        skill.content = args.content;
        skill.description = args.description;
        self.ai_skill_service.update_skill(skill).await.map_err(|error| AiToolError::Execution(error.to_string()))?;
        let body = format!("已更新技能：{}", args.name);
        context.insert_result(vec![WrapItem::wrap_item(None, body.clone())]);
        Ok(body)
    }
}
