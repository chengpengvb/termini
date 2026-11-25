use crate::util::id_factory::uuid;
use crate::model::resp::resp_agent::*;
use crate::util::time_util::now_millis;
use crate::model::ai_chat_item::AiChatItem;

pub enum RespEventAgent {
    OnOpen(RespAgentOpen),
    OnCancelled(RespAgentCancelled),
    OnConflict(RespAgentConflict),
    OnDone(RespAgentDone),
    OnError(RespAgentError),
    OnUser(RespAgentUser),
    OnText(RespAgentDelta),
    OnReasoning(RespAgentDelta),
    OnToolCall(RespAgentToolCall),
    OnToolCallDone(RespAgentToolCallDone),
    OnToolCallChunk(RespAgentToolCallChunk),
    OnTextFinal(RespAgentDelta),
    OnReasoningFinal(RespAgentDelta),
    OnConflictAnswered(RespAgentConflictAnswered),
    OnToolSupervise(RespAgentToolSupervise),
    OnShellDaemon(RespAgentShellDaemon),
    OnShellDaemonChunk(RespAgentShellDaemonChunk),
    OnStatus(RespAgentStatus),
    OnQuestion(RespAgentQuestion),
    OnSubagentToolCall(RespAgentSubagentToolCall),
    OnSubagentToolDone(RespAgentSubagentToolDone),
    OnSubagentOpen(RespAgentSubagentOpen),
    OnImage(RespAgentImage),
    OnQuestionAnswered(RespAgentQuestionAnswered),
}

impl RespEventAgent {
    pub const ON_OPEN: i16 = 60;
    pub const ON_CANCELLED: i16 = 61;
    pub const ON_CONFLICT: i16 = 62;
    pub const ON_DONE: i16 = 63;
    pub const ON_ERROR: i16 = 64;
    pub const ON_USER: i16 = 65;
    pub const ON_TEXT: i16 = 66;
    pub const ON_REASONING: i16 = 67;
    pub const ON_TOOL_CALL: i16 = 68;
    pub const ON_TOOL_CALL_DONE: i16 = 69;
    pub const ON_TOOL_CALL_CHUNK: i16 = 70;
    pub const ON_TEXT_FINAL: i16 = 71;
    pub const ON_REASONING_FINAL: i16 = 72;
    pub const ON_CONFLICT_ANSWERED: i16 = 73;
    pub const ON_TOOL_SUPERVISE: i16 = 74;
    pub const ON_SHELL_DAEMON: i16 = 75;
    pub const ON_STATUS: i16 = 76;
    pub const ON_QUESTION: i16 = 77;
    pub const ON_SUBAGENT_TOOL_CALL: i16 = 78;
    pub const ON_SUBAGENT_TOOL_DONE: i16 = 79;
    pub const ON_SUBAGENT_OPEN: i16 = 80;
    pub const ON_IMAGE: i16 = 81;
    pub const ON_QUESTION_ANSWERED: i16 = 82;
    pub const ON_SHELL_DAEMON_CHUNK: i16 = 83;

