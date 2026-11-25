use std::sync::Arc;
use dashmap::DashMap;
use parking_lot::Mutex;
use std::time::Duration;
use summer::tracing::info;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use crate::util::time_util::now_millis;
use std::sync::atomic::{AtomicBool, Ordering};
use crate::base::base_app::{AppContext, BaseApp};
use crate::module::shell::client_shell_pty::ClientShellPty;
use crate::module::shell::client_shell::{ShellExit, ShellProcess};
use crate::model::resp::resp_agent::{RespAgentShellDaemonRow, RespAgentToolStatus};

const EXIT_KILL_WAIT: Duration = Duration::from_secs(3);
const DAEMON_OUTPUT_TAIL_CHARS: usize = 8000;

#[derive(Clone)]
pub struct ShellProcessEntry {
    pub chat_id: String,
    pub call_id: String,
    pub command: String,
    pub shell_id: String,
    pub started_ms: i64,
    pub process_id: String,
    pub cwd: Option<String>,
    pub process: ShellProcess,
    pub resident: Arc<AtomicBool>,
    pub pty: Option<Arc<ClientShellPty>>,
    pub chunk_buffer: Arc<Mutex<String>>,
    pub last_at_ms: Arc<Mutex<Option<i64>>>,
}

impl ShellProcessEntry {
    pub fn is_tty(&self) -> bool {
        self.pty.is_some()
    }

    pub fn state_text(&self) -> String {
        match self.process.exit_status() {
            None => "running".to_string(),
            Some(ShellExit::Code(code)) => {
                if self.process.kill_requested() {
                    return format!("killed(exit={})", code);
                }
                format!("exited={}", code)
            }
            Some(ShellExit::Signal) => {
                if self.process.kill_requested() {
                    return "killed".to_string();
                }
                "signal".to_string()
            }
        }
    }

    pub fn elapsed_secs(&self) -> i64 {
        now_millis().saturating_sub(self.started_ms) / 1000
    }

    pub fn silence_secs(&self) -> i64 {
        let last = self.last_at_ms.lock().unwrap_or(self.started_ms);
        now_millis().saturating_sub(last) / 1000
    }

    pub fn is_resident(&self) -> bool {
        self.resident.load(Ordering::Relaxed)
    }

    pub fn mark_resident(&self) {
        self.resident.store(true, Ordering::Relaxed)
    }

    pub fn is_running(&self) -> bool {
        self.process.exit_status().is_none()
    }

    pub fn summary_text(&self) -> String {
        if self.is_tty() {
            return format!(
                "process_id={} 状态={} pid={} 已运行={}s 交互终端=是 cwd={} 命令={}",
                self.process_id,
                self.state_text(),
                self.process.pid().map(|pid| pid.to_string()).unwrap_or_else(|| "-".to_string()),
                self.elapsed_secs(),
                self.cwd.clone().unwrap_or_else(|| "-".to_string()),
                self.command.replace('\n', " ")
            );
        }
        format!(
            "process_id={} 状态={} pid={} 已运行={}s 静默={}s 输出字数={} 常驻={} 交互终端={} cwd={} 命令={}",
            self.process_id,
            self.state_text(),
            self.process.pid().map(|pid| pid.to_string()).unwrap_or_else(|| "-".to_string()),
            self.elapsed_secs(),
            self.silence_secs(),
            self.chunk_buffer.lock().chars().count(),
            if self.is_resident() { "是" } else { "否" },
            if self.is_tty() { "是" } else { "否" },
            self.cwd.clone().unwrap_or_else(|| "-".to_string()),
            self.command.replace('\n', " ")
        )
    }

    pub fn output_tail(&self, max_chars: usize) -> String {
        let output = self.chunk_buffer.lock();
        let count = output.chars().count();
        if count <= max_chars {
            return output.clone();
        }
        output.chars().skip(count - max_chars).collect()
    }

