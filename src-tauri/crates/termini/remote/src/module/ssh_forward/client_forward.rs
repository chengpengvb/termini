use std::sync::Arc;
use futures::FutureExt;
use russh::client::Handle;
use summer::tracing::warn;
use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use anyhow::{anyhow, bail, Result};
use parking_lot::Mutex as SyncMutex;
use tokio::sync::Mutex as AsyncMutex;
use std::sync::atomic::{AtomicU8, Ordering};
use summer::plugin::component::ComponentRef;
use crate::module::common::client_terminal::EventHandler;
use crate::model::port_forward::{ForwardKind, PortForward};
use crate::module::common::context::context_ssh::ContextSsh;
use crate::module::ssh_forward::pump::{forward_pump, socks5};
use crate::module::ssh::client_ssh_handler::ClientSshHandler;
use crate::module::ssh_forward::pump::forward_sink::ForwardSink;
use crate::model::resp::event::resp_event_forward::RespEventForward;
use crate::module::ssh::client_session_factory::ClientSessionFactory;

type SshHandle = Arc<Handle<ClientSshHandler>>;
type ConnTasks = Arc<SyncMutex<Vec<JoinHandle<()>>>>;
type StartOutcome = (u16, Option<JoinHandle<()>>, Option<(String, u16)>);

pub struct ClientForward {
    session_id: String,
    open_state: Arc<AtomicU8>,
    factory: ComponentRef<ClientSessionFactory>,
    inner: Arc<AsyncMutex<Option<ForwardRuntime>>>,
}

struct ForwardRuntime {
    channel_id: String,
    handle: SshHandle,
    conns: ConnTasks,
    accept_handle: Option<JoinHandle<()>>,
    remote_bind: Option<(String, u16)>,
}

impl ClientForward {
    pub fn new(session_id: String, factory: ComponentRef<ClientSessionFactory>) -> Self {
        Self { session_id, factory, open_state: Arc::new(AtomicU8::new(0)), inner: Arc::new(AsyncMutex::new(None)) }
    }

    pub async fn start(&self, rule: PortForward, context: Arc<ContextSsh>, handler: EventHandler<RespEventForward>) -> Result<u16> {
        if self.open_state.compare_exchange(0, 1, Ordering::AcqRel, Ordering::Acquire).is_err() {
            bail!("forward already started");
        }
        let kind = rule.forward_kind().ok_or_else(|| anyhow!("invalid forward kind: {}", rule.kind))?;
        let (channel_id, handle) = self.factory.connect_forward_anchor(self.session_id.clone(), context.as_ref(), handler, None).await?;
        let conns: ConnTasks = Arc::new(SyncMutex::new(Vec::new()));
        let outcome = match kind {
            ForwardKind::Local => Self::start_local(&rule, handle.clone(), conns.clone()).await,
            ForwardKind::Dynamic => Self::start_dynamic(&rule, handle.clone(), conns.clone()).await,
            ForwardKind::Remote => self.start_remote(&rule, handle.clone(), conns.clone()).await,
        };
        match outcome {
            Ok((bound_port, accept_handle, remote_bind)) => {
                let mut guard = self.inner.lock().await;
                *guard = Some(ForwardRuntime { channel_id, handle, conns, accept_handle, remote_bind });
                Ok(bound_port)
            }
            Err(error) => {
                let _ = self.factory.close(&channel_id).await;
                Err(error)
            }
        }
    }

    pub async fn stop(&self) -> Result<()> {
        let runtime = {
            let mut guard = self.inner.lock().await;
            guard.take()
        };
        let Some(runtime) = runtime else {
            return Ok(());
        };
        if let Some(accept_handle) = runtime.accept_handle {
            accept_handle.abort();
            if let Err(error) = accept_handle.await {
                if !error.is_cancelled() {
                    warn!("[ssh] forward 等待监听任务退出失败: {}", error);
                }
            }
        }
        {
            let mut conns = runtime.conns.lock();
            for task in conns.drain(..) {
                task.abort();
            }
        }
        if let Some((address, port)) = runtime.remote_bind {
            if let Some(Err(error)) = runtime.handle.cancel_tcpip_forward(address.clone(), port as u32).now_or_never() {
                warn!("[ssh] forward 取消远端转发失败 [{}:{}]: {}", address, port, error);
            }
            self.factory.deregister_forward(&(address, port));
        }
        self.factory.close(&runtime.channel_id).await?;
        Ok(())
    }

