use anyhow::anyhow;
use std::sync::Arc;
use dashmap::DashMap;
use tokio::runtime::Handle;
use tokio::io::AsyncWriteExt;
use summer::tracing::{info, warn};
use parking_lot::Mutex as SyncMutex;
use tokio::time::{timeout, Duration};
use crate::module::rdp::protocol::core::mcs;
use std::sync::atomic::{AtomicU8, Ordering};
use summer::plugin::component::ComponentRef;
use crate::module::rdp::protocol::core::disp;
use crate::module::rdp::protocol::core::input;
use crate::module::rdp::protocol::core::channel;
use crate::module::rdp::protocol::core::cliprdr;
use crate::module::rdp::protocol::core::drdynvc;
use crate::model::dto::clipboard::ClipboardEvent;
use crate::module::rdp::protocol::core::fastpath;
use crate::module::rdp::auth::auth_gate::AuthGate;
use crate::module::rdp::protocol::core::activation;
use crate::model::dto::cancel_session::CancelSession;
use crate::module::common::util::task_hub::TaskGroup;
use crate::module::rdp::protocol::core::capabilities;
use tokio::sync::{mpsc, oneshot, Mutex as AsyncMutex};
use crate::service::service_clipboard::ClipboardService;
use crate::module::common::client_terminal::EventHandler;
use crate::module::rdp::protocol::core::error::RdpResult;
use crate::module::rdp::protocol::gdi::framebuffer::Rect;
use crate::service::service_known_host::KnownHostService;
use crate::module::rdp::client_rdp_clipboard as clipboard;
use crate::model::req::event::ws_rdp::{RdpFocus, RdpInput};
use crate::module::common::context::context_rdp::ContextRdp;
use crate::module::common::context::context_base::WindowSize;
use crate::module::common::dto::ssh_auth_input::SshAuthAnswer;
use crate::module::rdp::protocol::core::drdynvc::DynamicChannels;
use crate::module::rdp::protocol::core::redirection::Redirection;
use crate::module::rdp::protocol::core::logon::AutoReconnectCookie;
use crate::module::rdp::client_session_factory::{ConnectedRdp, RdpSessionFactory};
use crate::model::resp::event::resp_event_rdp::{RespEventRdp, RespRdpOpenStepType};
use crate::module::rdp::protocol::core::cliprdr::{ClipboardState, FileEntry, LocalImage};
use crate::module::rdp::client_rdp_session::{desktop_monitors, RdpSession, RdpSettings, RdpState};
use crate::module::rdp::client_rdp_reader::{spawn_reader, FocusState, FrameSyncState, ReaderContext};

const EVENT_CHANNEL_CAPACITY: usize = 256;
const WORKER_JOIN_WAIT: Duration = Duration::from_secs(2);

#[derive(Clone)]
pub struct ClientRdp {
    focus: Arc<FocusState>,
    pub open_state: Arc<AtomicU8>,
    frame_sync: Arc<FrameSyncState>,
    open_cancel: Arc<CancelSession>,
    clipboard_service: ClipboardService,
    pub self_weak: std::sync::Weak<ClientRdp>,
    session: Arc<AsyncMutex<Option<Arc<RdpSession>>>>,
    known_host_service: ComponentRef<KnownHostService>,
    pub context: Arc<AsyncMutex<Option<Arc<ContextRdp>>>>,
    event_tx: Arc<SyncMutex<Option<mpsc::Sender<RespEventRdp>>>>,
    cert_answers: Arc<DashMap<String, oneshot::Sender<SshAuthAnswer>>>,
}

impl ClientRdp {
    pub fn new(known_host_service: ComponentRef<KnownHostService>, clipboard_service: ClipboardService) -> Arc<ClientRdp> {
        Arc::new_cyclic(|weak| Self {
            clipboard_service,
            known_host_service,
            self_weak: weak.clone(),
            focus: FocusState::new(),
            frame_sync: FrameSyncState::new(),
            cert_answers: Arc::new(DashMap::new()),
            open_state: Arc::new(AtomicU8::new(0)),
            session: Arc::new(AsyncMutex::new(None)),
            context: Arc::new(AsyncMutex::new(None)),
            event_tx: Arc::new(SyncMutex::new(None)),
            open_cancel: Arc::new(CancelSession::new()),
        })
    }

