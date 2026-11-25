use std::sync::Arc;
use dashmap::DashMap;
use parking_lot::Mutex;
use tokio::sync::Notify;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use crate::base::base_app::BaseApp;
use crate::util::time_util::now_millis;
use crate::module::shell::client_shell::ShellExit;
use crate::module::common::client_terminal::EventHandler;
use crate::module::shell::client_shell_pty::ClientShellPty;
use crate::model::resp::event::resp_event_shell::RespEventShell;
use crate::module::common::context::context_shell::ContextShell;
use crate::model::resp::resp_shell::{ShellInputLog, ShellInputSource, ShellSnapshot};

const ENTER: char = '\r';
const ESC: char = '\u{1b}';
const INPUT_LOG_MAX: usize = 200;
const SCROLLBACK_MAX_BYTES: usize = 256 * 1024;

pub fn prepare_text_for_terminal(text: &str) -> String {
    text.replace("\r\n", "\r").replace('\n', "\r")
}

pub fn enter_key() -> String {
    ENTER.to_string()
}

pub fn ctrl_key(letter: char) -> Option<String> {
    if !letter.is_ascii_alphabetic() {
        return None;
    }
    Some(((letter.to_ascii_uppercase() as u8 - b'@') as char).to_string())
}

pub fn is_terminal_reply(data: &str) -> bool {
    let Some(rest) = data.strip_prefix(ESC) else {
        return false;
    };
    if rest.starts_with(']') || rest.starts_with('P') {
        return true;
    }
    let Some(body) = rest.strip_prefix('[') else {
        return false;
    };
    if body == "I" || body == "O" {
        return true;
    }
    let Some(final_byte) = body.chars().last() else {
        return false;
    };
    if !matches!(final_byte, 'R' | 'c' | 'y') {
        return false;
    }
    body[..body.len() - final_byte.len_utf8()].chars().all(|c| c.is_ascii_digit() || matches!(c, ';' | '?' | '>' | '=' | '$'))
}

pub struct ShellSessionRecord {
    exit_notify: Notify,
    pub command: String,
    attach_notify: Notify,
    snapshot_notify: Notify,
    scrollback: Mutex<Vec<u8>>,
    pub pty: Arc<ClientShellPty>,
    pub context: Arc<ContextShell>,
    exit: Mutex<Option<ShellExit>>,
    inputs: Mutex<Vec<ShellInputLog>>,
    snapshot: Mutex<Option<ShellSnapshot>>,
    viewer: Mutex<Option<EventHandler<RespEventShell>>>,
}

impl ShellSessionRecord {
    pub fn new(pty: Arc<ClientShellPty>, command: String, context: Arc<ContextShell>) -> Self {
        Self {
            pty,
            command,
            context,
            exit: Mutex::new(None),
            viewer: Mutex::new(None),
            snapshot: Mutex::new(None),
            exit_notify: Notify::new(),
            attach_notify: Notify::new(),
            inputs: Mutex::new(Vec::new()),
            snapshot_notify: Notify::new(),
            scrollback: Mutex::new(Vec::new()),
        }
    }

    pub fn sink(self: &Arc<Self>) -> EventHandler<RespEventShell> {
        let record = Arc::clone(self);
        Arc::new(move |event| {
            let record = Arc::clone(&record);
            Box::pin(async move {
                record.on_pty_event(event).await;
                Ok(())
            })
        })
    }

    async fn on_pty_event(&self, event: RespEventShell) {
        match &event {
            RespEventShell::OnData(data) => self.push_scrollback(data),
            RespEventShell::Exit { code } => self.mark_exit(code.map(ShellExit::Code).unwrap_or(ShellExit::Signal)),
            RespEventShell::OnClose | RespEventShell::OnError(_) => {
                if self.exit().is_none() {
                    self.mark_exit(ShellExit::Signal);
                }
            }
            RespEventShell::OnOpen | RespEventShell::SnapshotRequest => {}
        }
        let viewer = self.viewer.lock().clone();
        if let Some(viewer) = viewer {
            let _ = viewer(event).await;
        }
    }

