use anyhow::anyhow;
use std::sync::Arc;
use tokio::net::TcpStream;
use async_trait::async_trait;
use std::collections::HashMap;
use summer::tracing::{info, warn};
use tokio::net::tcp::OwnedReadHalf;
use parking_lot::Mutex as SyncMutex;
use tokio::net::tcp::OwnedWriteHalf;
use tokio::time::{timeout, Duration};
use crate::module::telnet::core::command::*;
use std::sync::atomic::{AtomicU8, Ordering};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use crate::module::telnet::core::codec::TelnetCodec;
use crate::model::dto::cancel_session::CancelSession;
use crate::module::common::util::task_hub::TaskGroup;
use tokio::sync::{mpsc, oneshot, Mutex as AsyncMutex};
use crate::service::service_device_type::DeviceOsDetector;
use crate::module::common::context::context_base::WindowSize;
use crate::module::telnet::core::environ::encode_new_environ;
use crate::module::common::context::context_telnet::ContextTelnet;
use crate::module::telnet::keepalive::keepalive_nop::KeepaliveNop;
use crate::module::telnet::core::subnegotiation::encode_terminal_type_is;
use crate::module::telnet::keepalive::keepalive_tcp::apply_tcp_keepalive;
use crate::module::telnet::core::probe::{probe_telnet, TelnetProbeResult};
use crate::module::common::client_terminal::{ClientTerminal, EventHandler};
use crate::module::telnet::core::event::{TelnetEnvironmentValue, TelnetEvent};
use crate::module::telnet::auth::auth_login::{AuthLoginType, AuthState, TelnetAuthMeta};
use crate::module::telnet::gate::telnet_session_gate::{TelnetPermit, TelnetSessionGate, TELNET_GATE_WAIT};
use crate::model::resp::event::resp_event_telnet::{OpenTelnetStep, RespEventTelnet, RespTelnetOpenStepType};

const READ_BUFFER_SIZE: usize = 8192;
const DEFAULT_TERMINAL_TYPE: &str = "vt100";

struct TelnetSession {
    tasks: TaskGroup,
    permit: TelnetPermit,
    codec: Arc<AsyncMutex<TelnetCodec>>,
    writer: Arc<AsyncMutex<OwnedWriteHalf>>,
    keepalive_nop: Option<Arc<KeepaliveNop>>,
    close_tx: SyncMutex<Option<oneshot::Sender<()>>>,
    reader_shutdown_tx: SyncMutex<Option<oneshot::Sender<()>>>,
}

#[derive(Clone)]
pub struct ClientTelnet {
    pub open_state: Arc<AtomicU8>,
    open_cancel: Arc<CancelSession>,
    pub auth_meta: Arc<AsyncMutex<TelnetAuthMeta>>,
    pub self_weak: std::sync::Weak<ClientTelnet>,
    os_detector: Arc<AsyncMutex<DeviceOsDetector>>,
    session: Arc<AsyncMutex<Option<Arc<TelnetSession>>>>,
    pub context: Arc<AsyncMutex<Option<Arc<ContextTelnet>>>>,
}

#[async_trait]
impl ClientTerminal for ClientTelnet {
    type Event = RespEventTelnet;
    type Context = ContextTelnet;

    async fn close(&self) -> anyhow::Result<()> {
        let session = {
            let mut guard = self.session.lock().await;
            guard.take()
        };
        if let Some(session) = session {
            if let Some(keepalive_nop) = session.keepalive_nop.as_ref() {
                keepalive_nop.close().await;
            }
            if let Some(close_tx) = session.close_tx.lock().take() {
                let _ = close_tx.send(());
            }
            if let Some(reader_shutdown_tx) = session.reader_shutdown_tx.lock().take() {
                let _ = reader_shutdown_tx.send(());
            }
            session.tasks.shutdown_within(Duration::from_secs(2)).await;
            let writer = Arc::clone(&session.writer);
            let _ = timeout(Duration::from_secs(2), async move {
                let _ = writer.lock().await.shutdown().await;
            })
            .await;
            info!("[telnet] Telnet[{}]关闭.", session.permit.key());
        } else {
            info!("[telnet] Telnet关闭.");
        }
        *self.context.lock().await = None;
        self.open_state.store(0, Ordering::Release);
        Ok(())
    }