    pub fn to_daemon_row(&self) -> RespAgentShellDaemonRow {
        RespAgentShellDaemonRow {
            tty: self.is_tty(),
            pid: self.process.pid(),
            cwd: self.cwd.clone(),
            started_ms: self.started_ms,
            command: self.command.clone(),
            process_id: self.process_id.clone(),
            shell_id: Some(self.shell_id.clone()),
            output: self.output_tail(DAEMON_OUTPUT_TAIL_CHARS),
        }
    }
}

fn create_processes() -> Arc<DashMap<String, DashMap<String, ShellProcessEntry>>> {
    Arc::new(DashMap::new())
}

#[base_app(sort = 761)]
#[derive(Clone, Service)]
pub struct LocalShellRegistry {
    #[inject(func = create_processes())]
    processes: Arc<DashMap<String, DashMap<String, ShellProcessEntry>>>,
}

#[async_trait]
impl BaseApp for LocalShellRegistry {
    fn name(&self) -> &'static str {
        "LocalShellRegistry"
    }

    async fn on_exit(&self, _ctx: &AppContext) -> anyhow::Result<()> {
        let entries: Vec<ShellProcessEntry> = self.processes.iter().flat_map(|by_process| by_process.value().iter().map(|entry| entry.value().clone()).collect::<Vec<_>>()).collect();
        let running: Vec<ShellProcessEntry> = entries.into_iter().filter(|entry| entry.is_running()).collect();
        if running.is_empty() {
            return Ok(());
        }
        info!("[ai] 退出前关闭 {} 个后台进程.", running.len());
        for entry in &running {
            entry.process.kill();
        }
        let wait_all = async {
            for entry in running {
                let mut process = entry.process.clone();
                let _ = process.wait_exit().await;
            }
        };
        if tokio::time::timeout(EXIT_KILL_WAIT, wait_all).await.is_err() {
            info!("[ai] 等待后台进程退出超时, 交由系统回收.");
        }
        self.processes.clear();
        Ok(())
    }
}

impl LocalShellRegistry {
    pub fn insert(&self, entry: ShellProcessEntry) {
        let by_process = self.processes.entry(entry.chat_id.clone()).or_default();
        by_process.insert(entry.process_id.clone(), entry);
    }

    pub fn get(&self, chat_id: &str, process_id: &str) -> Option<ShellProcessEntry> {
        let by_process = self.processes.get(chat_id)?;
        by_process.get(process_id).map(|entry| entry.value().clone())
    }

    pub fn list(&self, chat_id: &str) -> Vec<ShellProcessEntry> {
        let Some(by_process) = self.processes.get(chat_id) else {
            return Vec::new();
        };
        let mut rows: Vec<ShellProcessEntry> = by_process.iter().map(|entry| entry.value().clone()).collect();
        rows.sort_by_key(|entry| entry.started_ms);
        rows
    }

    pub fn list_running_resident(&self, chat_id: &str) -> Vec<ShellProcessEntry> {
        self.list(chat_id).into_iter().filter(|entry| entry.is_resident() && entry.is_running()).collect()
    }

    pub fn event_status_by_call_id(&self, chat_id: &str, call_id: &str) -> Option<i16> {
        let detached = self.list(chat_id).iter().any(|entry| entry.call_id == call_id && entry.is_running());
        detached.then_some(RespAgentToolStatus::Detached as i16)
    }

    pub fn remove(&self, chat_id: &str, process_id: &str) {
        let Some(by_process) = self.processes.get(chat_id) else {
            return;
        };
        by_process.remove(process_id);
    }

    pub fn release(&self, chat_id: &str) {
        let Some((_, by_process)) = self.processes.remove(chat_id) else {
            return;
        };
        for entry in by_process.iter() {
            entry.value().process.kill();
        }
    }

    pub fn release_transient(&self, chat_id: &str) {
        let Some(by_process) = self.processes.get(chat_id) else {
            return;
        };
        let transient_ids: Vec<String> = by_process.iter().filter(|entry| !entry.value().is_resident()).map(|entry| entry.key().clone()).collect();
        for process_id in transient_ids {
            if let Some((_, entry)) = by_process.remove(&process_id) {
                entry.process.kill();
            }
        }
    }
}
