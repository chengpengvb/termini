use anyhow::anyhow;
use std::sync::Arc;
use summer::tracing::info;
use async_trait::async_trait;
use crate::util::ios_local_dir;
use parking_lot::Mutex as SyncMutex;
use std::sync::atomic::{AtomicU8, Ordering};
use crate::module::shell::platform::ios::ios_system;
use crate::module::common::util::task_hub::TaskGroup;
use tokio::sync::{mpsc, oneshot, Mutex as AsyncMutex};
use crate::module::common::context::context_base::WindowSize;
use crate::model::resp::event::resp_event_shell::RespEventShell;
use crate::module::common::context::context_shell::ContextShell;
use crate::module::common::client_terminal::{ClientTerminal, EventHandler};

#[derive(Clone)]
pub struct ClientShellPty {
    open_state: Arc<AtomicU8>,
    size: Arc<SyncMutex<WindowSize>>,
    session: Arc<AsyncMutex<Option<Arc<IosShellSession>>>>,
}

struct IosShellSession {
    id: usize,
    tasks: TaskGroup,
    input_tx: mpsc::UnboundedSender<String>,
    close_tx: SyncMutex<Option<oneshot::Sender<()>>>,
}

#[async_trait]
impl ClientTerminal for ClientShellPty {
    type Event = RespEventShell;
    type Context = ContextShell;

    async fn close(&self) -> anyhow::Result<()> {
        let session = self.session.lock().await.take();
        let Some(session) = session else {
            return Ok(());
        };
        if let Some(close_tx) = session.close_tx.lock().take() {
            let _ = close_tx.send(());
        }
        ios_system::interrupt();
        session.tasks.shutdown().await;
        ios_system::close_session(session.id);
        info!("[shell] 关闭 iOS 本地终端 session={}", session.id);
        Ok(())
    }

    async fn write(&self, data: String) -> anyhow::Result<()> {
        let guard = self.session.lock().await;
        let session = guard.as_ref().ok_or_else(|| anyhow!("Session not found or already closed"))?;
        session.input_tx.send(data).map_err(|error| anyhow!("failed to write to ios shell: {}", error))
    }

    async fn set_window_size(&self, size: WindowSize) -> anyhow::Result<()> {
        *self.size.lock() = size;
        Ok(())
    }

    async fn open(&self, context: Arc<Self::Context>, handler: EventHandler<Self::Event>) -> anyhow::Result<()> {
        let cwd = Self::resolve_cwd(&context)?;
        Self::prepare()?;
        let (session, input_rx, close_rx) = self.begin()?;
        session.tasks.spawn("ios-shell", Self::interactive(session.id, cwd, handler, input_rx, close_rx));
        self.keep(session).await;
        Ok(())
    }
}

impl ClientShellPty {
    pub fn new() -> Self {
        Self { open_state: Arc::new(AtomicU8::new(0)), size: Arc::new(SyncMutex::new(WindowSize::default())), session: Arc::new(AsyncMutex::new(None)) }
    }

    pub async fn pid(&self) -> Option<u32> {
        None
    }

    pub async fn open_command(&self, context: Arc<ContextShell>, command_text: &str, handler: EventHandler<RespEventShell>) -> anyhow::Result<()> {
        let cwd = Self::resolve_cwd(&context)?;
        Self::prepare()?;
        let (session, input_rx, close_rx) = self.begin()?;
        let command = command_text.to_string();
        session.tasks.spawn("ios-shell-command", Self::one_shot(session.id, cwd, command, handler, input_rx, close_rx));
        self.keep(session).await;
        Ok(())
    }

    fn begin(&self) -> anyhow::Result<(Arc<IosShellSession>, mpsc::UnboundedReceiver<String>, oneshot::Receiver<()>)> {
        if self.open_state.compare_exchange(0, 1, Ordering::AcqRel, Ordering::Acquire).is_err() {
            return Err(anyhow!("shell already opened"));
        }
        let id = ios_system::new_session();
        let (input_tx, input_rx) = mpsc::unbounded_channel::<String>();
        let (close_tx, close_rx) = oneshot::channel::<()>();
        let session = Arc::new(IosShellSession { id, tasks: TaskGroup::new(), input_tx, close_tx: SyncMutex::new(Some(close_tx)) });
        Ok((session, input_rx, close_rx))
    }

    async fn keep(&self, session: Arc<IosShellSession>) {
        info!("[shell] iOS 本地终端打开成功 session={}", session.id);
        *self.session.lock().await = Some(session);
    }