    fn push_scrollback(&self, data: &[u8]) {
        let mut scrollback = self.scrollback.lock();
        scrollback.extend_from_slice(data);
        if scrollback.len() > SCROLLBACK_MAX_BYTES {
            let overflow = scrollback.len() - SCROLLBACK_MAX_BYTES;
            scrollback.drain(..overflow);
        }
    }

    pub fn scrollback(&self) -> Vec<u8> {
        self.scrollback.lock().clone()
    }

    pub fn attach(&self, viewer: EventHandler<RespEventShell>) {
        *self.viewer.lock() = Some(viewer);
        self.attach_notify.notify_waiters();
    }

    pub fn detach(&self) {
        *self.viewer.lock() = None;
    }

    pub fn is_attached(&self) -> bool {
        self.viewer.lock().is_some()
    }

    pub async fn wait_attached(&self) {
        loop {
            let notified = self.attach_notify.notified();
            if self.is_attached() {
                return;
            }
            notified.await;
        }
    }

    pub async fn request_snapshot(&self) -> bool {
        let viewer = self.viewer.lock().clone();
        let Some(viewer) = viewer else {
            return false;
        };
        viewer(RespEventShell::SnapshotRequest).await.is_ok()
    }

    pub fn mark_exit(&self, exit: ShellExit) {
        *self.exit.lock() = Some(exit);
        self.exit_notify.notify_waiters();
    }

    pub fn exit(&self) -> Option<ShellExit> {
        *self.exit.lock()
    }

    pub fn is_running(&self) -> bool {
        self.exit().is_none()
    }

    pub async fn wait_exit(&self) -> ShellExit {
        loop {
            let notified = self.exit_notify.notified();
            if let Some(exit) = self.exit() {
                return exit;
            }
            notified.await;
        }
    }

    pub fn record_input(&self, source: ShellInputSource, data: &str) {
        if is_terminal_reply(data) {
            return;
        }
        let mut inputs = self.inputs.lock();
        inputs.push(ShellInputLog { source, data: data.to_string(), at_ms: now_millis() });
        if inputs.len() > INPUT_LOG_MAX {
            let overflow = inputs.len() - INPUT_LOG_MAX;
            inputs.drain(..overflow);
        }
    }

    pub fn inputs_since(&self, since_ms: i64) -> Vec<ShellInputLog> {
        self.inputs.lock().iter().filter(|log| log.at_ms > since_ms).cloned().collect()
    }

    pub fn put_snapshot(&self, snapshot: ShellSnapshot) {
        *self.snapshot.lock() = Some(snapshot);
        self.snapshot_notify.notify_waiters();
    }

    pub fn snapshot(&self) -> Option<ShellSnapshot> {
        self.snapshot.lock().clone()
    }

    pub async fn wait_snapshot_after(&self, after_ms: i64) -> ShellSnapshot {
        loop {
            let notified = self.snapshot_notify.notified();
            if let Some(snapshot) = self.snapshot() {
                if snapshot.at_ms > after_ms {
                    return snapshot;
                }
            }
            notified.await;
        }
    }
}

fn create_records() -> Arc<DashMap<String, Arc<ShellSessionRecord>>> {
    Arc::new(DashMap::new())
}

#[base_app(sort = 210)]
#[derive(Clone, Service)]
pub struct ShellSessionHub {
    #[inject(func = create_records())]
    records: Arc<DashMap<String, Arc<ShellSessionRecord>>>,
}

#[async_trait]
impl BaseApp for ShellSessionHub {
    fn name(&self) -> &'static str {
        "ShellSessionHub"
    }
}

impl ShellSessionHub {
    pub fn insert(&self, process_id: &str, record: Arc<ShellSessionRecord>) {
        self.records.insert(process_id.to_string(), record);
    }

    pub fn get(&self, process_id: &str) -> Option<Arc<ShellSessionRecord>> {
        self.records.get(process_id).map(|entry| entry.value().clone())
    }

    pub fn remove(&self, process_id: &str) {
        self.records.remove(process_id);
    }
}
