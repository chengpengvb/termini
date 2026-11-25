use russh::client;
use anyhow::Result;
use russh::Channel;
use std::sync::Arc;
use dashmap::DashMap;
use russh::Disconnect;
use futures::FutureExt;
use russh::client::Msg;
use tokio::time::timeout;
use russh::client::Config;
use russh::client::Handle;
use tokio::net::TcpStream;
use tokio::time::Duration;
use summer::plugin::Service;
use crate::util::id_factory::uuid;
use summer::tracing::{debug, info, warn};
use summer::plugin::component::ComponentRef;
use crate::service::service_base::BaseService;
use crate::service::service_device::DeviceService;
use crate::module::ssh::ssh_transport::SshTransport;
use crate::module::common::dto::ssh_prompt::SshPrompt;
use crate::module::common::dto::ssh_session::SshSession;
use crate::module::common::client_terminal::EventHandler;
use crate::service::service_config_ssh::ConfigSshService;
use crate::service::service_known_host::KnownHostService;
use crate::module::common::context::context_ssh::ContextSsh;
use crate::module::ssh::client_ssh_handler::ClientSshHandler;
use crate::module::common::dto::ssh_auth_input::SshAuthAnswer;
use crate::module::common::dto::ssh_session_key::SshSessionKey;
use crate::module::ssh_forward::pump::forward_sink::ForwardSink;
use crate::module::common::dto::ssh_channel_kind::SshChannelKind;
use crate::service::service_config_key_pair::ConfigKeyPairService;
use crate::model::resp::event::resp_event_ssh_auth::{RespSshOpenStepType, SshConnectEvent, SshOpenCallback};
use crate::module::common::dto::ssh_session_state::{SshAuthAnswerEntry, SshAuthInbox, SshChannel, SshSessionFactoryState, SshSharedConn};

const DEFAULT_CONNECT_TIMEOUT_SECONDS: u64 = 3;
const DEFAULT_SSH_TERM: &str = "xterm-256color";
const MAX_JUMP_DEPTH: usize = 8;

fn create_state() -> Arc<SshSessionFactoryState<ClientSshHandler>> {
    Arc::new(SshSessionFactoryState::new())
}

fn create_session2_callback() -> Arc<DashMap<String, Arc<SshOpenCallback>>> {
    Arc::new(DashMap::new())
}

fn create_forward_registry() -> Arc<DashMap<(String, u16), ForwardSink>> {
    Arc::new(DashMap::new())
}

#[derive(Clone, Service)]
pub struct ClientSessionFactory {
    #[inject(component)]
    config_key_pair_service: ConfigKeyPairService,

    #[inject(component)]
    device_service: DeviceService,

    #[inject(component)]
    config_ssh_service: ConfigSshService,

    known_host_service: ComponentRef<KnownHostService>,

    #[inject(func = create_state())]
    state: Arc<SshSessionFactoryState<ClientSshHandler>>,

    #[inject(func = create_forward_registry())]
    forward_registry: Arc<DashMap<(String, u16), ForwardSink>>,

    #[inject(func = create_session2_callback())]
    session_id2_callback: Arc<DashMap<String, Arc<SshOpenCallback>>>,
}

impl ClientSessionFactory {
    pub(crate) fn key_pair_service(&self) -> &ConfigKeyPairService {
        &self.config_key_pair_service
    }

    pub fn cancel_connect(&self, session_id: &str) {
        let Some(entry) = self.session_id2_callback.get(session_id) else {
            return;
        };
        entry.cancel();
    }

    pub async fn connect<E>(&self, session_id: String, context: &ContextSsh, trust_code: Option<i16>, handler: EventHandler<E>, kind: SshChannelKind, force_share: Option<bool>) -> Result<SshSession>
    where
        E: SshConnectEvent,
    {
        let callback = SshOpenCallback::new(session_id.clone(), handler, kind);
        self.connect_internal(context, trust_code, callback, kind, force_share).await
    }