    pub fn should_persist(r#type: i16) -> bool {
        match r#type {
            Self::ON_IMAGE => true,
            Self::ON_SUBAGENT_OPEN => true,
            Self::ON_OPEN => false,
            Self::ON_CANCELLED => true,
            Self::ON_CONFLICT => true,
            Self::ON_DONE => true,
            Self::ON_ERROR => true,
            Self::ON_USER => true,
            Self::ON_TEXT => false,
            Self::ON_REASONING => false,
            Self::ON_TOOL_CALL => true,
            Self::ON_TOOL_CALL_DONE => true,
            Self::ON_TOOL_CALL_CHUNK => false,
            Self::ON_TEXT_FINAL => true,
            Self::ON_REASONING_FINAL => true,
            Self::ON_CONFLICT_ANSWERED => true,
            Self::ON_TOOL_SUPERVISE => true,
            Self::ON_SHELL_DAEMON => false,
            Self::ON_SHELL_DAEMON_CHUNK => false,
            Self::ON_STATUS => false,
            Self::ON_QUESTION => true,
            Self::ON_SUBAGENT_TOOL_CALL => true,
            Self::ON_SUBAGENT_TOOL_DONE => true,
            _ => false,
        }
    }

    pub fn to_type(&self) -> i16 {
        match self {
            RespEventAgent::OnOpen(_) => Self::ON_OPEN,
            RespEventAgent::OnCancelled(_) => Self::ON_CANCELLED,
            RespEventAgent::OnConflict(_) => Self::ON_CONFLICT,
            RespEventAgent::OnDone(_) => Self::ON_DONE,
            RespEventAgent::OnError(_) => Self::ON_ERROR,
            RespEventAgent::OnUser(_) => Self::ON_USER,
            RespEventAgent::OnText(_) => Self::ON_TEXT,
            RespEventAgent::OnReasoning(_) => Self::ON_REASONING,
            RespEventAgent::OnToolCall(_) => Self::ON_TOOL_CALL,
            RespEventAgent::OnToolCallDone(_) => Self::ON_TOOL_CALL_DONE,
            RespEventAgent::OnToolCallChunk(_) => Self::ON_TOOL_CALL_CHUNK,
            RespEventAgent::OnTextFinal(_) => Self::ON_TEXT_FINAL,
            RespEventAgent::OnReasoningFinal(_) => Self::ON_REASONING_FINAL,
            RespEventAgent::OnConflictAnswered(_) => Self::ON_CONFLICT_ANSWERED,
            RespEventAgent::OnToolSupervise(_) => Self::ON_TOOL_SUPERVISE,
            RespEventAgent::OnShellDaemon(_) => Self::ON_SHELL_DAEMON,
            RespEventAgent::OnShellDaemonChunk(_) => Self::ON_SHELL_DAEMON_CHUNK,
            RespEventAgent::OnStatus(_) => Self::ON_STATUS,
            RespEventAgent::OnQuestion(_) => Self::ON_QUESTION,
            RespEventAgent::OnSubagentToolCall(_) => Self::ON_SUBAGENT_TOOL_CALL,
            RespEventAgent::OnSubagentToolDone(_) => Self::ON_SUBAGENT_TOOL_DONE,
            RespEventAgent::OnSubagentOpen(_) => Self::ON_SUBAGENT_OPEN,
            RespEventAgent::OnImage(_) => Self::ON_IMAGE,
            RespEventAgent::OnQuestionAnswered(_) => Self::ON_QUESTION_ANSWERED,
        }
    }

    pub fn to_payload(&self) -> serde_json::Value {
        match self {
            RespEventAgent::OnOpen(p) => serde_json::to_value(p).unwrap(),
            RespEventAgent::OnCancelled(p) => serde_json::to_value(p).unwrap(),
            RespEventAgent::OnConflict(p) => serde_json::to_value(p).unwrap(),
            RespEventAgent::OnDone(p) => serde_json::to_value(p).unwrap(),
            RespEventAgent::OnError(p) => serde_json::to_value(p).unwrap(),
            RespEventAgent::OnUser(p) => serde_json::to_value(p).unwrap(),
            RespEventAgent::OnText(p) => serde_json::to_value(p).unwrap(),
            RespEventAgent::OnReasoning(p) => serde_json::to_value(p).unwrap(),
            RespEventAgent::OnToolCall(p) => serde_json::to_value(p).unwrap(),
            RespEventAgent::OnToolCallDone(p) => serde_json::to_value(p).unwrap(),
            RespEventAgent::OnToolCallChunk(p) => serde_json::to_value(p).unwrap(),
            RespEventAgent::OnTextFinal(p) => serde_json::to_value(p).unwrap(),
            RespEventAgent::OnReasoningFinal(p) => serde_json::to_value(p).unwrap(),
            RespEventAgent::OnConflictAnswered(p) => serde_json::to_value(p).unwrap(),
            RespEventAgent::OnToolSupervise(p) => serde_json::to_value(p).unwrap(),
            RespEventAgent::OnShellDaemon(p) => serde_json::to_value(p).unwrap(),
            RespEventAgent::OnShellDaemonChunk(p) => serde_json::to_value(p).unwrap(),
            RespEventAgent::OnStatus(p) => serde_json::to_value(p).unwrap(),
            RespEventAgent::OnQuestion(p) => serde_json::to_value(p).unwrap(),
            RespEventAgent::OnSubagentToolCall(p) => serde_json::to_value(p).unwrap(),
            RespEventAgent::OnSubagentToolDone(p) => serde_json::to_value(p).unwrap(),
            RespEventAgent::OnSubagentOpen(p) => serde_json::to_value(p).unwrap(),
            RespEventAgent::OnImage(p) => serde_json::to_value(p).unwrap(),
            RespEventAgent::OnQuestionAnswered(p) => serde_json::to_value(p).unwrap(),
        }
    }

    pub fn to_ai_chat_item(&self, chat_id: String, id: Option<String>, pid: Option<String>) -> AiChatItem {
        let r#type = self.to_type();
        let context = self.to_payload();
        let id = if let Some(id) = id { id } else { uuid() };
        AiChatItem { id, chat_id, r#type, timestamp: now_millis(), context, pid }
    }

    pub fn req_id(&self) -> Option<&str> {
        self.base().map(|base| base.req_id.as_str())
    }

    pub fn set_parent_req_id(&mut self, parent_req_id: String) {
        if let Some(base) = self.base_mut() {
            base.parent_req_id = Some(parent_req_id);
        }
    }

    fn base(&self) -> Option<&RespAgentBase> {
        match self {
            RespEventAgent::OnOpen(p) => Some(&p.base),
            RespEventAgent::OnCancelled(p) => Some(&p.base),
            RespEventAgent::OnConflict(p) => Some(&p.base),
            RespEventAgent::OnDone(p) => Some(&p.base),
            RespEventAgent::OnError(p) => Some(&p.base),
            RespEventAgent::OnUser(p) => Some(&p.base),
            RespEventAgent::OnText(p) => Some(&p.base),
            RespEventAgent::OnReasoning(p) => Some(&p.base),
            RespEventAgent::OnToolCall(p) => Some(&p.base),
            RespEventAgent::OnToolCallDone(p) => Some(&p.base),
            RespEventAgent::OnToolCallChunk(p) => Some(&p.base),
            RespEventAgent::OnTextFinal(p) => Some(&p.base),
            RespEventAgent::OnReasoningFinal(p) => Some(&p.base),
            RespEventAgent::OnConflictAnswered(p) => Some(&p.base),
            RespEventAgent::OnToolSupervise(p) => Some(&p.base),
            RespEventAgent::OnShellDaemon(_) => None,
            RespEventAgent::OnShellDaemonChunk(_) => None,
            RespEventAgent::OnStatus(p) => Some(&p.base),
            RespEventAgent::OnQuestion(p) => Some(&p.base),
            RespEventAgent::OnSubagentToolCall(p) => Some(&p.base),
            RespEventAgent::OnSubagentToolDone(p) => Some(&p.base),
            RespEventAgent::OnSubagentOpen(p) => Some(&p.base),
            RespEventAgent::OnImage(p) => Some(&p.base),
            RespEventAgent::OnQuestionAnswered(_) => None,
        }
    }

    fn base_mut(&mut self) -> Option<&mut RespAgentBase> {
        match self {
            RespEventAgent::OnOpen(p) => Some(&mut p.base),
            RespEventAgent::OnCancelled(p) => Some(&mut p.base),
            RespEventAgent::OnConflict(p) => Some(&mut p.base),
            RespEventAgent::OnDone(p) => Some(&mut p.base),
            RespEventAgent::OnError(p) => Some(&mut p.base),
            RespEventAgent::OnUser(p) => Some(&mut p.base),
            RespEventAgent::OnText(p) => Some(&mut p.base),
            RespEventAgent::OnReasoning(p) => Some(&mut p.base),
            RespEventAgent::OnToolCall(p) => Some(&mut p.base),
            RespEventAgent::OnToolCallDone(p) => Some(&mut p.base),
            RespEventAgent::OnToolCallChunk(p) => Some(&mut p.base),
            RespEventAgent::OnTextFinal(p) => Some(&mut p.base),
            RespEventAgent::OnReasoningFinal(p) => Some(&mut p.base),
            RespEventAgent::OnConflictAnswered(p) => Some(&mut p.base),
            RespEventAgent::OnToolSupervise(p) => Some(&mut p.base),
            RespEventAgent::OnShellDaemon(_) => None,
            RespEventAgent::OnShellDaemonChunk(_) => None,
            RespEventAgent::OnStatus(p) => Some(&mut p.base),
            RespEventAgent::OnQuestion(p) => Some(&mut p.base),
            RespEventAgent::OnSubagentToolCall(p) => Some(&mut p.base),
            RespEventAgent::OnSubagentToolDone(p) => Some(&mut p.base),
            RespEventAgent::OnSubagentOpen(p) => Some(&mut p.base),
            RespEventAgent::OnImage(p) => Some(&mut p.base),
            RespEventAgent::OnQuestionAnswered(_) => None,
        }
    }
}