    async fn start_local(rule: &PortForward, handle: SshHandle, conns: ConnTasks) -> Result<StartOutcome> {
        let bind_address = rule.bind_address.clone();
        let listener =
            TcpListener::bind((bind_address.as_str(), rule.bind_port as u16)).await.map_err(|e| anyhow!("forward failed to listen locally on {}:{}: {}", bind_address, rule.bind_port, e))?;
        let bound_port = listener.local_addr().map(|addr| addr.port()).unwrap_or(rule.bind_port as u16);
        let target_host = rule.target_host.clone();
        let target_port = rule.target_port as u16;
        let conns_for_loop = conns.clone();
        let accept_handle = tokio::spawn(async move {
            loop {
                let (tcp, peer) = match listener.accept().await {
                    Ok(value) => value,
                    Err(error) => {
                        warn!("[ssh] forward 本地接受连接失败: {}", error);
                        break;
                    }
                };
                let handle = handle.clone();
                let target_host = target_host.clone();
                let task = tokio::spawn(async move {
                    let channel = match handle.channel_open_direct_tcpip(target_host, target_port as u32, peer.ip().to_string(), peer.port() as u32).await {
                        Ok(channel) => channel,
                        Err(error) => {
                            warn!("[ssh] forward 打开 direct-tcpip 失败: {}", error);
                            return;
                        }
                    };
                    forward_pump::pump(channel, tcp).await;
                });
                conns_for_loop.lock().push(task);
            }
        });
        Ok((bound_port, Some(accept_handle), None))
    }

    async fn start_dynamic(rule: &PortForward, handle: SshHandle, conns: ConnTasks) -> Result<StartOutcome> {
        let bind_address = rule.bind_address.clone();
        let listener =
            TcpListener::bind((bind_address.as_str(), rule.bind_port as u16)).await.map_err(|e| anyhow!("forward failed to start dynamic listener on {}:{}: {}", bind_address, rule.bind_port, e))?;
        let bound_port = listener.local_addr().map(|addr| addr.port()).unwrap_or(rule.bind_port as u16);
        let conns_for_loop = conns.clone();
        let accept_handle = tokio::spawn(async move {
            loop {
                let (tcp, peer) = match listener.accept().await {
                    Ok(value) => value,
                    Err(error) => {
                        warn!("[ssh] forward 动态接受连接失败: {}", error);
                        break;
                    }
                };
                let handle = handle.clone();
                let task = tokio::spawn(async move {
                    let mut tcp = tcp;
                    let target = match socks5::handshake(&mut tcp).await {
                        Ok(target) => target,
                        Err(error) => {
                            warn!("[ssh] forward socks5 握手失败: {}", error);
                            return;
                        }
                    };
                    let channel = match handle.channel_open_direct_tcpip(target.host, target.port as u32, peer.ip().to_string(), peer.port() as u32).await {
                        Ok(channel) => channel,
                        Err(error) => {
                            warn!("[ssh] forward 动态打开 direct-tcpip 失败: {}", error);
                            let _ = socks5::reply_failed(&mut tcp).await;
                            return;
                        }
                    };
                    if let Err(error) = socks5::reply_succeeded(&mut tcp).await {
                        warn!("[ssh] forward socks5 成功应答失败: {}", error);
                        return;
                    }
                    forward_pump::pump(channel, tcp).await;
                });
                conns_for_loop.lock().push(task);
            }
        });
        Ok((bound_port, Some(accept_handle), None))
    }

    async fn start_remote(&self, rule: &PortForward, handle: SshHandle, conns: ConnTasks) -> Result<StartOutcome> {
        let returned_port =
            handle.tcpip_forward(rule.bind_address.clone(), rule.bind_port as u32).await.map_err(|e| anyhow!("forward failed to listen remotely on {}:{}: {}", rule.bind_address, rule.bind_port, e))?
                as u16;
        let bound_port = if returned_port == 0 { rule.bind_port as u16 } else { returned_port };
        let key = (rule.bind_address.clone(), bound_port);
        let sink = ForwardSink { target_host: rule.target_host.clone(), target_port: rule.target_port as u16, conns };
        self.factory.register_forward(key.clone(), sink);
        Ok((bound_port, None, Some(key)))
    }
}