    async fn connect_internal(&self, context: &ContextSsh, trust_code: Option<i16>, callback: SshOpenCallback, kind: SshChannelKind, force_share: Option<bool>) -> Result<SshSession> {
        let channel_id = uuid();
        let callback = Arc::new(callback);
        self.session_id2_callback.insert(callback.session_id.clone(), callback.clone());

        self.auth_inbox(&channel_id);
        let channel_id_for_cleanup = channel_id.clone();
        let key = self.resolve_reuse_key(context, force_share);
        let result = (|| async {
            let handle = match &key {
                None => self.acquire_independent(&channel_id, context, trust_code, &callback).await?,
                Some(key) => self.acquire_shared(&channel_id, context, trust_code, &callback, key.clone()).await?,
            };
            let channel = match self.open_channel(context, kind, &callback, &handle).await {
                Ok(channel) => channel,
                Err(error) => {
                    if let Some(key) = &key {
                        warn!("[ssh] ssh 共享连接 [{}:{}] 无法再开通道, 丢弃复用缓存: {}", key.host, key.port, error);
                        self.forget_shared(key, &handle);
                    }
                    return Err(error);
                }
            };
            self.state.channels.insert(channel_id.clone(), SshChannel { key, handle, callback: (*callback).clone() });
            Ok::<_, anyhow::Error>(SshSession { channel_id, channel })
        })()
        .await;
        self.session_id2_callback.remove(&callback.session_id);

        if result.is_err() {
            self.state.auth_inboxes.remove(&channel_id_for_cleanup);
        }
        result
    }

    pub async fn connect_forward_anchor<E>(&self, session_id: String, context: &ContextSsh, handler: EventHandler<E>, force_share: Option<bool>) -> Result<(String, Arc<Handle<ClientSshHandler>>)>
    where
        E: SshConnectEvent,
    {
        let channel_id = uuid();
        let callback = Arc::new(SshOpenCallback::new(session_id, handler, SshChannelKind::Pty));
        self.session_id2_callback.insert(callback.session_id.clone(), callback.clone());

        self.auth_inbox(&channel_id);
        let channel_id_for_cleanup = channel_id.clone();
        let key = self.resolve_reuse_key(context, force_share);
        let result = (|| async {
            let handle = match &key {
                None => self.acquire_independent(&channel_id, context, None, &callback).await?,
                Some(key) => self.acquire_shared(&channel_id, context, None, &callback, key.clone()).await?,
            };
            self.state.channels.insert(channel_id.clone(), SshChannel { key, handle: handle.clone(), callback: (*callback).clone() });
            Ok::<_, anyhow::Error>((channel_id.clone(), handle))
        })()
        .await;
        self.session_id2_callback.remove(&callback.session_id);

        if result.is_err() {
            self.state.auth_inboxes.remove(&channel_id_for_cleanup);
        }
        result
    }

    pub(crate) fn register_forward(&self, key: (String, u16), sink: ForwardSink) {
        self.forward_registry.insert(key, sink);
    }

    pub(crate) fn deregister_forward(&self, key: &(String, u16)) {
        self.forward_registry.remove(key);
    }

    pub(crate) fn lookup_forward(&self, address: &str, port: u16) -> Option<ForwardSink> {
        self.forward_registry.get(&(address.to_string(), port)).map(|entry| entry.value().clone())
    }

    pub async fn close(&self, channel_id: &str) -> Result<()> {
        let Some((_, channel)) = self.state.channels.remove(channel_id) else {
            return Ok(());
        };
        self.state.auth_inboxes.remove(channel_id);
        let handle = Arc::clone(&channel.handle);
        let remaining_channel_count = self.count_channels_on_handle(&handle);
        match &channel.key {
            Some(key) => {
                debug!("[ssh] ssh 关闭 channel [{}]. [{}:{}] 当前 key 下剩余通道数: {}", channel_id, key.host, key.port, remaining_channel_count);
            }
            None => {
                debug!("[ssh] ssh 关闭 channel [{}]. 当前剩余通道数: {}", channel_id, remaining_channel_count);
            }
        }
        if remaining_channel_count > 0 {
            return Ok(());
        }
        if let Some(key) = &channel.key {
            self.forget_shared(key, &handle);
        }
        Self::disconnect_alive(&handle)?;
        match &channel.key {
            Some(key) => {
                info!("[ssh] ssh session [{}:{}] 已全部关闭, 无剩余通道", key.host, key.port);
            }
            None => {
                info!("[ssh] ssh session [channel_id={}] 已全部关闭, 无剩余通道", channel_id);
            }
        }
        Ok(())
    }

