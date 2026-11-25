use serde::{Deserialize, Serialize};
use crate::model::req::event::ws_base::WsBase;
use crate::model::req::req_chat_context::ReqChatContextPayload;

pub type CancelEvent = WsBase<AgentCancelData>;
pub type ToolAnswerEvent = WsBase<AgentAnswerData>;
pub type ChatEvent = WsBase<ReqChatContextPayload>;
pub type SubscribeEvent = WsBase<AgentSubscribeData>;
pub type ToolCancelEvent = WsBase<AgentToolCancelData>;
pub type ConfirmModeEvent = WsBase<AgentConfirmModeData>;
pub type QuestionAnswerEvent = WsBase<AgentQuestionAnswerData>;

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub enum AgentAnswerCode {
    Allow = 1,
    Reject = 2,
    AllowRemember = 3,      //允许并记住，之后同设备同工具的同一条命令不再审核
    AllowSessionPrefix = 4, //允许, 且本会话内同工具同设备、命令前缀相同(如 git status …)的不再问; 只在内存里, 会话结束即失效
}

impl AgentAnswerCode {
    pub fn from_i16(value: i16) -> Option<Self> {
        match value {
            1 => Some(AgentAnswerCode::Allow),
            2 => Some(AgentAnswerCode::Reject),
            3 => Some(AgentAnswerCode::AllowRemember),
            4 => Some(AgentAnswerCode::AllowSessionPrefix),
            _ => None,
        }
    }

    pub fn is_allow(self) -> bool {
        matches!(self, AgentAnswerCode::Allow | AgentAnswerCode::AllowRemember | AgentAnswerCode::AllowSessionPrefix)
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentCancelData {
    pub req_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentAnswerData {
    pub code: i16,
    pub call_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentToolCancelData {
    pub req_id: String,
    pub run_id: String,
    pub call_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentSubscribeData {
    pub ai_chat_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentConfirmModeData {
    pub ai_chat_id: String,
    pub confirm_mode: i8,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentQuestionAnswerItem {
    pub question_id: String,
    #[serde(default)]
    pub option_ids: Vec<String>,
    #[serde(default)]
    pub text: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentQuestionAnswerData {
    pub ai_chat_id: String,
    pub ai_provide_id: String,
    pub call_id: String,
    pub skipped: bool,
    #[serde(default)]
    pub confirm_mode: i8,
    #[serde(default)]
    pub thinking_level: Option<String>,
    #[serde(default)]
    pub answers: Vec<AgentQuestionAnswerItem>,
}