    pub fn notify_cert_answer(&self, request_id: String, answer: SshAuthAnswer) -> anyhow::Result<()> {
        let Some((_, tx)) = self.cert_answers.remove(&request_id) else {
            return Err(anyhow!("rdp cert request not found: {}", request_id));
        };
        tx.send(answer).map_err(|_| anyhow!("rdp cert waiter dropped"))?;
        Ok(())
    }

    pub fn set_focus(&self, focus: Option<RdpFocus>) -> anyhow::Result<()> {
        let rect = match focus {
            Some(focus) => Some(Rect::new(focus.left, focus.top, focus.right, focus.bottom).map_err(|error| anyhow!("{}", error))?),
            None => None,
        };
        self.focus.set(rect);
        Ok(())
    }

    pub fn ack_frame(&self, timestamp: u64) {
        let Some(bars) = self.frame_sync.ack(timestamp) else {
            return;
        };
        let Some(event_tx) = self.event_tx.lock().clone() else {
            return;
        };
        let _ = event_tx.try_send(RespEventRdp::OnNetwork(bars));
    }

    fn auth_gate(&self) -> AuthGate {
        AuthGate { answers: Arc::clone(&self.cert_answers), cancel: Arc::clone(&self.open_cancel), known_host: self.known_host_service.clone() }
    }

    pub fn cancel_open(&self) {
        self.open_cancel.cancel();
    }

    pub fn is_open_cancelled(&self) -> bool {
        self.open_cancel.is_cancelled()
    }

    pub async fn close(&self) -> anyhow::Result<()> {
        *self.event_tx.lock() = None;
        let session = {
            let mut guard = self.session.lock().await;
            guard.take()
        };
        if let Some(session) = session {
            let (channel, desktop) = {
                let guard = session.state.lock();
                (guard.channel, guard.desktop.clone())
            };
            let shutdown = activation::build_shutdown_request(channel.share_id, channel.user_id).and_then(|body| mcs::build_send_data_request(channel.user_id, channel.global_channel_id, &body));
            if let Some(close_tx) = session.close_tx.lock().take() {
                let _ = close_tx.send(());
            }
            if let Some(reader_shutdown_tx) = session.reader_shutdown_tx.lock().take() {
                let _ = reader_shutdown_tx.send(());
            }
            drop(session.clipboard_subscription.lock().take());
            session.tasks.shutdown_within(WORKER_JOIN_WAIT).await;
            let writer = Arc::clone(&session.writer);
            let _ = timeout(WORKER_JOIN_WAIT, async move {
                let mut writer = writer.lock().await;
                if let Ok(bytes) = shutdown {
                    let _ = writer.write_all(&bytes).await;
                }
                let _ = writer.shutdown().await;
            })
            .await;
            info!("[rdp] Rdp[{}x{}]关闭.", desktop.width, desktop.height);
        } else {
            info!("[rdp] Rdp关闭.");
        }
        *self.context.lock().await = None;
        self.open_state.store(0, Ordering::Release);
        Ok(())
    }

    pub async fn input(&self, events: Vec<RdpInput>) -> anyhow::Result<()> {
        let guard = self.session.lock().await;
        let Some(session) = guard.as_ref() else {
            return Ok(());
        };
        let events = fold_move_events(events);
        let (use_fastpath, channel, pdus) = {
            let mut state = session.state.lock();
            let input_flags = state.input_flags;
            let encoded = events.iter().map(|event| encode_input(event, input_flags)).collect::<RdpResult<Vec<Vec<u8>>>>()?;
            let use_fastpath = capabilities::fastpath_input(input_flags);
            let pdus =
                if use_fastpath { fastpath::build_input_pdus(&encoded, &mut state.security)? } else { input::build_slowpath_input_pdus(state.channel.share_id, state.channel.user_id, &encoded)? };
            (use_fastpath, state.channel, pdus)
        };
        if use_fastpath {
            let mut writer = session.writer.lock().await;
            for pdu in pdus {
                writer.write_all(&pdu).await.map_err(|error| anyhow!("Failed to write rdp input: {}", error))?;
            }
            writer.flush().await.map_err(|error| anyhow!("Failed to flush rdp stream: {}", error))?;
            return Ok(());
        }
        for pdu in pdus {
            Self::write_channel(session, channel.user_id, channel.global_channel_id, &pdu, "input").await?;
        }
        Ok(())
    }