    fn resolve_reuse_key(&self, context: &ContextSsh, force_share: Option<bool>) -> Option<SshSessionKey> {
        match force_share {
            Some(true) => self.build_session_key(context),
            Some(false) => None,
            None => {
                if !context.config.read().share_session {
                    return None;
                }
                self.build_session_key(context)
            }
        }
    }

    fn build_session_key(&self, context: &ContextSsh) -> Option<SshSessionKey> {
        let config = context.config.read();
        let host = context.device.read().host.clone();
        SshSessionKey::from_config(&host, &config)
    }

    async fn acquire_shared(&self, channel_id: &str, context: &ContextSsh, trust_code: Option<i16>, callback: &SshOpenCallback, key: SshSessionKey) -> Result<Arc<Handle<ClientSshHandler>>> {
        loop {
            let shared = self.state.shared.entry(key.clone()).or_insert_with(|| Arc::new(SshSharedConn::new())).value().clone();
            let _guard = callback.wait_until_cancelled_or(|| async { Ok(shared.connect_lock.lock().await) }).await?;
            let still_current = self.state.shared.get(&key).map(|v| Arc::ptr_eq(v.value(), &shared)).unwrap_or(false);
            if !still_current {
                continue;
            }
            let cached = shared.handle.lock().clone();
            if let Some(handle) = cached {
                if !handle.is_closed() {
                    return Ok(handle);
                }
                shared.handle.lock().take();
            }
            let handle = match self.connect_and_auth(channel_id, context, trust_code, callback).await {
                Ok(v) => Arc::new(v),
                Err(e) => {
                    self.state.shared.remove_if(&key, |_, current| Arc::ptr_eq(current, &shared));
                    return Err(e);
                }
            };
            *shared.handle.lock() = Some(handle.clone());
            return Ok(handle);
        }
    }

    fn disconnect_alive(handle: &Arc<Handle<ClientSshHandler>>) -> Result<()> {
        if handle.is_closed() {
            return Ok(());
        }
        if let Some(result) = handle.disconnect(Disconnect::ByApplication, "", "").now_or_never() {
            result?;
        }
        Ok(())
    }

    pub(crate) async fn purge_closed(&self) {
        let closed: Vec<String> = self.state.channels.iter().filter(|entry| entry.value().handle.is_closed()).map(|entry| entry.key().clone()).collect();
        for channel_id in closed {
            let Some((_, channel)) = self.state.channels.remove(&channel_id) else {
                continue;
            };
            self.state.auth_inboxes.remove(&channel_id);
            info!("[ssh] ssh 会话已断开, 清理通道 [{}]", channel_id);
            channel.callback.emit_session_closed().await;
        }
        let dead_keys: Vec<SshSessionKey> =
            self.state.shared.iter().filter(|entry| entry.value().handle.lock().as_ref().map(|handle| handle.is_closed()).unwrap_or(false)).map(|entry| entry.key().clone()).collect();
        for key in dead_keys {
            let Some(shared) = self.state.shared.get(&key).map(|entry| entry.value().clone()) else {
                continue;
            };
            shared.handle.lock().take();
            self.state.shared.remove_if(&key, |_, current| Arc::ptr_eq(current, &shared));
            info!("[ssh] ssh session [{}:{}] 已断开, 清理复用缓存", key.host, key.port);
        }
    }

    async fn acquire_independent(&self, channel_id: &str, context: &ContextSsh, trust_code: Option<i16>, callback: &SshOpenCallback) -> Result<Arc<Handle<ClientSshHandler>>> {
        let handle = self.connect_and_auth(channel_id, context, trust_code, callback).await?;
        Ok(Arc::new(handle))
    }