    async fn write(&self, data: String) -> anyhow::Result<()> {
        let guard = self.session.lock().await;
        let session = guard.as_ref().ok_or_else(|| anyhow!("Session not found or already closed"))?;
        let event = session.codec.lock().await.send_text(data.as_bytes());
        match event {
            TelnetEvent::Send(bytes) => {
                let mut writer = session.writer.lock().await;
                writer.write_all(&bytes).await.map_err(|e| anyhow!("Failed to write to telnet stream: {}", e))?;
                writer.flush().await.map_err(|e| anyhow!("Failed to flush telnet stream: {}", e))?;
                Ok(())
            }
            TelnetEvent::Error(error) => Err(anyhow!("{}", error.message)),
            _ => Ok(()),
        }
    }

    async fn set_window_size(&self, size: WindowSize) -> anyhow::Result<()> {
        let guard = self.session.lock().await;
        let session = guard.as_ref().ok_or_else(|| anyhow!("Session not found or already closed"))?;
        let body = Self::encode_naws_body(size);
        let events = session.codec.lock().await.subnegotiation(TELNET_TELOPT_NAWS, &body);
        let mut writer = session.writer.lock().await;
        for event in events {
            if let TelnetEvent::Send(bytes) = event {
                writer.write_all(&bytes).await.map_err(|e| anyhow!("Failed to write telnet window size: {}", e))?;
            }
        }
        writer.flush().await.map_err(|e| anyhow!("Failed to flush telnet stream: {}", e))?;
        Ok(())
    }

    async fn open(&self, context: Arc<Self::Context>, handler: EventHandler<Self::Event>) -> anyhow::Result<()> {
        if self.open_state.compare_exchange(0, 1, Ordering::AcqRel, Ordering::Acquire).is_err() {
            return Err(anyhow!("telnet already opened"));
        }
        {
            let mut guard = self.context.lock().await;
            *guard = Some(Arc::clone(&context));
        }
        let mut ttype_index = 0usize;
        let (close_tx, close_rx) = oneshot::channel::<()>();
        let environment = Self::build_environment(&context)?;
        let terminal_types = Self::build_terminal_types(&context)?;
        let (event_tx, event_rx) = mpsc::channel::<RespEventTelnet>(128);
        let (reader_shutdown_tx, reader_shutdown_rx) = oneshot::channel::<()>();
        let tasks = TaskGroup::new();
        Self::spawn_dispatch(&tasks, handler.clone(), close_rx, event_rx);
        let keepalive_interval_seconds = context.config.read().get_base()?.keepalive_interval_seconds;
        let permit = self.acquire_permit(&context).await?;
        let TelnetProbeResult { stream, codec, initial_data, .. } = self.connect_and_probe(&context, &handler).await?;
        let (codec, writer, reader) = self.setup_protocol(stream, codec, &context, keepalive_interval_seconds).await?;

        let keepalive_nop = (keepalive_interval_seconds > 0).then(|| Arc::new(KeepaliveNop::new(Arc::clone(&writer), keepalive_interval_seconds)));

        let session = Arc::new(TelnetSession { codec, writer, keepalive_nop, permit, tasks, close_tx: SyncMutex::new(Some(close_tx)), reader_shutdown_tx: SyncMutex::new(Some(reader_shutdown_tx)) });
        {
            let mut guard = self.session.lock().await;
            *guard = Some(Arc::clone(&session));
        }

        if !initial_data.is_empty() {
            let events = session.codec.lock().await.receive(&initial_data);
            self.handle_read_events(&mut ttype_index, &terminal_types, events, &event_tx, &environment, &session.writer, session.keepalive_nop.as_ref()).await;
        }
        let client = self.self_weak.upgrade().expect("ClientTelnet dropped");
        let writer = Arc::clone(&session.writer);
        let codec_reader = Arc::clone(&session.codec);
        Self::spawn_reader(&session.tasks, client, reader, &terminal_types, ttype_index, codec_reader, event_tx, reader_shutdown_rx, writer, environment, session.keepalive_nop.clone());

        Self::emit_open_step(&handler, RespTelnetOpenStepType::SessionOpened, 100.0, None).await;
        let _ = handler(RespEventTelnet::OnOpen).await;
        let config = context.config.read().clone();
        let host = context.device.read().host.clone();
        match config.get_base().ok() {
            Some(base) => {
                info!("[telnet] Telnet 打开 [{}:{}] 成功. 终端类型: {}, 窗口大小: {}x{}", host, base.port, terminal_types.join(","), context.size.cols, context.size.rows);
            }
            None => {
                info!("[telnet] Telnet 打开 [{}] 成功. 窗口大小: {}x{}", host, context.size.cols, context.size.rows);
            }
        }
        Ok(())
    }
}

