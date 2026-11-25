use std::sync::Arc;
use dashmap::DashMap;
use parking_lot::Mutex;
use tokio::sync::Notify;
use summer::tracing::info;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use crate::module::ai::dto::AiToolId;
use crate::util::time_util::now_millis;
use tokio_util::sync::CancellationToken;
use crate::base::base_app::{AppContext, BaseApp};
use crate::model::resp::resp_agent::RespAgentToolStatus;

const PROGRESS_MAX_CHARS: usize = 64 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    Running,
    Done,
    Failed,
    Cancelled,
}

impl TaskState {
    pub fn as_str(self) -> &'static str {
        match self {
            TaskState::Running => "running",
            TaskState::Done => "done",
            TaskState::Failed => "failed",
            TaskState::Cancelled => "cancelled",
        }
    }

    pub fn is_finished(self) -> bool {
        !matches!(self, TaskState::Running)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskKind {
    Subagent,
    ImageRead,
    ImageGenerate,
    PdfRead,
}

impl TaskKind {
    pub fn label(self) -> &'static str {
        match self {
            TaskKind::Subagent => "子代理",
            TaskKind::ImageRead => "图片识别",
            TaskKind::ImageGenerate => "图片生成",
            TaskKind::PdfRead => "PDF 版面识别",
        }
    }

    pub fn tool_id(self) -> AiToolId {
        match self {
            TaskKind::Subagent => AiToolId::ToolTask,
            TaskKind::ImageRead => AiToolId::ToolImageRead,
            TaskKind::ImageGenerate => AiToolId::ToolImageGenerate,
            TaskKind::PdfRead => AiToolId::ToolPdfRead,
        }
    }
}

#[derive(Clone)]
pub struct TaskEntry {
    pub kind: TaskKind,
    pub prompt: String,
    pub chat_id: String,
    pub task_id: String,
    pub call_id: String,
    pub started_ms: i64,
    pub parent_req_id: String,
    pub finished: Arc<Notify>,
    pub cancel: CancellationToken,
    pub detached: Arc<Mutex<bool>>,
    pub state: Arc<Mutex<TaskState>>,
    pub progress: Arc<Mutex<String>>,
    pub result: Arc<Mutex<Option<String>>>,
    pub last_at_ms: Arc<Mutex<Option<i64>>>,
}

impl TaskEntry {
    pub fn new(kind: TaskKind, parent_req_id: String, chat_id: String, task_id: String, call_id: String, prompt: String, cancel: CancellationToken) -> Self {
        Self {
            kind,
            chat_id,
            task_id,
            call_id,
            prompt,
            cancel,
            parent_req_id,
            started_ms: now_millis(),
            finished: Arc::new(Notify::new()),
            result: Arc::new(Mutex::new(None)),
            detached: Arc::new(Mutex::new(false)),
            last_at_ms: Arc::new(Mutex::new(None)),
            progress: Arc::new(Mutex::new(String::new())),
            state: Arc::new(Mutex::new(TaskState::Running)),
        }
    }

    pub fn state(&self) -> TaskState {
        *self.state.lock()
    }

    pub fn is_detached(&self) -> bool {
        *self.detached.lock()
    }

    pub fn mark_detached(&self) {
        *self.detached.lock() = true;
    }

    pub fn elapsed_secs(&self) -> i64 {
        now_millis().saturating_sub(self.started_ms) / 1000
    }

    pub fn silence_secs(&self) -> i64 {
        let last = self.last_at_ms.lock().unwrap_or(self.started_ms);
        now_millis().saturating_sub(last) / 1000
    }

    pub fn append_progress(&self, line: &str) {
        let mut buffer = self.progress.lock();
        buffer.push_str(line);
        if buffer.chars().count() > PROGRESS_MAX_CHARS {
            let keep: String = buffer.chars().skip(buffer.chars().count() - PROGRESS_MAX_CHARS).collect();
            *buffer = keep;
        }
        drop(buffer);
        *self.last_at_ms.lock() = Some(now_millis());
    }

    pub fn finish(&self, state: TaskState, result: Option<String>) {
        *self.state.lock() = state;
        if let Some(text) = result {
            *self.result.lock() = Some(text);
        }
        *self.last_at_ms.lock() = Some(now_millis());
        self.finished.notify_waiters();
    }

    pub fn progress_lines(&self) -> usize {
        self.progress.lock().lines().count()
    }

    pub fn summary_text(&self) -> String {
        format!(
            "task_id={} 类型={} 状态={} 已运行={}s 静默={}s 进度行数={} 任务={}",
            self.task_id,
            self.kind.label(),
            self.state().as_str(),
            self.elapsed_secs(),
            self.silence_secs(),
            self.progress_lines(),
            first_line(&self.prompt, 80)
        )
    }