    pub async fn set_window_size(&self, size: WindowSize) -> anyhow::Result<()> {
        let guard = self.session.lock().await;
        let Some(session) = guard.as_ref() else {
            return Ok(());
        };
        if !session.settings.dynamic_desktop {
            return Self::send_refresh(session).await;
        }
        let (channel, channel_id, packets) = {
            let state = session.state.lock();
            let Some(dynamic_id) = state.dynamic.channel_id(disp::CHANNEL_NAME) else {
                return Ok(());
            };
            if state.max_num_monitors == 0 {
                return Ok(());
            }
            let channel_id = state.channels.channel_id(channel::DRDYNVC_CHANNEL_NAME).ok_or_else(|| anyhow!("Dynamic channel is open without drdynvc"))?;
            let body = drdynvc::build_data(dynamic_id, &disp::build_monitor_layout(&desktop_monitors(size.pixel_width, size.pixel_height, size.dpi)?)?)?;
            (state.channel, channel_id, state.channels.split(channel_id, &body)?)
        };
        for packet in packets {
            Self::write_channel(session, channel.user_id, channel_id, &packet, "monitor layout").await?;
        }
        Ok(())
    }

    pub async fn suppress_output(&self, allow: bool) -> anyhow::Result<()> {
        let guard = self.session.lock().await;
        let Some(session) = guard.as_ref() else {
            return Ok(());
        };
        let (channel, desktop, suppress_output) = {
            let state = session.state.lock();
            (state.channel, state.desktop.clone(), state.suppress_output)
        };
        if !suppress_output {
            return Ok(());
        }
        let body = activation::build_suppress_output(channel.share_id, channel.user_id, allow, desktop.width, desktop.height)?;
        Self::write_channel(session, channel.user_id, channel.global_channel_id, &body, "suppress output").await
    }

    pub async fn set_clipboard(&self, text: String, files: Vec<FileEntry>, html: String, image: Option<LocalImage>) -> anyhow::Result<()> {
        let guard = self.session.lock().await;
        let Some(session) = guard.as_ref() else {
            return Ok(());
        };
        let (channel, channel_id, packets) = {
            let mut state = session.state.lock();
            let Some(channel_id) = state.channels.channel_id(channel::CLIPRDR_CHANNEL_NAME) else {
                return Ok(());
            };
            let Some(body) = cliprdr::set_local(&mut state.clipboard, text, files, html, image)? else {
                return Ok(());
            };
            let packets = state.channels.split(channel_id, &body)?;
            (state.channel, channel_id, packets)
        };
        for packet in packets {
            Self::write_channel(session, channel.user_id, channel_id, &packet, "clipboard").await?;
        }
        Ok(())
    }

    async fn send_refresh(session: &Arc<RdpSession>) -> anyhow::Result<()> {
        let (channel, desktop, refresh_rect) = {
            let state = session.state.lock();
            (state.channel, state.desktop.clone(), state.refresh_rect)
        };
        if !refresh_rect {
            return Ok(());
        }
        let body = activation::build_refresh_rect(channel.share_id, channel.user_id, desktop.width, desktop.height)?;
        Self::write_channel(session, channel.user_id, channel.global_channel_id, &body, "refresh").await
    }

    pub async fn open(&self, context: Arc<ContextRdp>, handler: EventHandler<RespEventRdp>) -> anyhow::Result<()> {
        self.open_session(context, handler, None).await
    }

    pub async fn redirect(&self, redirection: Redirection) -> anyhow::Result<()> {
        let Some((context, handler, auto_reconnect)) = self.take_redirect_target().await else {
            return Err(anyhow!("Session not found or already closed"));
        };
        self.close().await?;
        info!("[rdp] Rdp 会话重定向. 会话: 0x{:08x}", redirection.session_id);
        self.open_session(context, handler, Some((redirection, auto_reconnect))).await
    }

    async fn take_redirect_target(&self) -> Option<(Arc<ContextRdp>, EventHandler<RespEventRdp>, Option<AutoReconnectCookie>)> {
        let guard = self.session.lock().await;
        let session = guard.as_ref()?;
        let auto_reconnect = session.state.lock().auto_reconnect;
        Some((Arc::clone(&session.context), session.handler.clone(), auto_reconnect))
    }