    async fn connect_and_auth(&self, channel_id: &str, context: &ContextSsh, trust_code: Option<i16>, callback: &SshOpenCallback) -> Result<Handle<ClientSshHandler>> {
        callback.ensure_not_cancelled()?;
        callback.emit_open_step(RespSshOpenStepType::ProbeStart, 0.0, None).await;
        let host = context.device.read().host.clone();
        let base = context.config.read().get_base()?;
        let port = base.port;
        let seconds = base.connect_timeout_seconds.unwrap_or(DEFAULT_CONNECT_TIMEOUT_SECONDS).clamp(3, 120);

        callback.emit_open_step(RespSshOpenStepType::ProbeTcpConnecting, 10.0, None).await;
        callback.ensure_not_cancelled()?;
        let jump_device_id = context.config.read().jump_device_id.clone().filter(|id| !id.trim().is_empty());
        let stream = match jump_device_id {
            None => self.connect_tcp(&host, port, seconds, callback).await?,
            Some(jump_device_id) => {
                let visited = vec![context.device.read().id.clone()];
                self.connect_via_jump(&jump_device_id, &host, port, callback, visited).await?
            }
        };
        callback.emit_open_step(RespSshOpenStepType::ProbeTcpConnected, 20.0, None).await;
        callback.emit_open_step(RespSshOpenStepType::ProbeNegotiating, 30.0, None).await;
        debug!("[ssh] ssh [{}:{}] TCP 连接成功, 开始协商", host, port);
        let mut client_config = Config::default();
        if base.keepalive_interval_seconds > 0 {
            client_config.keepalive_interval = Some(Duration::from_secs(base.keepalive_interval_seconds));
        }

        callback.ensure_not_cancelled()?;
        let inbox = self.auth_inbox(channel_id);
        let ssh_handler = ClientSshHandler::new(
            port,
            host.clone(),
            channel_id.to_string(),
            context.clone(),
            trust_code,
            inbox,
            callback.clone(),
            self.known_host_service.clone(),
            self.state.auth_requests.clone(),
            self.clone(),
        );
        let client_config = Arc::new(client_config);
        let mut handle = match callback
            .wait_until_cancelled_or(|| async { client::connect_stream(client_config, stream, ssh_handler).await.map_err(|error| anyhow::anyhow!("SSH protocol handshake failed: {}", error)) })
            .await
        {
            Ok(handle) => handle,
            Err(error) => {
                let message = error.to_string();
                callback.emit_open_step(RespSshOpenStepType::ErrorProbeFailed, 30.0, Some(serde_json::Value::String(message))).await;
                return Err(error);
            }
        };

        callback.emit_open_step(RespSshOpenStepType::ProbeCompleted, 40.0, None).await;
        callback.emit_open_step(RespSshOpenStepType::AuthStart, 50.0, None).await;
        callback.ensure_not_cancelled()?;
        debug!("[ssh] ssh [{}:{}] 连接成功, 开始认证", host, port);
        match self.authenticate(channel_id, &mut handle, context, callback).await {
            Ok(_) => Ok(handle),
            Err(error) => {
                let message = error.to_string();
                callback.emit_open_step(RespSshOpenStepType::ErrorAuthFailed, callback.current_progress(), Some(serde_json::Value::String(message))).await;
                Err(error)
            }
        }
    }

    async fn connect_tcp(&self, host: &str, port: u16, seconds: u64, callback: &SshOpenCallback) -> Result<SshTransport> {
        let callback_for_tcp = callback.clone();
        let host_for_tcp = host.to_string();
        let stream = callback
            .wait_until_cancelled_or(|| async move {
                match timeout(Duration::from_secs(seconds), TcpStream::connect((host_for_tcp.as_str(), port))).await {
                    Ok(Ok(stream)) => Ok(stream),
                    Ok(Err(error)) => {
                        let message = format!("Failed to connect ssh {}:{}: {}", host_for_tcp, port, error);
                        callback_for_tcp.emit_open_step(RespSshOpenStepType::ErrorTcpFailed, 10.0, Some(serde_json::Value::String(message.clone()))).await;
                        Err(anyhow::anyhow!(message))
                    }
                    Err(_) => {
                        let message = format!("Failed to connect ssh {}:{}: connection timed out ({}s)", host_for_tcp, port, seconds);
                        callback_for_tcp.emit_open_step(RespSshOpenStepType::ErrorTcpFailed, 10.0, Some(serde_json::Value::String(message.clone()))).await;
                        Err(anyhow::anyhow!(message))
                    }
                }
            })
            .await?;
        let _ = stream.set_nodelay(true);
        Ok(SshTransport::Tcp(stream))
    }

