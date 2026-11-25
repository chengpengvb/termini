use std::sync::Arc;
use tokio::sync::Notify;
use std::collections::VecDeque;
use tokio::sync::Mutex as AsyncMutex;
use crate::module::ai::shared::ConflictSubject;
use crate::model::req::event::ws_agent::AgentAnswerCode;

pub struct ToolConflictAnswerEntry {
    pub call_id: String,
    pub code: AgentAnswerCode,
}

pub struct ToolConflictInbox {
    pub notify: Notify,
    pub queue: AsyncMutex<VecDeque<ToolConflictAnswerEntry>>,
}

impl ToolConflictInbox {
    pub fn new() -> Self {
        Self { notify: Notify::new(), queue: AsyncMutex::new(VecDeque::new()) }
    }
}

pub struct ToolConflictPending {
    pub tool_id: String,
    pub session_key: String,
    pub chat_id: Option<String>,
    pub subject: ConflictSubject,
    pub command_tokens: Vec<String>,
    pub inbox: Arc<ToolConflictInbox>,
}

impl ToolConflictPending {
    pub fn match_key(&self) -> String {
        self.subject.match_key(&self.tool_id)
    }

    pub async fn push_answer(&self, call_id: &str, code: AgentAnswerCode) {
        self.inbox.queue.lock().await.push_back(ToolConflictAnswerEntry { call_id: call_id.to_string(), code });
        self.inbox.notify.notify_waiters();
    }
}

pub enum AuditOutcome {
    Allow(Option<String>),
    Escalate(String),
}
