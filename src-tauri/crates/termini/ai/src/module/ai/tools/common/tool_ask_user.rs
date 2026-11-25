use std::sync::Arc;
use summer::tracing::warn;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use serde_json::{json, Value};
use crate::base::base_app::BaseApp;
use serde::{Deserialize, Serialize};
use crate::module::ai::dto::AiToolId;
use crate::module::ai::dto::AiRunContext;
use crate::module::ai::shared::ToolCallDonePush;
use crate::service::service_agent::AgentService;
use rig::tool::{DynamicTool, Tool, ToolContext};
use crate::module::ai::shared::ChatSessionCacheManager;
use crate::module::ai::tools::tool_manager::ToolManager;
use crate::module::ai::tools::ai_tool_error::AiToolError;
use crate::model::req::event::ws_agent::AgentQuestionAnswerItem;
use crate::model::resp::event::resp_event_agent::RespEventAgent;
use crate::module::ai::tools::base::{ToolBaseOutWrap, WrapItem};
use crate::module::ai::tools::tool_base::{ToolBase, ToolDomain};
use crate::model::resp::resp_agent::{RespAgentBase, RespAgentQuestion, RespAgentRunBase};

const QUESTIONS_MAX: usize = 5;
const ANSWER_HEADER: &str = "[用户回答] 以下是用户对 ask_user 提问的作答，按此继续：";
const ASKED_RESULT: &str = "已向用户展示问题，本轮到此结束。用户的回答会以新消息发来；若用户直接发送了其他内容，视为不作答，按新内容继续。";

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AskUserOption {
    pub id: String,
    pub label: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AskUserQuestion {
    pub id: String,
    pub prompt: String,
    #[serde(default)]
    pub options: Vec<AskUserOption>,
    #[serde(default)]
    pub allow_multiple: bool,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ToolAskUserArgs {
    pub questions: Vec<AskUserQuestion>,

    #[serde(default, rename = "__internal_call_id")]
    pub internal_call_id: Option<String>,
}

#[base_app(sort = 766)]
#[derive(Clone, Service)]
pub struct ToolAskUser {
    #[inject(component)]
    tool_manager: ToolManager,

    #[inject(component)]
    agent_service: AgentService,

    #[inject(component)]
    tool_call_done_push: ToolCallDonePush,

    #[inject(component)]
    chat_session_cache_manager: ChatSessionCacheManager,
}

#[async_trait]
impl BaseApp for ToolAskUser {
    fn name(&self) -> &'static str {
        "ToolAskUser"
    }

    async fn start(&self, _ctx: &crate::base::base_app::AppContext) -> anyhow::Result<()> {
        self.tool_manager.register(Arc::new(self.clone()));
        Ok(())
    }
}

impl ToolBase for ToolAskUser {
    fn tool_id(&self) -> AiToolId {
        AiToolId::ToolAskUser
    }

    fn domains(&self) -> &'static [ToolDomain] {
        &[ToolDomain::Main]
    }

    fn as_rig_tool(&self, done_push: Option<ToolCallDonePush>) -> DynamicTool {
        ToolBaseOutWrap::wrap(self.clone(), self.max_result_tokens(), done_push)
    }
}

impl ToolAskUser {
    pub fn render_answer(questions: &Value, answers: &[AgentQuestionAnswerItem]) -> anyhow::Result<String> {
        let questions: Vec<AskUserQuestion> = serde_json::from_value(questions.clone())?;
        let mut lines = vec![ANSWER_HEADER.to_string()];
        for question in &questions {
            let Some(answer) = answers.iter().find(|answer| answer.question_id == question.id) else {
                return Err(anyhow::anyhow!("question {} has no answer", question.id));
            };
            let mut parts: Vec<String> = question.options.iter().filter(|option| answer.option_ids.contains(&option.id)).map(|option| option.label.clone()).collect();
            if let Some(text) = answer.text.as_deref().map(str::trim).filter(|text| !text.is_empty()) {
                parts.push(text.to_string());
            }
            if parts.is_empty() {
                return Err(anyhow::anyhow!("question {} answer is empty", question.id));
            }
            lines.push(format!("{}：{}", question.prompt, parts.join("、")));
        }
        Ok(lines.join("\n"))
    }