    async fn connect_via_jump(&self, jump_device_id: &str, target_host: &str, target_port: u16, callback: &SshOpenCallback, visited: Vec<String>) -> Result<SshTransport> {
        let jump_context = self.build_jump_context(jump_device_id, &visited).await?;
        let jump_host = jump_context.device.read().host.clone();
        let handle = Box::pin(self.acquire_jump_handle(&jump_context, callback, visited)).await?;
        let seconds = jump_context.config.read().get_base()?.connect_timeout_seconds.unwrap_or(DEFAULT_CONNECT_TIMEOUT_SECONDS).clamp(3, 120);
        let channel = callback
            .wait_until_cancelled_or(|| async {
                timeout(Duration::from_secs(seconds), handle.channel_open_direct_tcpip(target_host.to_string(), target_port as u32, jump_host.clone(), 0))
                    .await
                    .map_err(|error| anyhow::anyhow!("Jump channel timed out via {} to {}:{}: {}", jump_host, target_host, target_port, error))?
                    .map_err(|error| anyhow::anyhow!("Failed to open jump channel via {} to {}:{}: {}", jump_host, target_host, target_port, error))
            })
            .await?;
        info!("[ssh] ssh 经跳板 [{}] 连接 [{}:{}]", jump_host, target_host, target_port);
        Ok(SshTransport::Jump(channel.into_stream()))
    }

    async fn acquire_jump_handle(&self, jump_context: &ContextSsh, callback: &SshOpenCallback, visited: Vec<String>) -> Result<Arc<Handle<ClientSshHandler>>> {
        let mut visited = visited;
        visited.push(jump_context.device.read().id.clone());
        let channel_id = uuid();
        self.auth_inbox(&channel_id);
        let key = self.resolve_reuse_key(jump_context, None);
        let result = match &key {
            None => self.acquire_independent(&channel_id, jump_context, None, callback).await,
            Some(key) => self.acquire_shared(&channel_id, jump_context, None, callback, key.clone()).await,
        };
        match result {
            Ok(handle) => {
                self.state.channels.insert(channel_id, SshChannel { key, handle: handle.clone(), callback: callback.clone() });
                Ok(handle)
            }
            Err(error) => {
                self.state.auth_inboxes.remove(&channel_id);
                Err(error)
            }
        }
    }

    async fn build_jump_context(&self, jump_device_id: &str, visited: &[String]) -> Result<ContextSsh> {
        if visited.iter().any(|id| id == jump_device_id) {
            anyhow::bail!("Jump host chain has a cycle at device {}", jump_device_id);
        }
        if visited.len() >= MAX_JUMP_DEPTH {
            anyhow::bail!("Jump host chain exceeds {} hops", MAX_JUMP_DEPTH);
        }
        let Some(device) = self.device_service.find_by_id(jump_device_id.to_string()).await? else {
            anyhow::bail!("Jump host device {} not found", jump_device_id);
        };
        let Some(config) = self.config_ssh_service.get_by_device_id(jump_device_id.to_string()) else {
            anyhow::bail!("Jump host device {} has no ssh config", jump_device_id);
        };
        Ok(ContextSsh { size: Default::default(), config, device })
    }

    fn count_channels_on_handle(&self, handle: &Arc<Handle<ClientSshHandler>>) -> usize {
        self.state.channels.iter().filter(|entry| Arc::ptr_eq(&entry.value().handle, handle)).count()
    }