    pub fn event_status(&self) -> i16 {
        if self.is_detached() && !self.state().is_finished() {
            return RespAgentToolStatus::Detached as i16;
        }
        match self.state() {
            TaskState::Running => RespAgentToolStatus::Running as i16,
            TaskState::Done => RespAgentToolStatus::Done as i16,
            TaskState::Failed => RespAgentToolStatus::Failed as i16,
            TaskState::Cancelled => RespAgentToolStatus::Cancelled as i16,
        }
    }
}

fn first_line(text: &str, max: usize) -> String {
    let line = text.lines().find(|row| !row.trim().is_empty()).unwrap_or("");
    if line.chars().count() <= max {
        return line.to_string();
    }
    line.chars().take(max).collect::<String>() + "..."
}

fn create_tasks() -> Arc<DashMap<String, DashMap<String, TaskEntry>>> {
    Arc::new(DashMap::new())
}

#[base_app(sort = 760)]
#[derive(Clone, Service)]
pub struct TaskRegistry {
    #[inject(func = create_tasks())]
    tasks: Arc<DashMap<String, DashMap<String, TaskEntry>>>,
}

#[async_trait]
impl BaseApp for TaskRegistry {
    fn name(&self) -> &'static str {
        "TaskRegistry"
    }

    async fn on_exit(&self, _ctx: &AppContext) -> anyhow::Result<()> {
        let running: Vec<TaskEntry> =
            self.tasks.iter().flat_map(|by_task| by_task.value().iter().map(|entry| entry.value().clone()).collect::<Vec<_>>()).filter(|entry| !entry.state().is_finished()).collect();
        if running.is_empty() {
            return Ok(());
        }
        info!("[ai] 退出前取消 {} 个后台任务", running.len());
        for entry in running {
            entry.cancel.cancel();
        }
        self.tasks.clear();
        Ok(())
    }
}

impl TaskRegistry {
    pub fn insert(&self, entry: TaskEntry) {
        let by_task = self.tasks.entry(entry.chat_id.clone()).or_default();
        by_task.insert(entry.task_id.clone(), entry);
    }

    pub fn get(&self, chat_id: &str, task_id: &str) -> Option<TaskEntry> {
        let by_task = self.tasks.get(chat_id)?;
        by_task.get(task_id).map(|entry| entry.value().clone())
    }

    pub fn list(&self, chat_id: &str) -> Vec<TaskEntry> {
        let Some(by_task) = self.tasks.get(chat_id) else {
            return Vec::new();
        };
        let mut rows: Vec<TaskEntry> = by_task.iter().map(|entry| entry.value().clone()).collect();
        rows.sort_by_key(|entry| entry.started_ms);
        rows
    }

    pub fn list_detached(&self, chat_id: &str) -> Vec<TaskEntry> {
        self.list(chat_id).into_iter().filter(|entry| entry.is_detached()).collect()
    }

    pub fn list_detached_running(&self, chat_id: &str) -> Vec<TaskEntry> {
        self.list_detached(chat_id).into_iter().filter(|entry| !entry.state().is_finished()).collect()
    }

    pub fn cancel_by_call_id(&self, call_id: &str) -> bool {
        for by_task in self.tasks.iter() {
            for entry in by_task.value().iter() {
                let entry = entry.value();
                if entry.call_id == call_id && !entry.state().is_finished() {
                    entry.cancel.cancel();
                    return true;
                }
            }
        }
        false
    }

    pub fn cancel_by_req_id(&self, req_id: &str) {
        for entry in self.list_all() {
            if entry.parent_req_id == req_id && !entry.state().is_finished() {
                entry.cancel.cancel();
            }
        }
    }

    pub fn event_status_by_call_id(&self, call_id: &str) -> Option<i16> {
        for by_task in self.tasks.iter() {
            for entry in by_task.value().iter() {
                if entry.value().call_id == call_id {
                    return Some(entry.value().event_status());
                }
            }
        }
        None
    }

    pub fn remove_finished_by_call_id(&self, call_id: &str) {
        let target = self.list_all().into_iter().find(|entry| entry.call_id == call_id && entry.state().is_finished() && !entry.is_detached());
        if let Some(entry) = target {
            self.remove(&entry.chat_id, &entry.task_id);
        }
    }

    fn list_all(&self) -> Vec<TaskEntry> {
        self.tasks.iter().flat_map(|by_task| by_task.value().iter().map(|entry| entry.value().clone()).collect::<Vec<_>>()).collect()
    }

    pub fn remove(&self, chat_id: &str, task_id: &str) {
        let Some(by_task) = self.tasks.get(chat_id) else {
            return;
        };
        by_task.remove(task_id);
    }

    pub fn release(&self, chat_id: &str) {
        let Some((_, by_task)) = self.tasks.remove(chat_id) else {
            return;
        };
        for entry in by_task.iter() {
            entry.value().cancel.cancel();
        }
    }
}
