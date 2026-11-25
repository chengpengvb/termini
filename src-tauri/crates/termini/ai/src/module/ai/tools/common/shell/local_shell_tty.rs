use anyhow::Result;
use std::sync::Arc;
use parking_lot::Mutex;
use tokio::sync::watch;
use summer::tracing::info;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use crate::util::id_factory::uuid;
use std::sync::atomic::AtomicBool;
use crate::base::base_app::BaseApp;
use crate::util::time_util::now_millis;
use tokio_util::sync::CancellationToken;
use tokio::time::{sleep, Duration, Instant};
use crate::module::ai::shared::ShellDaemonPush;
use crate::module::ai::tools::ai_tool_error::AiToolError;
use crate::module::common::client_terminal::ClientTerminal;
use crate::module::shell::client_shell_pty::ClientShellPty;
use crate::module::common::context::context_base::WindowSize;
use crate::module::common::context::context_shell::ContextShell;
use crate::module::shell::client_shell::{ShellExit, ShellProcess};
use crate::model::resp::resp_shell::{ShellInputSource, ShellSnapshot};
use crate::module::shell::shell_session_hub::{ShellSessionHub, ShellSessionRecord};
use crate::module::ai::tools::common::shell::local_shell_registry::{LocalShellRegistry, ShellProcessEntry};

const TTY_ROWS: u16 = 40;
const TTY_COLS: u16 = 120;
const ATTACH_WAIT: Duration = Duration::from_secs(15);
const SNAPSHOT_WAIT: Duration = Duration::from_secs(8);
const EXIT_POLL: Duration = Duration::from_millis(200);
const SETTLE_WAIT: Duration = Duration::from_millis(1200);

#[base_app(sort = 761)]
#[derive(Clone, Service)]
pub struct LocalShellTty {
    #[inject(component)]
    shell_daemon_push: ShellDaemonPush,

    #[inject(component)]
    shell_session_hub: ShellSessionHub,

    #[inject(component)]
    local_shell_registry: LocalShellRegistry,
}

#[async_trait]
impl BaseApp for LocalShellTty {
    fn name(&self) -> &'static str {
        "LocalShellTty"
    }
}

impl LocalShellTty {
    pub async fn spawn(&self, chat_id: String, call_id: String, shell_id: String, command: String, mut shell_context: ContextShell) -> Result<ShellProcessEntry, AiToolError> {
        shell_context.size = WindowSize { dpi: 0, cols: TTY_COLS, rows: TTY_ROWS, pixel_width: 0, pixel_height: 0 };
        let shell_context = Arc::new(shell_context);
        let pty = Arc::new(ClientShellPty::new());
        let record = Arc::new(ShellSessionRecord::new(pty.clone(), command.clone(), shell_context.clone()));
        pty.open_command(shell_context.clone(), &command, record.sink()).await.map_err(|error| AiToolError::Execution(error.to_string()))?;
        let process = Self::bridge_process(pty.clone(), record.clone()).await;
        let process_id = uuid();
        self.shell_session_hub.insert(&process_id, record);
        let cwd = shell_context.config.read().cwd.clone();
        let entry = ShellProcessEntry {
            cwd,
            process,
            chat_id,
            call_id,
            command,
            shell_id,
            process_id,
            pty: Some(pty),
            started_ms: now_millis(),
            last_at_ms: Arc::new(Mutex::new(None)),
            resident: Arc::new(AtomicBool::new(true)),
            chunk_buffer: Arc::new(Mutex::new(String::new())),
        };
        self.local_shell_registry.insert(entry.clone());
        self.shell_daemon_push.push_daemon_list(&entry.chat_id);
        self.spawn_exit_watcher(entry.clone());
        Ok(entry)
    }

    pub fn record(&self, process_id: &str) -> Option<Arc<ShellSessionRecord>> {
        self.shell_session_hub.get(process_id)
    }

    pub async fn wait_attached(&self, record: &ShellSessionRecord, cancel: &[CancellationToken]) -> bool {
        tokio::select! {
            _ = record.wait_attached() => true,
            _ = sleep(ATTACH_WAIT) => false,
            _ = Self::any_cancelled(cancel) => false,
        }
    }

