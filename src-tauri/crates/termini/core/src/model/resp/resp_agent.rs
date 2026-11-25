use serde_json::Value;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RespAgentBase {
    pub req_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_req_id: Option<String>,
}

impl RespAgentBase {
    pub fn new(req_id: String) -> Self {
        Self { req_id, parent_req_id: None }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RespAgentRunBase {
    pub run_id: String, //单次run_stream的id
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RespAgentOpen {
    #[serde(flatten)]
    pub base: RespAgentBase,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RespAgentStatus {
    #[serde(flatten)]
    pub base: RespAgentBase,
    pub stage: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attempt: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_attempts: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tokens_before: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tokens_after: Option<usize>,
}

impl RespAgentStatus {
    pub fn stage(req_id: &str, stage: &str) -> Self {
        Self { base: RespAgentBase::new(req_id.to_string()), stage: stage.to_string(), attempt: None, max_attempts: None, tokens_before: None, tokens_after: None }
    }

    pub fn retrying(req_id: &str, attempt: usize, max_attempts: usize) -> Self {
        Self { attempt: Some(attempt), max_attempts: Some(max_attempts), ..Self::stage(req_id, "retrying") }
    }

    pub fn memory_trimmed(req_id: &str, tokens_before: usize, tokens_after: usize) -> Self {
        Self { tokens_before: Some(tokens_before), tokens_after: Some(tokens_after), ..Self::stage(req_id, "memoryTrimmed") }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RespAgentDone {
    #[serde(flatten)]
    pub base: RespAgentBase,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RespAgentCancelled {
    #[serde(flatten)]
    pub base: RespAgentBase,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RespAgentError {
    #[serde(flatten)]
    pub base: RespAgentBase,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<i32>,
    #[serde(default)]
    pub error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RespAgentUser {
    #[serde(flatten)]
    pub base: RespAgentBase,
    pub message: String,
    #[serde(default)]
    pub origin: i16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RespAgentDelta {
    #[serde(flatten)]
    pub base: RespAgentBase,

    #[serde(flatten)]
    pub run: RespAgentRunBase,

    pub delta: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RespAgentImage {
    #[serde(flatten)]
    pub base: RespAgentBase,

    #[serde(flatten)]
    pub run: RespAgentRunBase,

    pub path: String,
    pub media_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RespAgentImageData {
    pub data: String,
    pub media_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RespAgentToolCall {
    #[serde(flatten)]
    pub base: RespAgentBase,

    #[serde(flatten)]
    pub run: RespAgentRunBase,

    pub args: Value,
    pub call_id: String,
    pub tool_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RespAgentToolCallDone {
    pub call_id: String,

    pub tool_id: String,

    pub result: String,

    #[serde(default = "default_tool_call_status")]
    pub status: i16,

    #[serde(flatten)]
    pub base: RespAgentBase,

    #[serde(flatten)]
    pub run: RespAgentRunBase,
}

pub fn default_tool_call_status() -> i16 {
    RespAgentToolStatus::Done as i16
}

#[repr(i16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RespAgentToolStatus {
    Running = 0,
    Detached = 1,
    Done = 2,
    Killed = 3,
    Failed = 4,
    Cancelled = 5,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RespAgentSubagentToolCall {
    pub args: Value,

    pub tool_id: String,

    pub call_id: String,

    pub sub_call_id: String,

    #[serde(flatten)]
    pub base: RespAgentBase,

    #[serde(flatten)]
    pub run: RespAgentRunBase,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RespAgentSubagentOpen {
    #[serde(flatten)]
    pub base: RespAgentBase,

    #[serde(flatten)]
    pub run: RespAgentRunBase,

    pub call_id: String,

    pub sub_req_id: String,

    pub title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RespAgentSubagentToolDone {
    pub ok: bool,

    pub call_id: String,

    pub result: String,

    pub tool_id: String,

    pub elapsed_ms: i64,

    pub sub_call_id: String,

    #[serde(flatten)]
    pub base: RespAgentBase,

    #[serde(flatten)]
    pub run: RespAgentRunBase,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RespAgentToolCallChunk {
    pub delta: String,

    pub call_id: String,

    pub tool_id: String,

    #[serde(flatten)]
    pub base: RespAgentBase,

    #[serde(flatten)]
    pub run: RespAgentRunBase,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RespAgentToolSupervise {
    pub action: i16,

    pub round: usize,

    pub reason: String,

    pub call_id: String,

    pub tool_id: String,

    pub wait_seconds: u64,

    #[serde(flatten)]
    pub base: RespAgentBase,

    #[serde(flatten)]
    pub run: RespAgentRunBase,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RespAgentShellDaemonRow {
    pub tty: bool,

    pub output: String,

    pub command: String,

    pub started_ms: i64,

    pub pid: Option<u32>,

    pub cwd: Option<String>,

    pub process_id: String,

    pub shell_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RespAgentShellDaemon {
    pub chat_id: String,

    pub daemons: Vec<RespAgentShellDaemonRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RespAgentShellDaemonChunk {
    pub delta: String,

    pub chat_id: String,

    pub process_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RespAgentConflict {
    pub args: Value,

    pub call_id: String,

    pub tool_id: String,

    pub auto_allowed: bool,

    #[serde(flatten)]
    pub base: RespAgentBase,

    #[serde(flatten)]
    pub run: RespAgentRunBase,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub audit_reason: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_prefix: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RespAgentConflictAnswered {
    pub code: i16,

    pub call_id: String,

    pub tool_id: String,

    #[serde(flatten)]
    pub base: RespAgentBase,

    #[serde(flatten)]
    pub run: RespAgentRunBase,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RespAgentQuestion {
    pub call_id: String,

    pub tool_id: String,

    pub questions: Value,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub answers: Option<Value>,

    #[serde(flatten)]
    pub base: RespAgentBase,

    #[serde(flatten)]
    pub run: RespAgentRunBase,
}

impl RespAgentQuestion {
    pub const STATE_ANSWERED: &'static str = "answered";
    pub const STATE_SKIPPED: &'static str = "skipped";
    pub const STATE_DECLINED: &'static str = "declined";

    pub fn answered(&self) -> Option<RespAgentQuestionAnswered> {
        let state = self.state.clone()?;
        Some(RespAgentQuestionAnswered { call_id: self.call_id.clone(), state, answers: self.answers.clone() })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RespAgentQuestionAnswered {
    pub call_id: String,

    pub state: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub answers: Option<Value>,
}
