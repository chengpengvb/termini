use anyhow::anyhow;
use tokio::sync::watch;
#[cfg(not(target_os = "ios"))]
use std::process::Stdio;
use summer::tracing::warn;
#[cfg(not(target_os = "ios"))]
use tokio::io::AsyncReadExt;
#[cfg(not(target_os = "ios"))]
use tokio::process::Command;
#[cfg(not(target_os = "ios"))]
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
#[cfg(target_os = "ios")]
use crate::module::shell::platform::ios::ios_system;
#[cfg(not(target_os = "ios"))]
use crate::module::common::util::merge_env::merge_env;
#[cfg(not(target_os = "ios"))]
use crate::module::common::context::context_shell::ConfigShell;
use crate::module::common::context::context_shell::ContextShell;
use crate::module::common::dto::shell_chunk_callback::ShellChunkCallback;
#[cfg(not(target_os = "ios"))]
use crate::module::shell::platform::shell_discover::{kill_process_tree, limit_cpu_rate, native_shell_platform, spawn_in_own_group};

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShellExit {
    Code(i32),
    Signal,
}

#[derive(Clone)]
pub struct ShellProcess {
    pid: Option<u32>,
    kill: CancellationToken,
    exit: watch::Receiver<Option<ShellExit>>,
}

impl ShellProcess {
    pub fn new(pid: Option<u32>, kill: CancellationToken, exit: watch::Receiver<Option<ShellExit>>) -> Self {
        Self { pid, kill, exit }
    }

    pub fn pid(&self) -> Option<u32> {
        self.pid
    }

    pub fn kill(&self) {
        self.kill.cancel();
    }

    pub fn kill_requested(&self) -> bool {
        self.kill.is_cancelled()
    }

    pub fn exit_status(&self) -> Option<ShellExit> {
        *self.exit.borrow()
    }

    pub async fn wait_exit(&mut self) -> anyhow::Result<ShellExit> {
        loop {
            if let Some(exit) = *self.exit.borrow() {
                return Ok(exit);
            }
            self.exit.changed().await.map_err(|error| anyhow!("local shell exit notification lost: {}", error))?;
        }
    }
}

#[derive(Clone)]
pub struct ClientShell {}

impl ClientShell {
    pub fn new() -> Self {
        Self {}
    }

    #[cfg(target_os = "ios")]
    pub async fn spawn(&self, context: &ContextShell, command: String, on_chunk: ShellChunkCallback) -> anyhow::Result<ShellProcess> {
        let cwd = ios_system::session_dir(context.config.read().cwd.clone())?;
        let session = ios_system::new_session();
        let kill = CancellationToken::new();
        let kill_signal = kill.clone();
        let (exit_tx, exit_rx) = watch::channel(None);
        let (chunk_tx, mut chunk_rx) = tokio::sync::mpsc::unbounded_channel::<Vec<u8>>();
        tokio::spawn(async move {
            while let Some(data) = chunk_rx.recv().await {
                on_chunk(data).await;
            }
        });
        tokio::spawn(async move {
            let task = tokio::task::spawn_blocking(move || {
                ios_system::enter(session, &cwd)?;
                ios_system::run(session, &command, &chunk_tx)
            });
            let exit = tokio::select! {
                _ = kill_signal.cancelled() => {
                    ios_system::interrupt();
                    ShellExit::Signal
                }
                result = task => match result {
                    Ok(Ok(code)) => ShellExit::Code(code),
                    Ok(Err(error)) => {
                        warn!("[shell] iOS 命令执行失败 session={} error={}", session, error);
                        ShellExit::Signal
                    }
                    Err(error) => {
                        warn!("[shell] iOS 命令任务收束失败 session={} error={}", session, error);
                        ShellExit::Signal
                    }
                },
            };
            ios_system::close_session(session);
            let _ = exit_tx.send(Some(exit));
        });
        Ok(ShellProcess::new(None, kill, exit_rx))
    }

    #[cfg(not(target_os = "ios"))]
    pub async fn spawn(&self, context: &ContextShell, command: String, on_chunk: ShellChunkCallback) -> anyhow::Result<ShellProcess> {
        let (mut cmd, cpu_percent) = {
            let config = context.config.read();
            (self.build_exec_command(&config, command.as_str())?, config.cpu_percent)
        };
        cmd.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).kill_on_drop(true);
        spawn_in_own_group(&mut cmd);
        let mut child = cmd.spawn().map_err(|error| anyhow!("failed to start local shell: {}", error))?;
        if let Err(error) = limit_cpu_rate(&child, cpu_percent) {
            warn!("[shell] 未能压制本地 shell 的 CPU 占用 percent={} error={}", cpu_percent, error);
        }
        let Some(stdout) = child.stdout.take() else {
            return Err(anyhow!("local shell stdout pipe unavailable"));
        };
        let Some(stderr) = child.stderr.take() else {
            return Err(anyhow!("local shell stderr pipe unavailable"));
        };
        let pid = child.id();