    pub async fn capture(&self, record: &ShellSessionRecord, settle: Duration, cancel: &[CancellationToken]) -> Option<ShellSnapshot> {
        if !record.is_attached() {
            return record.snapshot();
        }
        let since = now_millis();
        let deadline = Instant::now() + settle;
        loop {
            if !record.is_running() || Instant::now() >= deadline {
                break;
            }
            tokio::select! {
                _ = record.wait_exit() => break,
                _ = sleep(EXIT_POLL) => {}
                _ = Self::any_cancelled(cancel) => return record.snapshot(),
            }
        }
        if !record.request_snapshot().await {
            return record.snapshot();
        }
        tokio::select! {
            snapshot = record.wait_snapshot_after(since) => Some(snapshot),
            _ = sleep(SNAPSHOT_WAIT) => record.snapshot(),
            _ = Self::any_cancelled(cancel) => record.snapshot(),
        }
    }

    pub async fn write(&self, record: &ShellSessionRecord, chars: &str) -> Result<(), AiToolError> {
        record.record_input(ShellInputSource::Agent, chars);
        record.pty.write(chars.to_string()).await.map_err(|error| AiToolError::Execution(format!("failed to write to terminal: {}", error)))
    }

    pub fn settle_wait() -> Duration {
        SETTLE_WAIT
    }

    pub fn render(record: &ShellSessionRecord, entry: &ShellProcessEntry, snapshot: Option<&ShellSnapshot>, since_ms: i64) -> String {
        let mut out = String::new();
        out.push_str(&entry.summary_text());
        out.push('\n');
        let user_inputs: Vec<String> = record.inputs_since(since_ms).into_iter().filter(|log| log.source == ShellInputSource::User).map(|log| Self::describe_input(&log.data)).collect();
        if !user_inputs.is_empty() {
            out.push_str(&format!("[用户在终端里输入了 {} 段内容：{}]\n", user_inputs.len(), user_inputs.join("、")));
        }
        match snapshot {
            Some(snapshot) => {
                out.push_str("[终端当前屏幕]\n");
                out.push_str(snapshot.text.trim_end());
                out.push('\n');
            }
            None if record.is_attached() => out.push_str("[终端已打开，但还没有拿到屏幕快照]\n"),
            None => out.push_str("[终端尚未在界面上打开，看不到屏幕内容；用户打开这个终端后再读]\n"),
        }
        if let Some(exit) = record.exit() {
            out.push_str(&match exit {
                ShellExit::Code(code) => format!("exit_code={}\n", code),
                ShellExit::Signal => "exit_code=signal\n".to_string(),
            });
        }
        out
    }

    fn describe_input(data: &str) -> String {
        let visible: String = data.chars().filter(|value| !value.is_control()).collect();
        if visible.is_empty() {
            return format!("{} 个控制字符", data.chars().count());
        }
        format!("{} 个字符", visible.chars().count())
    }

    async fn any_cancelled(cancel: &[CancellationToken]) {
        if cancel.is_empty() {
            std::future::pending::<()>().await;
        }
        let waits: Vec<_> = cancel.iter().map(|token| token.cancelled()).collect();
        futures::future::select_all(waits.into_iter().map(Box::pin)).await;
    }

    async fn bridge_process(pty: Arc<ClientShellPty>, record: Arc<ShellSessionRecord>) -> ShellProcess {
        let (exit_tx, exit_rx) = watch::channel(None);
        let kill = CancellationToken::new();
        let kill_signal = kill.clone();
        let exit_record = record.clone();
        tokio::spawn(async move {
            let exit = exit_record.wait_exit().await;
            let _ = exit_tx.send(Some(exit));
        });
        let kill_pty = pty.clone();
        tokio::spawn(async move {
            kill_signal.cancelled().await;
            if let Err(error) = kill_pty.close().await {
                info!("[ai] 关闭交互终端失败 error={}", error);
            }
        });
        ShellProcess::new(pty.pid().await, kill, exit_rx)
    }

    fn spawn_exit_watcher(&self, entry: ShellProcessEntry) {
        let registry = self.local_shell_registry.clone();
        let hub = self.shell_session_hub.clone();
        let push = self.shell_daemon_push.clone();
        tokio::spawn(async move {
            let mut process = entry.process.clone();
            let _ = process.wait_exit().await;
            info!("[ai] 交互进程结束 chat_id={} process_id={} 状态={}", entry.chat_id, entry.process_id, entry.state_text());
            registry.remove(&entry.chat_id, &entry.process_id);
            hub.remove(&entry.process_id);
            push.push_daemon_list(&entry.chat_id);
        });
    }
}
