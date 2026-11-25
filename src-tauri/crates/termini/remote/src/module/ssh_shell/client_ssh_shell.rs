use russh::Channel;
use std::sync::Arc;
use std::sync::Mutex;
use russh::ChannelMsg;
use russh::client::Msg;
use std::time::Duration;
use crate::SshAuthAnswer;
use tokio::time::timeout;
use anyhow::{anyhow, Result};
use tokio::sync::Mutex as AsyncMutex;
use summer::plugin::component::ComponentRef;
use crate::module::common::client_terminal::EventHandler;
use crate::module::common::dto::ssh_channel_kind::SshChannelKind;
use crate::module::ssh::client_session_factory::ClientSessionFactory;
use crate::model::resp::resp_ssh_open_failure::format_ssh_open_failure;
use crate::module::common::context::context_ssh_shell::ContextSshShell;
use crate::module::common::dto::shell_chunk_callback::ShellChunkCallback;
use crate::model::resp::event::resp_event_ssh_auth::{OpenSshStep, SshConnectEvent};

struct ShellSession {
    channel_id: String,
    channel: Channel<Msg>,
}

pub struct EventSshShellProbe {
    pub step: OpenSshStep,
}

impl SshConnectEvent for EventSshShellProbe {
    fn on_open_step(step: OpenSshStep) -> Self {
        EventSshShellProbe { step }
    }
}

#[derive(Clone)]
pub struct ClientSshShell {
    session_id: String,
    factory: ComponentRef<ClientSessionFactory>,
    state: Arc<AsyncMutex<Option<ShellSession>>>,
}

impl ClientSshShell {
    pub fn new(session_id: String, factory: ComponentRef<ClientSessionFactory>) -> Self {
        Self { session_id, factory, state: Arc::new(AsyncMutex::new(None)) }
    }

    pub async fn open(&self, context: &ContextSshShell, force_share: Option<bool>) -> Result<()> {
        let mut guard = self.state.lock().await;
        if guard.is_some() {
            return Err(anyhow!("ssh shell already opened"));
        }
        let steps = Arc::new(Mutex::new(Vec::<OpenSshStep>::new()));
        let steps_for_handler = steps.clone();
        let handler: EventHandler<EventSshShellProbe> = Arc::new(move |event| {
            let steps_for_handler = steps_for_handler.clone();
            Box::pin(async move {
                steps_for_handler.lock().expect("ssh open steps lock").push(event.step);
                Ok(())
            })
        });
        let connect = self.factory.connect(self.session_id.clone(), context, Some(SshAuthAnswer::HostFingerprintTrustOnce.to_code()), handler, SshChannelKind::Shell, force_share);
        let session = match connect.await {
            Ok(session) => session,
            Err(error) => {
                let collected = steps.lock().expect("ssh open steps lock").clone();
                let device_id = context.device.read().id.clone();
                return Err(anyhow!(format_ssh_open_failure(&device_id, &error.to_string(), &collected)));
            }
        };
        *guard = Some(ShellSession { channel_id: session.channel_id, channel: session.channel });
        Ok(())
    }

    pub async fn close(&self) -> Result<()> {
        let session = self.state.lock().await.take();
        let Some(session) = session else {
            return Ok(());
        };
        let _ = session.channel.close().await;
        self.factory.close(&session.channel_id).await
    }

    pub async fn exec(&self, command: String, exec_timeout: Duration, on_chunk: Option<ShellChunkCallback>) -> Result<(i32, String)> {
        let mut guard = self.state.lock().await;
        let session = guard.as_mut().ok_or_else(|| anyhow!("ssh shell not opened"))?;
        let collect = Self::collect_exec(&mut session.channel, command.as_bytes(), on_chunk);
        match timeout(exec_timeout, collect).await {
            Ok(Ok((exit_code, output))) => Ok((exit_code, String::from_utf8_lossy(&output).into_owned())),
            Ok(Err(error)) => Err(error),
            Err(_) => {
                let aborted = guard.take();
                drop(guard);
                self.abort_session(aborted).await;
                Err(anyhow!("ssh shell exec timed out after {:?}", exec_timeout))
            }
        }
    }

    async fn collect_exec(channel: &mut Channel<Msg>, command: &[u8], on_chunk: Option<ShellChunkCallback>) -> Result<(i32, Vec<u8>)> {
        channel.exec(true, command).await.map_err(|error| anyhow!("ssh shell exec failed: {}", error))?;
        let mut output = Vec::new();
        let mut exit_code: i32 = 0;
        loop {
            let Some(msg) = channel.wait().await else {
                break;
            };
            match msg {
                ChannelMsg::Data { data } => {
                    output.extend_from_slice(&data);
                    if let Some(callback) = &on_chunk {
                        callback(data.to_vec()).await;
                    }
                }
                ChannelMsg::ExtendedData { data, .. } => {
                    output.extend_from_slice(&data);
                    if let Some(callback) = &on_chunk {
                        callback(data.to_vec()).await;
                    }
                }
                ChannelMsg::ExitStatus { exit_status } => exit_code = exit_status as i32,
                ChannelMsg::Eof | ChannelMsg::Close => break,
                _ => {}
            }
        }
        Ok((exit_code, output))
    }

    async fn abort_session(&self, session: Option<ShellSession>) {
        let Some(session) = session else {
            return;
        };
        let _ = session.channel.close().await;
        let _ = self.factory.close(&session.channel_id).await;
    }
}