    async fn open_session(&self, context: Arc<ContextRdp>, handler: EventHandler<RespEventRdp>, redirect: Option<(Redirection, Option<AutoReconnectCookie>)>) -> anyhow::Result<()> {
        if self.open_state.compare_exchange(0, 1, Ordering::AcqRel, Ordering::Acquire).is_err() {
            return Err(anyhow!("rdp already opened"));
        }
        {
            let mut guard = self.context.lock().await;
            *guard = Some(Arc::clone(&context));
        }
        let factory = RdpSessionFactory;
        let gate = self.auth_gate();
        if redirect.is_none() {
            if let Err(error) = factory.resolve_credentials(&context, &gate, &handler).await {
                self.open_state.store(0, Ordering::Release);
                *self.context.lock().await = None;
                return Err(anyhow!("{}", error));
            }
        }
        let mut built = RdpSettings::build(&context)?;
        if let Some((redirection, auto_reconnect)) = &redirect {
            built.apply_redirection(redirection, *auto_reconnect);
        }
        let settings = Arc::new(built);
        let (close_tx, close_rx) = oneshot::channel::<()>();
        let (event_tx, event_rx) = mpsc::channel::<RespEventRdp>(EVENT_CHANNEL_CAPACITY);
        self.frame_sync.reset_network();
        *self.event_tx.lock() = Some(event_tx.clone());
        let (reader_shutdown_tx, reader_shutdown_rx) = oneshot::channel::<()>();
        let tasks = TaskGroup::new();
        Self::spawn_dispatch(&tasks, handler.clone(), close_rx, event_rx);

        let connected = match factory.connect(&settings, Some(&context), &handler, Some(&gate)).await {
            Ok(connected) => connected,
            Err(error) => {
                *self.event_tx.lock() = None;
                self.open_state.store(0, Ordering::Release);
                *self.context.lock().await = None;
                tasks.abort();
                return Err(anyhow!("{}", error));
            }
        };
        let ConnectedRdp { transport, channel, channels, bulk, security, message, desktop, refresh_rect, suppress_output, frame_acknowledge, input_flags, multifragment_update } = connected;

        let dynamic = DynamicChannels::default();
        let (reader, writer_half) = transport.split();
        let max_num_monitors = disp::DEFAULT_MAX_NUM_MONITORS;
        let writer = Arc::new(AsyncMutex::new(writer_half));
        let (text, files, html, image) = match self.clipboard_service.latest() {
            Some(snapshot) => clipboard::from_snapshot(snapshot.as_ref()),
            None => (String::new(), Vec::new(), String::new(), None),
        };
        let mut clipboard = ClipboardState::new(text.clone());
        cliprdr::set_local(&mut clipboard, text, files, html, image).map_err(|error| anyhow!("{}", error))?;
        let state = Arc::new(SyncMutex::new(RdpState {
            channel,
            channels,
            dynamic,
            clipboard,
            refresh_rect,
            suppress_output,
            frame_acknowledge,
            input_flags,
            max_num_monitors,
            security,
            auto_reconnect: None,
            desktop: desktop.clone(),
        }));
        let session = Arc::new(RdpSession {
            tasks,
            handler: handler.clone(),
            state: Arc::clone(&state),
            writer: Arc::clone(&writer),
            context: Arc::clone(&context),
            settings: Arc::clone(&settings),
            close_tx: SyncMutex::new(Some(close_tx)),
            clipboard_subscription: SyncMutex::new(None),
            reader_shutdown_tx: SyncMutex::new(Some(reader_shutdown_tx)),
        });
        {
            let mut guard = self.session.lock().await;
            *guard = Some(Arc::clone(&session));
        }

        let client = self.self_weak.upgrade().expect("ClientRdp dropped");
        let runtime = Handle::current();
        let subscribe_client = Arc::clone(&client);
        let subscription = self.clipboard_service.subscribe(move |event| {
            let ClipboardEvent::Snapshot(snapshot) = event else {
                return;
            };
            let (text, files, html, image) = clipboard::from_snapshot(snapshot.as_ref());
            let client = Arc::clone(&subscribe_client);
            runtime.spawn(async move {
                if let Err(error) = client.set_clipboard(text, files, html, image).await {
                    warn!("[rdp] rdp 剪贴板同步到远端失败: {}", error);
                }
            });
        });
        *session.clipboard_subscription.lock() = Some(subscription);
        let reader_context = ReaderContext::new(
            Arc::clone(&settings),
            state,
            writer,
            event_tx,
            &desktop,
            bulk,
            message,
            multifragment_update,
            self.clipboard_service.clone(),
            Arc::clone(&self.frame_sync),
            Arc::clone(&self.focus),
            &session.tasks,
        );
        spawn_reader(Arc::clone(&client), reader, reader_context, reader_shutdown_rx, &session.tasks);

        let _ = handler(RespEventRdp::OnDesktopResize(desktop.clone())).await;
        factory.emit_open_step(&handler, RespRdpOpenStepType::SessionOpened, 100.0, None).await;
        let _ = handler(RespEventRdp::OnOpen).await;
        let (text, files, html, image) = match self.clipboard_service.latest() {
            Some(snapshot) => clipboard::from_snapshot(snapshot.as_ref()),
            None => (String::new(), Vec::new(), String::new(), None),
        };
        if let Err(error) = client.set_clipboard(text, files, html, image).await {
            warn!("[rdp] rdp 同步本机剪贴板失败: {}", error);
        }
        info!("[rdp] Rdp 打开 [{}:{}] 成功. 桌面: {}x{}, 色深: {}", settings.host, settings.port, desktop.width, desktop.height, settings.color_depth);
        Ok(())
    }