impl ClientTelnet {
    pub fn new() -> Arc<ClientTelnet> {
        Arc::new_cyclic(|weak| Self {
            self_weak: weak.clone(),
            open_state: Arc::new(AtomicU8::new(0)),
            session: Arc::new(AsyncMutex::new(None)),
            context: Arc::new(AsyncMutex::new(None)),
            open_cancel: Arc::new(CancelSession::new()),
            os_detector: Arc::new(AsyncMutex::new(DeviceOsDetector::new())),
            auth_meta: Arc::new(AsyncMutex::new(TelnetAuthMeta { try_times: 0, pattern: None, echo: String::new(), auth_state: AuthState::None, login_type: AuthLoginType::NONE })),
        })
    }

    pub fn cancel_open(&self) {
        self.open_cancel.cancel();
    }

    pub fn is_open_cancelled(&self) -> bool {
        self.open_cancel.is_cancelled()
    }

    pub fn context(&self) -> Arc<AsyncMutex<Option<Arc<ContextTelnet>>>> {
        Arc::clone(&self.context)
    }

    /// 交互式开连接，名额被占满就等，等不到才报错
    async fn acquire_permit(&self, context: &ContextTelnet) -> anyhow::Result<TelnetPermit> {
        let (host, port) = Self::resolve_endpoint(context)?;
        let limit = context.config.read().resolve_max_sessions();
        TelnetSessionGate::global().acquire(&host, port, limit, TELNET_GATE_WAIT).await
    }

    async fn connect_and_probe(&self, context: &ContextTelnet, handler: &EventHandler<RespEventTelnet>) -> anyhow::Result<TelnetProbeResult> {
        Self::emit_open_step(handler, RespTelnetOpenStepType::ProbeStart, 0.0, None).await;
        let (host, port) = Self::resolve_endpoint(context)?;
        let connect_timeout_seconds = context.config.read().get_base()?.connect_timeout_seconds;
        Self::emit_open_step(handler, RespTelnetOpenStepType::ProbeTcpConnecting, 20.0, None).await;
        let stream = match Self::connect_tcp(host.as_str(), port, connect_timeout_seconds).await {
            Ok(stream) => stream,
            Err(message) => {
                Self::emit_open_step(handler, RespTelnetOpenStepType::ErrorTcpFailed, 20.0, Some(serde_json::Value::String(message.clone()))).await;
                return Err(anyhow!(message));
            }
        };
        Self::emit_open_step(handler, RespTelnetOpenStepType::ProbeTcpConnected, 40.0, None).await;
        Self::emit_open_step(handler, RespTelnetOpenStepType::ProbeNegotiating, 60.0, None).await;
        match probe_telnet(stream).await {
            Ok(result) => {
                let note = result.note.clone().map(serde_json::Value::String);
                Self::emit_open_step(handler, RespTelnetOpenStepType::ProbeCompleted, 80.0, note).await;
                Ok(result)
            }
            Err(error) => {
                let message = format!("Telnet probe failed: {}", error);
                Self::emit_open_step(handler, RespTelnetOpenStepType::ErrorProbeFailed, 60.0, Some(serde_json::Value::String(message.clone()))).await;
                Err(anyhow!(message))
            }
        }
    }

    async fn setup_protocol(
        &self,
        stream: TcpStream,
        codec: TelnetCodec,
        context: &ContextTelnet,
        keepalive_interval_seconds: u64,
    ) -> anyhow::Result<(Arc<AsyncMutex<TelnetCodec>>, Arc<AsyncMutex<OwnedWriteHalf>>, OwnedReadHalf)> {
        if keepalive_interval_seconds > 0 {
            let _ = apply_tcp_keepalive(&stream, keepalive_interval_seconds.saturating_mul(3));
        }
        let (reader, writer_half) = stream.into_split();
        let codec = Arc::new(AsyncMutex::new(codec));
        let writer = Arc::new(AsyncMutex::new(writer_half));
        if context.config.read().nvt_eol {
            let mut guard = codec.lock().await;
            let flags = guard.flags() | TELNET_FLAG_NVT_EOL;
            guard.set_flags(flags);
        }
        let eor_events = codec.lock().await.negotiate(TELNET_DO, TELNET_TELOPT_EOR);
        Self::write_send_events(&eor_events, &writer).await?;
        Self::push_initial_window_size(context, &codec, &writer).await?;
        Ok((codec, writer, reader))
    }

    async fn push_initial_window_size(context: &ContextTelnet, codec: &Arc<AsyncMutex<TelnetCodec>>, writer: &Arc<AsyncMutex<OwnedWriteHalf>>) -> anyhow::Result<()> {
        let size = context.size;
        if size.cols == 0 || size.rows == 0 {
            return Ok(());
        }
        let body = Self::encode_naws_body(size);
        let events = codec.lock().await.subnegotiation(TELNET_TELOPT_NAWS, &body);
        Self::write_send_events(&events, writer).await
    }