    fn push_done(&self, run_context: &AiRunContext, call_id: &str, result: String) -> String {
        self.tool_call_done_push.push_tool_call_done_event(run_context, call_id, result.clone());
        result
    }

    fn emit_question(&self, context: &AiRunContext, call_id: &str, questions: serde_json::Value) {
        let Some(entry) = self.chat_session_cache_manager.get(&context.req_id) else {
            warn!("[ai] 推送提问事件失败：未知 req_id={}", context.req_id);
            return;
        };
        let ai_chat_id = entry.context.ai_chat_id.clone();
        drop(entry);
        let pid = self.chat_session_cache_manager.get_tool_item_id(&context.req_id, call_id);
        let event = RespEventAgent::OnQuestion(RespAgentQuestion {
            questions,
            state: None,
            answers: None,
            call_id: call_id.to_string(),
            tool_id: self.tool_id().as_str().to_string(),
            base: RespAgentBase::new(context.req_id.clone()),
            run: RespAgentRunBase { run_id: context.run_id.clone() },
        });
        self.agent_service.handler_agent_event(event, ai_chat_id, None, pid, None);
    }

    fn validate(args: &ToolAskUserArgs) -> Result<(), AiToolError> {
        if args.questions.is_empty() {
            return Err(AiToolError::Execution("questions is empty".to_string()));
        }
        if args.questions.len() > QUESTIONS_MAX {
            return Err(AiToolError::Execution(format!("questions allows at most {} entries", QUESTIONS_MAX)));
        }
        for question in &args.questions {
            if question.id.trim().is_empty() {
                return Err(AiToolError::Execution("question id is empty".to_string()));
            }
            if question.prompt.trim().is_empty() {
                return Err(AiToolError::Execution("question prompt is empty".to_string()));
            }
        }
        Ok(())
    }
}

impl Tool for ToolAskUser {
    const NAME: &'static str = AiToolId::ToolAskUser.as_str();

    type Error = AiToolError;
    type Args = ToolAskUserArgs;
    type Output = String;

    fn description(&self) -> String {
        "向用户提问。当缺少只有用户才能提供的关键信息、或需要用户在多个可行方案中做决定时使用；能自行查证的问题不要问。调用后本轮立即结束，用户的回答会以新消息发来。每个问题给出候选项，把推荐项放第一个；用户可以单选/多选，也可以手动补充说明或跳过。".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "questions": {
                    "type": "array",
                    "description": "问题列表，最多 5 个",
                    "items": {
                        "type": "object",
                        "properties": {
                            "id": {
                                "type": "string",
                                "description": "问题唯一标识，如 q1"
                            },
                            "prompt": {
                                "type": "string",
                                "description": "问题内容，语言遵循 [输出语言]"
                            },
                            "options": {
                                "type": "array",
                                "description": "候选项，至少 2 个；推荐项放第一个并在文本末尾标注（推荐）；每项 label 语言遵循 [输出语言]",
                                "items": {
                                    "type": "object",
                                    "properties": {
                                        "id": {
                                            "type": "string",
                                            "description": "选项唯一标识，如 a"
                                        },
                                        "label": {
                                            "type": "string",
                                            "description": "选项展示文本，语言遵循 [输出语言]"
                                        }
                                    },
                                    "required": ["id", "label"]
                                }
                            },
                            "allow_multiple": {
                                "type": "boolean",
                                "description": "是否允许多选，默认 false"
                            }
                        },
                        "required": ["id", "prompt", "options"]
                    }
                }
            },
            "required": ["questions"]
        })
    }

    async fn call(&self, context: &mut ToolContext, args: Self::Args) -> anyhow::Result<Self::Output, Self::Error> {
        let run_context = context.require::<Arc<AiRunContext>>().map_err(|error| AiToolError::Execution(error.to_string()))?.clone();
        let Some(call_id) = args.internal_call_id.clone() else {
            return Err(AiToolError::Execution("internal_call_id of the tool call is missing".to_string()));
        };
        Self::validate(&args)?;
        let questions = serde_json::to_value(&args.questions).map_err(|error| AiToolError::Execution(error.to_string()))?;
        self.emit_question(&run_context, &call_id, questions);
        run_context.finish_token.cancel();
        let body = self.push_done(&run_context, &call_id, ASKED_RESULT.to_string());
        context.insert_result(vec![WrapItem::wrap_item(None, body.clone())]);
        Ok(body)
    }
}