    async fn write_channel(session: &Arc<RdpSession>, user_id: u16, channel_id: u16, body: &[u8], action: &str) -> anyhow::Result<()> {
        let wrapped = session.state.lock().security.wrap(0, body)?;
        let bytes = mcs::build_send_data_request(user_id, channel_id, &wrapped)?;
        let mut writer = session.writer.lock().await;
        writer.write_all(&bytes).await.map_err(|error| anyhow!("Failed to write rdp {}: {}", action, error))?;
        writer.flush().await.map_err(|error| anyhow!("Failed to flush rdp stream: {}", error))?;
        Ok(())
    }

    fn spawn_dispatch(tasks: &TaskGroup, handler: EventHandler<RespEventRdp>, mut close_rx: oneshot::Receiver<()>, mut event_rx: mpsc::Receiver<RespEventRdp>) {
        tasks.spawn("rdp-dispatch", async move {
            loop {
                tokio::select! {
                    _ = &mut close_rx => {
                        let _ = handler(RespEventRdp::OnClose).await;
                        break;
                    }
                    maybe_event = event_rx.recv() => {
                        let Some(event) = maybe_event else {
                            break;
                        };
                        let _ = handler(event).await;
                    }
                }
            }
        });
    }
}

fn fold_move_events(events: Vec<RdpInput>) -> Vec<RdpInput> {
    let mut folded = Vec::with_capacity(events.len());
    for event in events {
        if matches!((folded.last(), &event), (Some(RdpInput::Move { .. }), RdpInput::Move { .. })) {
            let index = folded.len() - 1;
            folded[index] = event;
            continue;
        }
        folded.push(event);
    }
    folded
}

fn encode_input(event: &RdpInput, input_flags: u16) -> RdpResult<Vec<u8>> {
    if capabilities::fastpath_input(input_flags) {
        return match *event {
            RdpInput::Scancode { code, pressed, extended } => input::scancode_event(code, pressed, extended),
            RdpInput::Unicode { code, pressed } => input::unicode_event(code, pressed, input_flags),
            RdpInput::Mouse { x, y, button, pressed } => input::button_event(button, pressed, x, y, input_flags),
            RdpInput::Move { x, y } => Ok(input::move_event(x, y)),
            RdpInput::MoveRelative { x, y } => input::move_relative_event(x, y, input_flags),
            RdpInput::Wheel { x, y, delta, horizontal } => input::wheel_event(delta, horizontal, x, y, input_flags),
            RdpInput::Sync { scroll_lock, num_lock, caps_lock, kana_lock } => Ok(input::sync_event(scroll_lock, num_lock, caps_lock, kana_lock)),
        };
    }
    match *event {
        RdpInput::Scancode { code, pressed, extended } => input::slowpath_scancode_event(code, pressed, extended),
        RdpInput::Unicode { code, pressed } => input::slowpath_unicode_event(code, pressed, input_flags),
        RdpInput::Mouse { x, y, button, pressed } => input::slowpath_button_event(button, pressed, x, y, input_flags),
        RdpInput::Move { x, y } => Ok(input::slowpath_move_event(x, y)),
        RdpInput::MoveRelative { x, y } => input::slowpath_move_relative_event(x, y, input_flags),
        RdpInput::Wheel { x, y, delta, horizontal } => input::slowpath_wheel_event(delta, horizontal, x, y, input_flags),
        RdpInput::Sync { scroll_lock, num_lock, caps_lock, kana_lock } => Ok(input::slowpath_sync_event(scroll_lock, num_lock, caps_lock, kana_lock)),
    }
}