    async fn write_send_events(events: &[TelnetEvent], writer: &Arc<AsyncMutex<OwnedWriteHalf>>) -> anyhow::Result<()> {
        for event in events {
            if let TelnetEvent::Send(bytes) = event {
                writer.lock().await.write_all(bytes).await.map_err(|e| anyhow!("Failed to write to telnet stream: {}", e))?;
            }
        }
        Ok(())
    }

    async fn emit_open_step(handler: &EventHandler<RespEventTelnet>, step_type: RespTelnetOpenStepType, progress: f32, data: Option<serde_json::Value>) {
        let step = OpenTelnetStep { progress, step_type, data };
        let _ = handler(RespEventTelnet::OnOpenStep(step)).await;
    }

    fn resolve_endpoint(context: &ContextTelnet) -> anyhow::Result<(String, u16)> {
        let host = context.device.read().host.clone();
        let port = context.config.read().get_base()?.port;
        Ok((host, port))
    }

    async fn connect_tcp(host: &str, port: u16, connect_timeout_seconds: Option<u64>) -> Result<TcpStream, String> {
        let seconds = connect_timeout_seconds.unwrap_or_else(|| 3).clamp(3, 120);
        let connect_future = TcpStream::connect((host, port));
        match timeout(Duration::from_secs(seconds), connect_future).await {
            Ok(inner) => match inner {
                Ok(stream) => Ok(stream),
                Err(error) => Err(format!("Failed to connect telnet {}:{}: {}", host, port, error)),
            },
            Err(_) => Err(format!("Failed to connect telnet {}:{}: connection timed out ({}s)", host, port, seconds)),
        }
    }

    fn build_terminal_types(context: &ContextTelnet) -> anyhow::Result<Vec<String>> {
        let config = context.config.read();
        if let Some(types) = config.get_terminal_types()? {
            let named: Vec<String> = types.into_iter().filter(|name| !name.is_empty()).collect();
            if !named.is_empty() {
                return Ok(named);
            }
        }
        let term = config.get_base()?.term;
        if !term.is_empty() {
            return Ok(vec![term]);
        }
        Ok(vec![DEFAULT_TERMINAL_TYPE.to_string()])
    }

    fn build_environment(context: &ContextTelnet) -> anyhow::Result<Vec<TelnetEnvironmentValue>> {
        let config = context.config.read();
        let mut map: HashMap<String, String> = config.get_environment()?.unwrap_or_else(|| HashMap::new());
        if let Ok(base) = config.get_base() {
            if let Some(username) = base.username {
                if !username.is_empty() {
                    map.entry("USER".to_owned()).or_insert(username);
                }
            }
            if let Some(password) = base.password {
                if !password.is_empty() {
                    map.entry("PASSWORD".to_owned()).or_insert(password);
                }
            }
        }
        if map.is_empty() {
            return Ok(Vec::new());
        }
        let values = map.into_iter().map(|(key, value)| TelnetEnvironmentValue { kind: TELNET_ENVIRON_VAR, variable: key.into_bytes(), value: value.into_bytes() }).collect();
        Ok(values)
    }

    fn encode_naws_body(size: WindowSize) -> [u8; 4] {
        let cols = size.cols.to_be_bytes();
        let rows = size.rows.to_be_bytes();
        [cols[0], cols[1], rows[0], rows[1]]
    }

    fn spawn_reader(
        tasks: &TaskGroup,
        client: Arc<ClientTelnet>,
        mut reader: OwnedReadHalf,
        terminal_types: &[String],
        initial_ttype_index: usize,
        codec: Arc<AsyncMutex<TelnetCodec>>,
        event_tx: mpsc::Sender<RespEventTelnet>,
        mut shutdown_rx: oneshot::Receiver<()>,
        writer: Arc<AsyncMutex<OwnedWriteHalf>>,
        environment: Vec<TelnetEnvironmentValue>,
        keepalive_nop: Option<Arc<KeepaliveNop>>,
    ) {
        let terminal_types: Vec<String> = terminal_types.to_vec();
        tasks.spawn("telnet-reader", async move {
            let mut remote_closed = false;
            let mut buf = [0u8; READ_BUFFER_SIZE];
            let mut ttype_index = initial_ttype_index;
            loop {
                tokio::select! {
                    _ = &mut shutdown_rx => {
                        break;
                    }
                    result = reader.read(&mut buf) => {
                        match result {
                            Ok(0) => {
                                remote_closed = true;
                                break;
                            }
                            Ok(n) => {
                                let events = codec.lock().await.receive(&buf[..n]);
                                if !client.handle_read_events(&mut ttype_index, &terminal_types, events, &event_tx, &environment, &writer, keepalive_nop.as_ref()).await {
                                    remote_closed = true;
                                    break;
                                }
                            }
                            Err(e) => {
                                let _ = event_tx.send(RespEventTelnet::OnError(e.to_string())).await;
                                remote_closed = true;
                                break;
                            }
                        }
                    }
                }
            }
            if !remote_closed {
                return;
            }
            if let Err(error) = client.close().await {
                warn!("[telnet] telnet 连接断开后清理失败: {}", error);
            }
        });
    }

