use std::io::Read;
use anyhow::anyhow;
use std::io::Write;
use std::sync::Arc;
use summer::tracing::info;
use async_trait::async_trait;
use parking_lot::Mutex as SyncMutex;
use std::sync::atomic::{AtomicU8, Ordering};
use crate::module::common::util::task_hub::TaskGroup;
use crate::module::common::util::merge_env::merge_env;
use tokio::sync::{mpsc, oneshot, Mutex as AsyncMutex};
use crate::module::common::context::context_base::WindowSize;
use crate::model::resp::event::resp_event_shell::RespEventShell;
use crate::module::shell::platform::shell_discover::native_shell_platform;
use crate::module::common::client_terminal::{ClientTerminal, EventHandler};
use crate::module::common::context::context_shell::{ConfigShell, ContextShell};
use portable_pty::{native_pty_system, Child, ChildKiller, CommandBuilder, MasterPty, PtyPair, PtySize};

#[derive(Clone)]
pub struct ClientShellPty {
    open_state: Arc<AtomicU8>,
    session: Arc<AsyncMutex<Option<Arc<ShellSession>>>>,
    context: Arc<AsyncMutex<Option<Arc<ContextShell>>>>,
}

struct ShellSession {
    pid: Option<u32>,
    tasks: TaskGroup,
    master: AsyncMutex<Box<dyn MasterPty + Send>>,
    close_tx: SyncMutex<Option<oneshot::Sender<()>>>,
    writer: AsyncMutex<Option<Box<dyn Write + Send>>>,
    killer: SyncMutex<Option<Box<dyn ChildKiller + Send + Sync>>>,
}

#[async_trait]
impl ClientTerminal for ClientShellPty {
    type Event = RespEventShell;
    type Context = ContextShell;

    async fn close(&self) -> anyhow::Result<()> {
        let session = {
            let mut guard = self.session.lock().await;
            guard.take()
        };
        if let Some(session) = session {
            if let Some(mut killer) = session.killer.lock().take() {
                let _ = killer.kill();
            }
            if let Some(close_tx) = session.close_tx.lock().take() {
                let _ = close_tx.send(());
            }
            {
                let mut writer_guard = session.writer.lock().await;
                *writer_guard = None;
            }
            session.tasks.shutdown().await;
            match self.context.lock().await.as_ref() {
                Some(c) => {
                    info!("[shell] 关闭本地终端:[{:?}].", c.config.read().shell_descr.shell_id);
                }
                None => {
                    info!("[shell] 关闭本地终端:[{:?}].", "Context not found");
                }
            }
        }
        Ok(())
    }

    async fn write(&self, data: String) -> anyhow::Result<()> {
        let guard = self.session.lock().await;
        let session = guard.as_ref().ok_or_else(|| anyhow!("Session not found or already closed"))?;
        let mut writer_guard = session.writer.lock().await;
        if let Some(writer) = writer_guard.as_mut() {
            writer.write_all(data.as_bytes()).map_err(|e| anyhow!("Failed to write to PTY: {}", e))?;
            writer.flush().map_err(|e| anyhow!("Failed to flush PTY: {}", e))?;
        } else {
            return Err(anyhow!("Writer is already closed"));
        }
        Ok(())
    }

    async fn set_window_size(&self, size: WindowSize) -> anyhow::Result<()> {
        let guard = self.session.lock().await;
        let session = guard.as_ref().ok_or_else(|| anyhow!("Session not found or already closed"))?;
        let master = session.master.lock().await;
        let pty_size = PtySize { rows: size.rows, cols: size.cols, pixel_width: size.pixel_width, pixel_height: size.pixel_height };
        master.resize(pty_size).map_err(|e| anyhow!("Failed to resize PTY window: {}", e))?;
        Ok(())
    }

    async fn open(&self, context: Arc<Self::Context>, handler: EventHandler<Self::Event>) -> anyhow::Result<()> {
        let command = {
            let config = context.config.read();
            self.build_command(&config)?
        };
        self.open_with(context, command, handler).await
    }
}

impl ClientShellPty {
    pub fn new() -> Self {
        Self { open_state: Arc::new(AtomicU8::new(0)), session: Arc::new(AsyncMutex::new(None)), context: Arc::new(AsyncMutex::new(None)) }
    }

    pub async fn pid(&self) -> Option<u32> {
        self.session.lock().await.as_ref().and_then(|session| session.pid)
    }

    pub async fn open_command(&self, context: Arc<ContextShell>, command_text: &str, handler: EventHandler<RespEventShell>) -> anyhow::Result<()> {
        let command = {
            let config = context.config.read();
            self.build_exec_command(&config, command_text)?
        };
        self.open_with(context, command, handler).await
    }