    async fn open_channel(&self, context: &ContextSsh, kind: SshChannelKind, callback: &SshOpenCallback, handle: &Arc<Handle<ClientSshHandler>>) -> Result<Channel<Msg>> {
        let channel = match callback.wait_until_cancelled_or(|| async { handle.channel_open_session().await.map_err(anyhow::Error::from) }).await {
            Ok(channel) => channel,
            Err(error) => {
                let message = format!("SSH open channel failed: {}", error);
                callback.emit_open_step(RespSshOpenStepType::ErrorChannelFailed, callback.current_progress(), Some(serde_json::Value::String(message.clone()))).await;
                return Err(anyhow::anyhow!(message));
            }
        };
        let key_channel_count = self.count_channels_on_handle(handle) + 1;
        let request_result = callback
            .wait_until_cancelled_or(|| async {
                match kind {
                    SshChannelKind::Pty => self.request_pty_shell(&channel, context, key_channel_count).await,
                    SshChannelKind::Shell => self.request_shell(&channel, context, key_channel_count).await,
                    SshChannelKind::Sftp => self.request_sftp(&channel, context, key_channel_count).await,
                }
            })
            .await;
        if let Err(error) = request_result {
            let message = error.to_string();
            callback.emit_open_step(RespSshOpenStepType::ErrorChannelFailed, callback.current_progress(), Some(serde_json::Value::String(message.clone()))).await;
            return Err(anyhow::anyhow!(message));
        }
        Ok(channel)
    }

    async fn request_sftp(&self, channel: &Channel<Msg>, context: &ContextSsh, key_channel_count: usize) -> Result<()> {
        channel.request_subsystem(false, "sftp").await.map_err(|e| anyhow::anyhow!("SSH request sftp subsystem failed: {}", e))?;
        let config = context.config.read().clone();
        let host = context.device.read().host.clone();
        match config.get_base().ok() {
            Some(base) => {
                info!("[ssh] ssh sftp 打开 [{}:{}] 成功. 当前 key 下通道数: {}", host, base.port, key_channel_count);
            }
            None => {
                info!("[ssh] ssh sftp 打开 [{}] 成功. 当前 key 下通道数: {}", host, key_channel_count);
            }
        }
        Ok(())
    }

    async fn request_shell(&self, _channel: &Channel<Msg>, context: &ContextSsh, key_channel_count: usize) -> Result<()> {
        let config = context.config.read().clone();
        let host = context.device.read().host.clone();
        match config.get_base().ok() {
            Some(base) => {
                debug!("[ssh] ssh shell 打开 [{}:{}] 成功. 当前 key 下通道数: {}", host, base.port, key_channel_count);
            }
            None => {
                debug!("[ssh] ssh shell 打开 [{}] 成功. 当前 key 下通道数: {}", host, key_channel_count);
            }
        }
        Ok(())
    }

    async fn request_pty_shell(&self, channel: &Channel<Msg>, context: &ContextSsh, key_channel_count: usize) -> Result<()> {
        let size = context.size;
        let base = context.config.read().get_base()?;
        let term = if base.term.is_empty() { DEFAULT_SSH_TERM.to_string() } else { base.term };
        channel.request_pty(false, term.as_str(), size.cols as u32, size.rows as u32, 0, 0, &[]).await.map_err(|e| anyhow::anyhow!("SSH request pty failed: {}", e))?;
        channel.request_shell(false).await.map_err(|e| anyhow::anyhow!("SSH request shell failed: {}", e))?;
        let config = context.config.read().clone();
        let host = context.device.read().host.clone();

        match config.get_base().ok() {
            Some(base) => {
                info!("[ssh] ssh 打开 [{}:{}] 成功. 终端类型: {}, 窗口大小: {}x{}, 当前 key 下通道数: {}", host, base.port, term, context.size.cols, context.size.rows, key_channel_count);
            }
            None => {
                info!("[ssh] ssh 打开 [{}] 成功. 窗口大小: {}x{}, 当前 key 下通道数: {}", host, context.size.cols, context.size.rows, key_channel_count);
            }
        }
        Ok(())
    }

    fn forget_shared(&self, key: &SshSessionKey, handle: &Arc<Handle<ClientSshHandler>>) {
        let Some(shared) = self.state.shared.get(key).map(|v| v.value().clone()) else {
            return;
        };
        if shared.handle.lock().take_if(|current| Arc::ptr_eq(current, handle)).is_none() {
            return;
        }
        self.state.shared.remove_if(key, |_, current| Arc::ptr_eq(current, &shared));
    }

    fn auth_inbox(&self, channel_id: &str) -> Arc<SshAuthInbox> {
        self.state.auth_inboxes.entry(channel_id.to_string()).or_insert_with(|| Arc::new(SshAuthInbox::new())).clone()
    }

