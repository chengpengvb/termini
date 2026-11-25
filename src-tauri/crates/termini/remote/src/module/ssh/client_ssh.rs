use anyhow::anyhow;
use russh::Channel;
use std::sync::Arc;
use russh::ChannelMsg;
use russh::client::Msg;
use summer::tracing::warn;
use tokio::task::JoinHandle;
use async_trait::async_trait;
use parking_lot::Mutex as SyncMutex;
use tokio::time::{timeout, Duration};
use std::sync::atomic::{AtomicU8, Ordering};
use summer::plugin::component::ComponentRef;
use tokio::sync::{mpsc, Mutex as AsyncMutex};
use crate::module::common::dto::ssh_session::SshSession;
use crate::service::service_device_type::DeviceOsDetector;
use crate::model::resp::event::resp_event_ssh::RespEventSsh;
use crate::module::common::context::context_ssh::ContextSsh;
use crate::module::common::context::context_base::WindowSize;
use crate::module::common::dto::ssh_channel_kind::SshChannelKind;
use crate::module::ssh::client_session_factory::ClientSessionFactory;
use crate::module::common::client_terminal::{ClientTerminal, EventHandler};
use crate::model::resp::event::resp_event_ssh_auth::{OpenSshStep, RespSshOpenStepType};

const DRIVER_EXIT_WAIT: Duration = Duration::from_secs(3);

enum SshCommand {
    Write(String),
    Resize(WindowSize),
}

#[derive(Clone)]
pub struct ClientSsh {
    session_id: String,
    open_state: Arc<AtomicU8>,
    factory: ComponentRef<ClientSessionFactory>,
    session: Arc<AsyncMutex<Option<SshClientSession>>>,
}

struct SshClientSession {
    channel_id: String,
    cmd_tx: mpsc::Sender<SshCommand>,
    driver_handle: SyncMutex<Option<JoinHandle<()>>>,
}

#[async_trait]
impl ClientTerminal for ClientSsh {
    type Event = RespEventSsh;
    type Context = ContextSsh;

    async fn close(&self) -> anyhow::Result<()> {
        let session = {
            let mut guard = self.session.lock().await;
            guard.take()
        };
        if let Some(session) = session {
            drop(session.cmd_tx);
            let driver_handle = session.driver_handle.lock().take();
            if let Some(mut handle) = driver_handle {
                if timeout(DRIVER_EXIT_WAIT, &mut handle).await.is_err() {
                    handle.abort();
                    warn!("[ssh] ssh 通道关闭等待驱动退出超时 [{}]", session.channel_id);
                }
            }
            self.factory.close(&session.channel_id).await?;
        }
        Ok(())
    }

    async fn write(&self, data: String) -> anyhow::Result<()> {
        let sender = {
            let guard = self.session.lock().await;
            guard.as_ref().ok_or_else(|| anyhow!("Session not found or already closed"))?.cmd_tx.clone()
        };
        timeout(DRIVER_EXIT_WAIT, sender.send(SshCommand::Write(data)))
            .await
            .map_err(|error| anyhow!("SSH write queue timed out: {}", error))?
            .map_err(|e| anyhow!("Failed to send ssh write: {}", e))?;
        Ok(())
    }

    async fn set_window_size(&self, size: WindowSize) -> anyhow::Result<()> {
        let sender = {
            let guard = self.session.lock().await;
            guard.as_ref().ok_or_else(|| anyhow!("Session not found or already closed"))?.cmd_tx.clone()
        };
        timeout(DRIVER_EXIT_WAIT, sender.send(SshCommand::Resize(size)))
            .await
            .map_err(|error| anyhow!("SSH resize queue timed out: {}", error))?
            .map_err(|e| anyhow!("Failed to send ssh resize: {}", e))?;
        Ok(())
    }

    async fn open(&self, context: Arc<Self::Context>, handler: EventHandler<Self::Event>) -> anyhow::Result<()> {
        if self.open_state.compare_exchange(0, 1, Ordering::AcqRel, Ordering::Acquire).is_err() {
            return Err(anyhow!("ssh already opened"));
        }
        let SshSession { channel_id, channel } = self.factory.connect(self.session_id.clone(), context.as_ref(), None, handler.clone(), SshChannelKind::Pty, None).await?;
        let _ = handler(RespEventSsh::OnOpenStep(OpenSshStep { progress: 100.0, step_type: RespSshOpenStepType::SessionOpened, data: None })).await;
        let _ = handler(RespEventSsh::OnOpen).await;
        let (cmd_tx, cmd_rx) = mpsc::channel::<SshCommand>(128);
        let driver_handle = Self::spawn_driver(self.factory.clone(), channel_id.clone(), channel, cmd_rx, handler);

        let mut guard = self.session.lock().await;
        *guard = Some(SshClientSession { channel_id, cmd_tx, driver_handle: SyncMutex::new(Some(driver_handle)) });
        Ok(())
    }
}

impl ClientSsh {
    pub fn new(session_id: String, factory: ComponentRef<ClientSessionFactory>) -> Self {
        Self { session_id, factory, open_state: Arc::new(AtomicU8::new(0)), session: Arc::new(AsyncMutex::new(None)) }
    }

    async fn detect_device_os(detector: &mut DeviceOsDetector, data: &[u8], handler: &EventHandler<RespEventSsh>) {
        let Some(os_type) = detector.observe(data) else {
            return;
        };
        let _ = handler(RespEventSsh::OnDeviceOsType(os_type.as_i16())).await;
    }

    fn spawn_driver(
        factory: ComponentRef<ClientSessionFactory>,
        channel_id: String,
        mut channel: Channel<Msg>,
        mut cmd_rx: mpsc::Receiver<SshCommand>,
        handler: EventHandler<RespEventSsh>,
    ) -> JoinHandle<()> {
        tokio::spawn(async move {
            let mut detector = DeviceOsDetector::new();
            detector.start();
            loop {
                let wake = tokio::select! {
                    cmd = cmd_rx.recv() => DriverWake::Cmd(cmd),
                    msg = channel.wait() => DriverWake::Msg(msg),
                };
                match wake {
                    DriverWake::Cmd(None) => break,
                    DriverWake::Cmd(Some(SshCommand::Write(data))) => {
                        if channel.data(data.as_bytes()).await.is_err() {
                            break;
                        }
                    }
                    DriverWake::Cmd(Some(SshCommand::Resize(size))) => {
                        let _ = channel.window_change(size.cols as u32, size.rows as u32, size.pixel_width as u32, size.pixel_height as u32).await;
                    }
                    DriverWake::Msg(Some(ChannelMsg::Data { data })) => {
                        Self::detect_device_os(&mut detector, &data, &handler).await;
                        let _ = handler(RespEventSsh::OnData(data.to_vec())).await;
                    }
                    DriverWake::Msg(Some(ChannelMsg::ExtendedData { data, .. })) => {
                        Self::detect_device_os(&mut detector, &data, &handler).await;
                        let _ = handler(RespEventSsh::OnData(data.to_vec())).await;
                    }
                    DriverWake::Msg(Some(ChannelMsg::Close)) | DriverWake::Msg(None) => break,
                    DriverWake::Msg(Some(_)) => {}
                }
            }
            let _ = channel.close().await;
            if let Err(error) = factory.close(&channel_id).await {
                warn!("[ssh] ssh 通道断开后清理失败 [{}]: {}", channel_id, error);
            }
            let _ = handler(RespEventSsh::OnClose).await;
        })
    }
}

enum DriverWake {
    Cmd(Option<SshCommand>),
    Msg(Option<ChannelMsg>),
}
