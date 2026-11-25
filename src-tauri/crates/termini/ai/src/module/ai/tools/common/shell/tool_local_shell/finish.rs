use regex::Regex;
use std::sync::Arc;
use summer::tracing::info;
use crate::module::ai::dto::AiToolId;
use crate::model::req::req_chat_context::ReqChatContext;
use crate::module::shell::client_shell::{ShellExit, ShellProcess};
use crate::module::ai::shared::{WakeBus, WakeSource, WakeSourceKind};
use crate::module::ai::tools::common::shell::local_shell_wake::wait_regex_hit;
use crate::module::ai::tools::common::shell::tool_local_shell::ToolLocalShell;
use crate::module::ai::tools::common::shell::local_shell_registry::ShellProcessEntry;

impl ToolLocalShell {
    pub(super) fn finish_exited(&self, entry: &ShellProcessEntry, exit: ShellExit) -> String {
        if entry.process.kill_requested() {
            return self.finish_result(entry, "exit_code=killed\nthe process was terminated by the supervise model", true);
        }
        let elapsed = entry.elapsed_secs();
        let tail = match exit {
            ShellExit::Code(code) => format!("exit_code={} elapsed={}s", code, elapsed),
            ShellExit::Signal => format!("exit_code=signal elapsed={}s", elapsed),
        };
        self.finish_result(entry, &tail, true)
    }

    pub(super) async fn finish_killed(&self, entry: &ShellProcessEntry, reason: &str) -> String {
        let mut process = entry.process.clone();
        process.kill();
        let _ = process.wait_exit().await;
        let tail = format!("exit_code=killed\nthe command was terminated, reason: {}", reason);
        self.finish_result(entry, &tail, true)
    }

    pub(super) fn finish_cancelled(&self, entry: &ShellProcessEntry) -> String {
        entry.process.kill();
        self.finish_result(entry, "exit_code=cancelled", true)
    }

    pub(super) fn finish_detached(&self, entry: &ShellProcessEntry, chat_context: Arc<ReqChatContext>, resident: bool, reason: &str) -> String {
        if resident {
            return self.finish_resident(entry, chat_context, None, reason);
        }
        self.finish_async(entry, reason)
    }

    pub(super) fn finish_async(&self, entry: &ShellProcessEntry, reason: &str) -> String {
        let tail = format!(
            "the command keeps running asynchronously: process_id={} pid={}, reason: {}. it is not a wake source and lives only until this turn ends, then it is killed. keep going with steps that do not depend on it; when you need its result, call {} with wait_seconds to sleep until it exits and read its output; stop it early with {}.",
            entry.process_id,
            Self::pid_text(entry),
            reason,
            AiToolId::ToolShellProcessLog.as_str(),
            AiToolId::ToolShellProcessKill.as_str()
        );
        self.finish_result(entry, &tail, false)
    }

    pub(super) fn finish_resident(&self, entry: &ShellProcessEntry, chat_context: Arc<ReqChatContext>, pattern: Option<Regex>, reason: &str) -> String {
        entry.mark_resident();
        let tail = match pattern {
            Some(regex) => {
                let title = entry.command.split('\n').next().expect("split yields at least one piece").trim().to_string();
                let source = WakeSource::new(WakeSourceKind::Shell, entry.process_id.clone(), title, Some(regex), chat_context);
                self.wake_bus.register(source.clone());
                self.spawn_daemon_watcher(entry.clone(), Some(source));
                format!(
                    "the command is mounted in the background as a wake source: process_id={} pid={}, reason: {}. it outlives this turn; when a full output line matches wake_pattern or it exits you will be woken by a new [后台唤醒] message carrying the output, so do not poll it with {}. keep going with steps that do not depend on it; when nothing else is left, end this turn and state what you are waiting for. stop it with {} when it is no longer needed; the user may also stop it in the UI; a stopped source never wakes you again.",
                    entry.process_id,
                    Self::pid_text(entry),
                    reason,
                    AiToolId::ToolShellProcessLog.as_str(),
                    AiToolId::ToolShellProcessKill.as_str()
                )
            }
            None => {
                self.spawn_daemon_watcher(entry.clone(), None);
                format!(
                    "the command is mounted in the background: process_id={} pid={}, reason: {}. it outlives this turn and is not a wake source, nothing wakes you when it exits. keep going with steps that do not depend on it; read its output with {} when needed; stop it with {} when it is no longer needed; the user may also stop it in the UI.",
                    entry.process_id,
                    Self::pid_text(entry),
                    reason,
                    AiToolId::ToolShellProcessLog.as_str(),
                    AiToolId::ToolShellProcessKill.as_str()
                )
            }
        };
        self.shell_daemon_push.push_daemon_list(&entry.chat_id);
        self.finish_result(entry, &tail, false)
    }

    fn spawn_daemon_watcher(&self, entry: ShellProcessEntry, source: Option<WakeSource>) {
        let registry = self.local_shell_registry.clone();
        let push = self.shell_daemon_push.clone();
        let wake_bus = self.wake_bus.clone();
        tokio::spawn(async move {
            let mut process = entry.process.clone();
            match &source {
                Some(source) => Self::watch_wake(&entry, &mut process, source, &wake_bus).await,
                None => {
                    let _ = process.wait_exit().await;
                }
            }
            info!("[ai] 后台进程结束 chat_id={} process_id={} 状态={}", entry.chat_id, entry.process_id, entry.state_text());
            if source.is_some() {
                wake_bus.finish(&entry.chat_id, &entry.process_id, Self::finish_text(&entry));
            }
            registry.remove(&entry.chat_id, &entry.process_id);
            push.push_daemon_list(&entry.chat_id);
        });
    }

    async fn watch_wake(entry: &ShellProcessEntry, process: &mut ShellProcess, source: &WakeSource, wake_bus: &WakeBus) {
        let mut since = entry.chunk_buffer.lock().len();
        loop {
            tokio::select! {
                biased;
                _ = process.wait_exit() => return,
                _ = source.stop.cancelled() => {
                    entry.process.kill();
                    let _ = process.wait_exit().await;
                    return;
                }
                lines = wait_regex_hit(entry, source.pattern.as_ref(), &mut since) => {
                    info!("[ai] 后台进程输出命中 wake_pattern chat_id={} process_id={}", entry.chat_id, entry.process_id);
                    wake_bus.signal(&entry.chat_id, &entry.process_id, format!("输出命中 wake_pattern：\n{}", lines));
                }
            }
        }
    }

    fn pid_text(entry: &ShellProcessEntry) -> String {
        entry.process.pid().map(|pid| pid.to_string()).unwrap_or_else(|| "-".to_string())
    }

    fn finish_text(entry: &ShellProcessEntry) -> String {
        let output = entry.chunk_buffer.lock().clone();
        let tail = format!("进程已结束 状态={} 已运行={}s", entry.state_text(), entry.elapsed_secs());
        if output.is_empty() {
            return tail;
        }
        format!("{}\n{}", output, tail)
    }

    fn finish_result(&self, entry: &ShellProcessEntry, tail: &str, remove: bool) -> String {
        if remove {
            self.local_shell_registry.remove(&entry.chat_id, &entry.process_id);
        }
        let output = entry.chunk_buffer.lock().clone();
        if output.is_empty() {
            return tail.to_string();
        }
        format!("{}\n{}", output, tail)
    }
}