        let stdout_on_chunk = on_chunk.clone();
        let stderr_on_chunk = on_chunk;
        let stdout_task = tokio::spawn(async move { Self::read_pipe(stdout, stdout_on_chunk).await });
        let stderr_task = tokio::spawn(async move { Self::read_pipe(stderr, stderr_on_chunk).await });

        let kill = CancellationToken::new();
        let kill_signal = kill.clone();
        let (exit_tx, exit_rx) = watch::channel(None);
        tokio::spawn(async move {
            let status = tokio::select! {
                status = child.wait() => status,
                _ = kill_signal.cancelled() => {
                    Self::kill_tree(pid).await;
                    match child.try_wait() {
                        Ok(None) => {
                            if let Err(error) = child.kill().await {
                                warn!("[shell] 终止本地 shell 失败 pid={:?} error={}", pid, error);
                            }
                        }
                        Ok(Some(_)) => {}
                        Err(error) => warn!("[shell] 查询本地 shell 状态失败 pid={:?} error={}", pid, error),
                    }
                    child.wait().await
                }
            };
            Self::join_reader(stdout_task, pid, "stdout").await;
            Self::join_reader(stderr_task, pid, "stderr").await;
            let exit = match status {
                Ok(status) => match status.code() {
                    Some(code) => ShellExit::Code(code),
                    None => ShellExit::Signal,
                },
                Err(error) => {
                    warn!("[shell] 等待本地 shell 结束失败 pid={:?} error={}", pid, error);
                    ShellExit::Signal
                }
            };
            let _ = exit_tx.send(Some(exit));
        });
        Ok(ShellProcess { pid, kill, exit: exit_rx })
    }

    #[cfg(not(target_os = "ios"))]
    async fn kill_tree(pid: Option<u32>) {
        let Some(pid) = pid else {
            warn!("[shell] 本地 shell 无 pid，跳过进程树终止");
            return;
        };
        if let Err(error) = kill_process_tree(pid).await {
            warn!("[shell] 终止本地 shell 进程树失败 pid={} error={}", pid, error);
        }
    }

    #[cfg(not(target_os = "ios"))]
    async fn join_reader(task: JoinHandle<anyhow::Result<()>>, pid: Option<u32>, pipe: &str) {
        match task.await {
            Ok(Ok(())) => {}
            Ok(Err(error)) => warn!("[shell] 读取本地 shell {} 失败 pid={:?} error={}", pipe, pid, error),
            Err(error) => warn!("[shell] 收束本地 shell {} 读取任务失败 pid={:?} error={}", pipe, pid, error),
        }
    }

    #[cfg(not(target_os = "ios"))]
    async fn read_pipe<R: AsyncReadExt + Unpin>(mut reader: R, on_chunk: ShellChunkCallback) -> anyhow::Result<()> {
        let mut buf = [0u8; 8192];
        loop {
            let n = reader.read(&mut buf).await.map_err(|error| anyhow!("failed to read local shell output: {}", error))?;
            if n == 0 {
                break;
            }
            on_chunk(buf[..n].to_vec()).await;
        }
        Ok(())
    }

    #[cfg(not(target_os = "ios"))]
    fn build_exec_command(&self, config: &ConfigShell, command: &str) -> anyhow::Result<Command> {
        let descriptor = &config.shell_descr;
        let Some(exe) = descriptor.exe.as_ref() else {
            return Err(anyhow!("shell.exe is not set"));
        };
        let args = native_shell_platform().build_exec_args(descriptor, command)?;
        let mut cmd = Command::new(exe);
        cmd.args(args);
        // 主程序是 GUI 子系统, 子进程自带控制台的话会在 Windows 上弹出黑窗;
        // 这里输出走管道进终端组件, 不需要真实控制台。
        #[cfg(windows)]
        cmd.creation_flags(CREATE_NO_WINDOW);
        for (key, value) in merge_env(&descriptor.env, &config.env) {
            cmd.env(key, value);
        }
        if let Some(directory) = &config.cwd {
            cmd.current_dir(directory);
        }
        Ok(cmd)
    }
}