    async fn handle_read_events(
        &self,
        ttype_index: &mut usize,
        terminal_types: &[String],
        events: Vec<TelnetEvent>,
        event_tx: &mpsc::Sender<RespEventTelnet>,
        environment: &[TelnetEnvironmentValue],
        writer: &Arc<AsyncMutex<OwnedWriteHalf>>,
        keepalive_nop: Option<&Arc<KeepaliveNop>>,
    ) -> bool {
        for event in events {
            match event {
                TelnetEvent::Data(bytes) => {
                    self.try_login(bytes.clone()).await;
                    self.detect_device_os(&bytes, event_tx).await;
                    if event_tx.send(RespEventTelnet::OnData(bytes)).await.is_err() {
                        return false;
                    }
                }
                TelnetEvent::Send(bytes) => {
                    if let Err(error) = writer.lock().await.write_all(&bytes).await {
                        let _ = event_tx.send(RespEventTelnet::OnError(error.to_string())).await;
                        return false;
                    }
                }
                TelnetEvent::Will(telopt) => {
                    if telopt == TELNET_TELOPT_SGA {
                        if let Some(keepalive_nop) = keepalive_nop {
                            keepalive_nop.start_job();
                        }
                    }
                }
                TelnetEvent::TerminalType { command, .. } if command == TELNET_TTYPE_SEND => {
                    Self::respond_terminal_type(writer, terminal_types, ttype_index).await;
                }
                TelnetEvent::Environ { command, .. } if command == TELNET_ENVIRON_SEND => {
                    Self::respond_environ(writer, environment).await;
                }
                TelnetEvent::Error(error) => {
                    if event_tx.send(RespEventTelnet::OnError(error.message)).await.is_err() {
                        return false;
                    }
                }
                _ => {}
            }
        }
        true
    }

    async fn detect_device_os(&self, data: &[u8], event_tx: &mpsc::Sender<RespEventTelnet>) {
        if !self.auth_meta.lock().await.is_credential_sent() {
            return;
        }
        let mut detector = self.os_detector.lock().await;
        detector.start();
        let Some(os_type) = detector.observe(data) else {
            return;
        };
        let _ = event_tx.send(RespEventTelnet::OnDeviceOsType(os_type.as_i16())).await;
    }

    async fn respond_terminal_type(writer: &Arc<AsyncMutex<OwnedWriteHalf>>, terminal_types: &[String], ttype_index: &mut usize) {
        if terminal_types.is_empty() {
            return;
        }
        let last = terminal_types.len() - 1;
        let index = (*ttype_index).min(last);
        let name = terminal_types[index].clone();
        if *ttype_index < last {
            *ttype_index += 1;
        }
        let bytes = encode_terminal_type_is(name.as_bytes());
        let _ = writer.lock().await.write_all(&bytes).await;
    }

    async fn respond_environ(writer: &Arc<AsyncMutex<OwnedWriteHalf>>, environment: &[TelnetEnvironmentValue]) {
        if environment.is_empty() {
            return;
        }
        let keys: Vec<String> = environment.iter().filter_map(|item| String::from_utf8(item.variable.clone()).ok()).collect();
        info!("[telnet] Telnet 响应 NEW-ENVIRON SEND，发送变量: {}", keys.join(", "));
        let bytes = encode_new_environ(TELNET_ENVIRON_IS, environment);
        let _ = writer.lock().await.write_all(&bytes).await;
    }

    fn spawn_dispatch(tasks: &TaskGroup, handler: EventHandler<RespEventTelnet>, mut close_rx: oneshot::Receiver<()>, mut event_rx: mpsc::Receiver<RespEventTelnet>) {
        tasks.spawn("telnet-dispatch", async move {
            loop {
                tokio::select! {
                    _ = &mut close_rx => {
                        let _ = handler(RespEventTelnet::OnClose).await;
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