    async fn interactive(id: usize, cwd: String, handler: EventHandler<RespEventShell>, mut input_rx: mpsc::UnboundedReceiver<String>, mut close_rx: oneshot::Receiver<()>) {
        let _ = handler(RespEventShell::OnOpen).await;
        let mut current = match Self::enter(id, &cwd).await {
            Ok(dir) => dir,
            Err(error) => {
                let _ = handler(RespEventShell::OnError(error.to_string())).await;
                cwd.clone()
            }
        };
        Self::emit(&handler, Self::prompt(&current)).await;
        let mut line = String::new();
        loop {
            let input = tokio::select! {
                _ = &mut close_rx => break,
                input = input_rx.recv() => match input {
                    Some(text) => text,
                    None => break,
                },
            };
            for ch in input.chars() {
                match ch {
                    '\r' | '\n' => {
                        Self::emit(&handler, "\r\n".to_string()).await;
                        let command = line.trim().to_string();
                        line.clear();
                        if !command.is_empty() {
                            Self::execute(id, &command, &handler, &mut input_rx).await;
                            match Self::pwd(id).await {
                                Ok(dir) => current = dir,
                                Err(error) => {
                                    let _ = handler(RespEventShell::OnError(error.to_string())).await;
                                }
                            }
                        }
                        Self::emit(&handler, Self::prompt(&current)).await;
                    }
                    '\u{3}' => {
                        ios_system::interrupt();
                        line.clear();
                        Self::emit(&handler, "^C\r\n".to_string()).await;
                        Self::emit(&handler, Self::prompt(&current)).await;
                    }
                    '\u{7f}' | '\u{8}' => {
                        if line.pop().is_some() {
                            Self::emit(&handler, "\u{8} \u{8}".to_string()).await;
                        }
                    }
                    _ => {
                        line.push(ch);
                        Self::emit(&handler, ch.to_string()).await;
                    }
                }
            }
        }
        let _ = handler(RespEventShell::OnClose).await;
    }

    async fn one_shot(id: usize, cwd: String, command: String, handler: EventHandler<RespEventShell>, mut input_rx: mpsc::UnboundedReceiver<String>, mut close_rx: oneshot::Receiver<()>) {
        let _ = handler(RespEventShell::OnOpen).await;
        if let Err(error) = Self::enter(id, &cwd).await {
            let _ = handler(RespEventShell::OnError(error.to_string())).await;
        }
        let code = tokio::select! {
            _ = &mut close_rx => None,
            code = Self::execute(id, &command, &handler, &mut input_rx) => code,
        };
        let _ = handler(RespEventShell::Exit { code }).await;
    }

    async fn execute(id: usize, command: &str, handler: &EventHandler<RespEventShell>, input_rx: &mut mpsc::UnboundedReceiver<String>) -> Option<i32> {
        let (chunk_tx, mut chunk_rx) = mpsc::unbounded_channel::<Vec<u8>>();
        let owned = command.to_string();
        let task = tokio::task::spawn_blocking(move || ios_system::run(id, &owned, &chunk_tx));
        loop {
            tokio::select! {
                chunk = chunk_rx.recv() => match chunk {
                    Some(data) => {
                        let _ = handler(RespEventShell::OnData(data)).await;
                    }
                    None => break,
                },
                input = input_rx.recv() => {
                    if input.is_some_and(|text| text.contains('\u{3}')) {
                        ios_system::interrupt();
                    }
                }
            }
        }
        match task.await {
            Ok(Ok(code)) => Some(code),
            Ok(Err(error)) => {
                let _ = handler(RespEventShell::OnError(error.to_string())).await;
                None
            }
            Err(error) => {
                let _ = handler(RespEventShell::OnError(error.to_string())).await;
                None
            }
        }
    }

    async fn enter(id: usize, cwd: &str) -> anyhow::Result<String> {
        let target = cwd.to_string();
        tokio::task::spawn_blocking(move || ios_system::enter(id, &target)).await??;
        Self::pwd(id).await
    }

    async fn pwd(id: usize) -> anyhow::Result<String> {
        tokio::task::spawn_blocking(move || ios_system::logical_pwd(id)).await?
    }

    async fn emit(handler: &EventHandler<RespEventShell>, text: String) {
        let _ = handler(RespEventShell::OnData(text.into_bytes())).await;
    }

    fn prompt(cwd: &str) -> String {
        let name = cwd.rsplit('/').find(|part| !part.is_empty()).unwrap_or("/");
        format!("{} $ ", name)
    }

    fn prepare() -> anyhow::Result<()> {
        let documents = ios_local_dir::documents_dir()?;
        let root = documents.to_str().ok_or_else(|| anyhow!("non-UTF-8 documents path"))?;
        ios_system::initialize(root)
    }

    fn resolve_cwd(context: &ContextShell) -> anyhow::Result<String> {
        let configured = context.config.read().cwd.clone();
        ios_system::session_dir(configured)
    }
}