    pub(crate) async fn notify_auth_updated(&self, request_id: String, answer: SshAuthAnswer) -> Result<()> {
        if self.state.auth_requests.get(&request_id).is_none() {
            anyhow::bail!("SSH auth request not found request_id={}", request_id);
        }
        let fingerprint = matches!(answer, SshAuthAnswer::HostFingerprintReject | SshAuthAnswer::HostFingerprintTrustOnce | SshAuthAnswer::HostFingerprintTrustAlways);
        let pending: Vec<(String, Arc<SshAuthInbox>)> = if fingerprint {
            self.state.auth_requests.iter().map(|entry| (entry.key().clone(), entry.value().clone())).collect()
        } else {
            let inbox = self.state.auth_requests.get(&request_id).map(|entry| entry.value().clone()).ok_or_else(|| anyhow::anyhow!("SSH auth request not found request_id={}", request_id))?;
            vec![(request_id, inbox)]
        };
        for (id, inbox) in pending {
            inbox.queue.lock().await.push_back(SshAuthAnswerEntry { request_id: id, answer });
            inbox.notify.notify_waiters();
        }
        Ok(())
    }

    pub(crate) async fn prompt_and_wait_auth_answer(&self, call: &SshPrompt<'_>, progress: f32, timeout_ms: i64, expected: &[SshAuthAnswer]) -> Result<SshAuthAnswer> {
        if call.callback.interactive_forbidden() {
            anyhow::bail!("SSH shell open requires complete ConfigSsh, interactive prompt not allowed (step={})", i16::from(call.step_type));
        }
        let inbox = self.auth_inbox(call.channel_id);
        self.state.auth_requests.insert(call.request_id.to_string(), inbox);
        call.callback.emit_open_step(call.step_type, progress, Some(call.payload.clone())).await;
        let result = self.wait_next_auth_answer(call, timeout_ms, expected).await;
        self.state.auth_requests.remove(call.request_id);
        result
    }

    pub(crate) async fn prompt_auth_answer_or_reselect(&self, call: &SshPrompt<'_>, primary: SshAuthAnswer, progress: f32, can_reselect: bool) -> Result<SshAuthAnswer> {
        let mut expected = vec![primary];
        if can_reselect {
            expected.push(SshAuthAnswer::ReselectRequested);
        }
        self.prompt_and_wait_auth_answer(call, progress, 180_000, &expected).await
    }

    async fn wait_next_auth_answer(&self, call: &SshPrompt<'_>, timeout_ms: i64, expected: &[SshAuthAnswer]) -> Result<SshAuthAnswer> {
        let callback = call.callback;
        let channel_id = call.channel_id;
        let request_id = call.request_id;
        let inbox = self.auth_inbox(channel_id);
        loop {
            if callback.is_cancelled() {
                return Err(anyhow::anyhow!("SSH open cancelled"));
            }
            let notified = inbox.notify.notified();
            let cancelled = callback.cancel_notify.notified();

            tokio::pin!(notified);
            tokio::pin!(cancelled);
            notified.as_mut().enable();
            cancelled.as_mut().enable();

            if let Some(value) = {
                let mut guard = inbox.queue.lock().await;
                let pos = guard.iter().position(|entry| entry.request_id == request_id && expected.contains(&entry.answer));
                match pos {
                    Some(index) => guard.remove(index).map(|entry| entry.answer),
                    None => None,
                }
            } {
                return Ok(value);
            }
            if timeout_ms <= 0 {
                tokio::select! {
                    _ = &mut notified => {}
                    _ = &mut cancelled => {
                        return Err(anyhow::anyhow!("SSH open cancelled"));
                    }
                }
                continue;
            }
            let sleep = tokio::time::sleep(Duration::from_millis(timeout_ms as u64));
            tokio::pin!(sleep);
            tokio::select! {
                _ = &mut notified => {}
                _ = &mut cancelled => {
                    return Err(anyhow::anyhow!("SSH open cancelled"));
                }
                _ = &mut sleep => {
                    return Err(anyhow::anyhow!("SSH authentication prompt timed out"));
                }
            }
        }
    }
}