    async fn open_with(&self, context: Arc<ContextShell>, command: CommandBuilder, handler: EventHandler<RespEventShell>) -> anyhow::Result<()> {
        if self.open_state.compare_exchange(0, 1, Ordering::AcqRel, Ordering::Acquire).is_err() {
            return Err(anyhow!("shell already opened"));
        }
        {
            let mut guard = self.context.lock().await;
            *guard = Some(Arc::clone(&context));
        }
        let pty_pair = self.open_pty(context.size)?;
        let (master, child, reader, writer) = self.spawn_shell(pty_pair, command)?;
        let pid = child.process_id();
        let killer = child.clone_killer();

        let (close_tx, close_rx) = oneshot::channel::<()>();
        let (exit_tx, exit_rx) = oneshot::channel::<Option<i32>>();
        let (event_tx, event_rx) = mpsc::channel::<RespEventShell>(128);
        let tasks = TaskGroup::new();
        Self::spawn_dispatch(&tasks, handler, close_rx, event_rx, exit_rx);
        Self::spawn_reader(&tasks, reader, event_tx);
        Self::spawn_wait(&tasks, child, exit_tx);

        let session = Arc::new(ShellSession {
            pid,
            tasks,
            master: AsyncMutex::new(master),
            killer: SyncMutex::new(Some(killer)),
            writer: AsyncMutex::new(Some(writer)),
            close_tx: SyncMutex::new(Some(close_tx)),
        });

        let mut guard = self.session.lock().await;
        *guard = Some(session);
        let config = context.config.read();
        info!("[shell] 本地终端打开 [{}] 成功.", config.shell_descr.shell_id);
        Ok(())
    }

    fn build_command(&self, config: &ConfigShell) -> anyhow::Result<CommandBuilder> {
        let descriptor = &config.shell_descr;
        let Some(exe) = descriptor.exe.as_deref() else {
            return Err(anyhow!("shell.exe is required!"));
        };
        let mut command = CommandBuilder::new(exe);
        for arg in &descriptor.args {
            command.arg(arg);
        }
        self.init_command(config, &mut command);
        Ok(command)
    }

    fn build_exec_command(&self, config: &ConfigShell, command_text: &str) -> anyhow::Result<CommandBuilder> {
        let descriptor = &config.shell_descr;
        let Some(exe) = descriptor.exe.as_deref() else {
            return Err(anyhow!("shell.exe is required!"));
        };
        let mut command = CommandBuilder::new(exe);
        for arg in native_shell_platform().build_exec_args(descriptor, command_text)? {
            command.arg(arg);
        }
        self.init_command(config, &mut command);
        Ok(command)
    }

    fn open_pty(&self, size: WindowSize) -> anyhow::Result<PtyPair> {
        let pty_system = native_pty_system();
        pty_system.openpty(PtySize { rows: size.rows, cols: size.cols, pixel_width: 0, pixel_height: 0 }).map_err(|error| anyhow!("Failed to open PTY: {}", error))
    }

    fn spawn_shell(&self, pty_pair: PtyPair, command: CommandBuilder) -> anyhow::Result<(Box<dyn MasterPty + Send>, Box<dyn Child + Send + Sync>, Box<dyn Read + Send>, Box<dyn Write + Send>)> {
        let child = pty_pair.slave.spawn_command(command).map_err(|error| anyhow!("Start the shell process error: {}!", error))?;
        drop(pty_pair.slave);

        let reader = pty_pair.master.try_clone_reader().map_err(|error| anyhow!("Clone of PTY reading end failed: {}", error))?;
        let writer = pty_pair.master.take_writer().map_err(|error| anyhow!("Failed to obtain the write end of PTY: {}", error))?;

        Ok((pty_pair.master, child, reader, writer))
    }

    fn spawn_wait(tasks: &TaskGroup, mut child: Box<dyn Child + Send + Sync>, exit_tx: oneshot::Sender<Option<i32>>) {
        tasks.spawn_blocking("pty-wait", move || {
            let code = child.wait().ok().and_then(|status| i32::try_from(status.exit_code()).ok());
            let _ = exit_tx.send(code);
        });
    }

    fn init_command(&self, config: &ConfigShell, command: &mut CommandBuilder) {
        let descriptor = &config.shell_descr;
        for (key, value) in merge_env(&descriptor.env, &config.env) {
            command.env(key, value);
        }
        if let Some(directory) = &config.cwd {
            command.cwd(directory.as_str());
        }
    }

    fn spawn_reader(tasks: &TaskGroup, mut reader: Box<dyn Read + Send>, event_tx: mpsc::Sender<RespEventShell>) {
        tasks.spawn_blocking("pty-reader", move || {
            let mut buf = [0u8; 8192];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => {
                        let _ = event_tx.try_send(RespEventShell::OnClose);
                        break;
                    }
                    Ok(n) => match event_tx.try_send(RespEventShell::OnData(buf[..n].to_vec())) {
                        Ok(()) | Err(mpsc::error::TrySendError::Full(_)) => {}
                        Err(mpsc::error::TrySendError::Closed(_)) => break,
                    },
                    Err(e) => {
                        let _ = event_tx.try_send(RespEventShell::OnError(e.to_string()));
                        break;
                    }
                }
            }
        });
    }

    fn spawn_dispatch(
        tasks: &TaskGroup,
        handler: EventHandler<RespEventShell>,
        mut close_rx: oneshot::Receiver<()>,
        mut event_rx: mpsc::Receiver<RespEventShell>,
        mut exit_rx: oneshot::Receiver<Option<i32>>,
    ) {
        tasks.spawn("pty-dispatch", async move {
            let _ = handler(RespEventShell::OnOpen).await;
            loop {
                tokio::select! {
                    _ = &mut close_rx => {
                        let _ = handler(RespEventShell::OnClose).await;
                        break;
                    }
                    code = &mut exit_rx => {
                        let _ = handler(RespEventShell::Exit { code: code.unwrap_or(None) }).await;
                        break;
                    }
                    maybe_ev = event_rx.recv() => {
                        let Some(ev) = maybe_ev else {
                            break;
                        };
                        let _ = handler(ev).await;
                    }
                }
            }
        });
    }
}
