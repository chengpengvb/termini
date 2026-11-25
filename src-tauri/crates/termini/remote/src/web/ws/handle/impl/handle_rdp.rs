use anyhow::anyhow;
use std::sync::Arc;
use dashmap::DashMap;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use tokio::sync::Mutex as AsyncMutex;
use summer::tracing::{debug, info, warn};
use crate::web::ws::ws_manager::WsManager;
use crate::service::service_rdp::RdpService;
use summer::plugin::component::ComponentRef;
use crate::module::rdp::client_rdp::ClientRdp;
use crate::service::service_base::BaseService;
use crate::base::base_app::{AppContext, BaseApp};
use summer_web::socketioxide::extract::SocketRef;
use crate::service::service_device::DeviceService;
use crate::model::req::req_ws_event::ReqWsEventRdp;
use crate::web::ws::handle::handle_base::HandleBase;
use crate::model::req::event::ws_ssh::AuthAnswerEvent;
use crate::model::req::req_ws_context::ReqSocketContext;
use crate::service::service_clipboard::ClipboardService;
use crate::module::common::client_terminal::EventHandler;
use crate::service::service_config_rdp::ConfigRdpService;
use crate::service::service_known_host::KnownHostService;
use crate::model::resp::event::resp_event_rdp::RespEventRdp;
use crate::module::common::context::context_rdp::ContextRdp;
use crate::module::common::dto::ssh_auth_input::SshAuthAnswer;
use crate::model::req::event::ws_rdp::{FocusEvent, FrameSyncEvent, InputEvent, OpenEvent, RdpInput, ResizeEvent, VisibilityEvent};

fn create_socket_id_2_session_locks() -> Arc<DashMap<String, Arc<DashMap<String, Arc<AsyncMutex<()>>>>>> {
    Arc::new(DashMap::new())
}

fn create_socket_id_2_session_id_2_client() -> Arc<DashMap<String, Arc<DashMap<String, Arc<ClientRdp>>>>> {
    Arc::new(DashMap::new())
}

#[base_app(sort = 906)]
#[derive(Clone, Service)]
pub struct HandleRdp {
    #[inject(component)]
    ws_manager: WsManager,

    #[inject(component)]
    device_service: DeviceService,

    #[inject(component)]
    clipboard_service: ClipboardService,

    #[inject(component)]
    config_rdp_service: ConfigRdpService,

    rdp_service: ComponentRef<RdpService>,

    known_host_service: ComponentRef<KnownHostService>,

    #[inject(func = create_socket_id_2_session_locks())]
    socket_id_2_session_locks: Arc<DashMap<String, Arc<DashMap<String, Arc<AsyncMutex<()>>>>>>,

    #[inject(func = create_socket_id_2_session_id_2_client())]
    socket_id_2_session_id_2_client: Arc<DashMap<String, Arc<DashMap<String, Arc<ClientRdp>>>>>,
}

#[async_trait]
impl BaseApp for HandleRdp {
    fn name(&self) -> &'static str {
        "HandleRdp"
    }
    async fn start(&self, _ctx: &AppContext) -> anyhow::Result<()> {
        let ref_self = Arc::new(self.clone());
        self.ws_manager.register(ref_self);
        Ok(())
    }
}

#[async_trait]
impl HandleBase for HandleRdp {
    fn get_name(&self) -> &'static str {
        "HandleRdp"
    }

    fn contains_type(&self, type_code: i16) -> bool {
        ReqWsEventRdp::form_number(type_code).is_some()
    }

    async fn handle_close(&self, socket: SocketRef) -> anyhow::Result<()> {
        let socket_id = socket.id.to_string();
        let clients = self.socket_id_2_session_id_2_client.remove(&socket_id).map(|(_, v)| v);
        let Some(clients) = clients else {
            return Ok(());
        };
        self.socket_id_2_session_locks.remove(&socket_id);

        let clients: Vec<Arc<ClientRdp>> = clients.iter().map(|e| Arc::clone(e.value())).collect();
        for client in clients {
            client.cancel_open();
            if let Err(e) = client.close().await {
                warn!("[rdp] Warning: failed to close client_rdp: {}", e);
            }
        }
        Ok(())
    }

    async fn handle_msg(&self, socket: SocketRef, req: Arc<ReqSocketContext>) -> anyhow::Result<()> {
        let Some(event) = ReqWsEventRdp::form_number(req.r#type) else {
            return Err(anyhow!("invalid event type: {}", req.r#type));
        };
        match event {
            ReqWsEventRdp::Open => {
                self.handle_open(socket, req).await?;
            }
            ReqWsEventRdp::Close => {
                self.handle_close(socket, req).await?;
            }
            ReqWsEventRdp::Input => {
                self.handle_input(socket, req).await?;
            }
            ReqWsEventRdp::Resize => {
                self.handle_resize(socket, req).await?;
            }
            ReqWsEventRdp::Visibility => {
                self.handle_visibility(socket, req).await?;
            }
            ReqWsEventRdp::AuthAnswer => {
                self.handle_auth_answer(socket, req).await?;
            }
            ReqWsEventRdp::FrameSync => {
                self.handle_frame_sync(socket, req).await?;
            }
            ReqWsEventRdp::Focus => {
                self.handle_focus(socket, req).await?;
            }
        }
        Ok(())
    }
}

impl HandleRdp {
    async fn handle_open(&self, socket: SocketRef, req: Arc<ReqSocketContext>) -> anyhow::Result<()> {
        let session_id = req.session_id.clone();
        if session_id.is_empty() {
            return Err(anyhow!("rdp open req session_id is empty"));
        }
        let socket_id = socket.id.to_string();
        let payload: OpenEvent = req.as_payload()?;
        let session_lock_map = self.socket_id_2_session_locks.entry(socket_id.clone()).or_insert_with(|| Arc::new(DashMap::new())).clone();
        let session_lock = session_lock_map.entry(session_id.clone()).or_insert_with(|| Arc::new(AsyncMutex::new(()))).clone();
        let _guard = session_lock.lock().await;

        let device_id = payload.data.device_id.clone();
        let device = self.device_service.find_by_id(device_id.clone()).await?.ok_or_else(|| anyhow!("device not found for device_id: {}", device_id))?;
        let config = self.config_rdp_service.get_by_device_id(device_id.clone()).ok_or_else(|| anyhow!("rdp config not found for device_id: {}", device_id))?;
        let context_rdp = Arc::new(ContextRdp { size: payload.data.size, config, device });

        let rdp_client = ClientRdp::new(self.known_host_service.clone(), self.clipboard_service.clone());
        let client_map = self.socket_id_2_session_id_2_client.entry(socket_id.clone()).or_insert_with(|| Arc::new(DashMap::new())).clone();
        if let Some(old_client) = client_map.insert(session_id.clone(), Arc::clone(&rdp_client)) {
            if let Err(e) = old_client.close().await {
                warn!("[rdp] Warning: failed to close previous rdp session {}: {}", session_id, e);
            }
        }

        let handler_req = Arc::clone(&req);
        let socket_for_handler = socket.clone();
        let rdp_service = self.rdp_service.clone();
        let handler: EventHandler<RespEventRdp> = Arc::new(move |event| {
            let socket = socket_for_handler.clone();
            let handler_req = Arc::clone(&handler_req);
            let rdp_service = rdp_service.clone();
            Box::pin(async move {
                rdp_service.handler_rdp_event(socket, handler_req, event);
                Ok(())
            })
        });

        match rdp_client.open(context_rdp, handler).await {
            Ok(()) => {
                if rdp_client.is_open_cancelled() {
                    if let Err(e) = rdp_client.close().await {
                        warn!("[rdp] Warning: failed to close cancelled rdp session {}: {}", session_id, e);
                    }
                    client_map.remove(&session_id);
                    session_lock_map.remove(&session_id);
                    if client_map.is_empty() {
                        self.socket_id_2_session_id_2_client.remove(&socket_id);
                        self.socket_id_2_session_locks.remove(&socket_id);
                    }
                    drop(_guard);
                    return Ok(());
                }
                drop(_guard);
                Ok(())
            }
            Err(error) => {
                client_map.remove(&session_id);
                let empty = client_map.is_empty();
                if empty {
                    session_lock_map.remove(&session_id);
                    if session_lock_map.is_empty() {
                        self.socket_id_2_session_id_2_client.remove(&socket_id);
                        self.socket_id_2_session_locks.remove(&socket_id);
                    }
                }
                drop(_guard);
                Err(error)
            }
        }
    }

    async fn handle_close(&self, _socket: SocketRef, req: Arc<ReqSocketContext>) -> anyhow::Result<()> {
        let session_id = req.session_id.clone();
        if session_id.is_empty() {
            return Err(anyhow!("rdp close req session_id is empty"));
        }
        let socket_id = _socket.id.to_string();

        if let Some(client_map) = self.socket_id_2_session_id_2_client.get(&socket_id) {
            if let Some(client) = client_map.get(&session_id) {
                client.cancel_open();
            }
        }

        let Some(session_lock_map) = self.socket_id_2_session_locks.get(&socket_id).map(|v| Arc::clone(v.value())) else {
            return Ok(());
        };
        let Some(session_lock) = session_lock_map.get(&session_id).map(|v| Arc::clone(v.value())) else {
            return Ok(());
        };
        let _guard = session_lock.lock().await;

        let Some(client_map) = self.socket_id_2_session_id_2_client.get(&socket_id).map(|v| Arc::clone(v.value())) else {
            session_lock_map.remove(&session_id);
            if session_lock_map.is_empty() {
                self.socket_id_2_session_locks.remove(&socket_id);
            }
            return Ok(());
        };
        let Some((_, client)) = client_map.remove(&session_id) else {
            session_lock_map.remove(&session_id);
            if client_map.is_empty() {
                self.socket_id_2_session_id_2_client.remove(&socket_id);
                self.socket_id_2_session_locks.remove(&socket_id);
            }
            return Ok(());
        };
        client.close().await?;

        session_lock_map.remove(&session_id);
        if client_map.is_empty() {
            self.socket_id_2_session_id_2_client.remove(&socket_id);
            self.socket_id_2_session_locks.remove(&socket_id);
        }
        Ok(())
    }

    async fn handle_input(&self, _socket: SocketRef, req: Arc<ReqSocketContext>) -> anyhow::Result<()> {
        let session_id = req.session_id.clone();
        if session_id.is_empty() {
            return Err(anyhow!("rdp input req session_id is empty"));
        }
        let socket_id = _socket.id.to_string();
        let payload: InputEvent = req.as_payload()?;

        let Some((session_lock, client)) = self.resolve_session(&socket_id, &session_id) else {
            warn!("[rdp] rdp 会话已失效, 丢弃输入 [{}]", session_id);
            return Ok(());
        };
        if let Some(summary) = Self::describe_input(&payload.data) {
            info!("[rdp] {} {}", session_id, summary);
        }
        let _guard = session_lock.lock().await;
        client.input(payload.data).await?;
        Ok(())
    }

    fn describe_input(events: &[RdpInput]) -> Option<String> {
        let parts: Vec<String> = events
            .iter()
            .filter_map(|event| match event {
                RdpInput::Mouse { x, y, button, pressed } => Some(format!("mouse(btn={} {} {},{})", button, if *pressed { "down" } else { "up" }, x, y)),
                RdpInput::Wheel { x, y, delta, horizontal } => Some(format!("wheel(delta={} h={} {},{})", delta, horizontal, x, y)),
                RdpInput::Scancode { code, pressed, .. } => Some(format!("key(0x{:02x} {})", code, if *pressed { "down" } else { "up" })),
                RdpInput::Unicode { code, pressed } => Some(format!("char(0x{:04x} {})", code, if *pressed { "down" } else { "up" })),
                _ => None,
            })
            .collect();
        if parts.is_empty() {
            return None;
        }
        Some(parts.join(" "))
    }

    async fn handle_resize(&self, _socket: SocketRef, req: Arc<ReqSocketContext>) -> anyhow::Result<()> {
        let session_id = req.session_id.clone();
        if session_id.is_empty() {
            return Err(anyhow!("rdp resize req session_id is empty"));
        }
        let socket_id = _socket.id.to_string();
        let payload: ResizeEvent = req.as_payload()?;
        let window_size = payload.data;

        let Some((session_lock, client)) = self.resolve_session(&socket_id, &session_id) else {
            debug!("[rdp] rdp 会话已失效, 丢弃窗口大小变更 [{}]", session_id);
            return Ok(());
        };
        let _guard = session_lock.lock().await;
        client.set_window_size(window_size).await?;
        Ok(())
    }

    async fn handle_visibility(&self, _socket: SocketRef, req: Arc<ReqSocketContext>) -> anyhow::Result<()> {
        let session_id = req.session_id.clone();
        if session_id.is_empty() {
            return Err(anyhow!("rdp visibility req session_id is empty"));
        }
        let socket_id = _socket.id.to_string();
        let payload: VisibilityEvent = req.as_payload()?;

        let Some((session_lock, client)) = self.resolve_session(&socket_id, &session_id) else {
            debug!("[rdp] rdp 会话已失效, 丢弃可见性变更 [{}]", session_id);
            return Ok(());
        };
        let _guard = session_lock.lock().await;
        client.suppress_output(payload.data).await?;
        Ok(())
    }

    async fn handle_auth_answer(&self, socket: SocketRef, req: Arc<ReqSocketContext>) -> anyhow::Result<()> {
        let session_id = req.session_id.clone();
        if session_id.is_empty() {
            return Err(anyhow!("rdp auth answer req session_id is empty"));
        }
        let socket_id = socket.id.to_string();
        let payload: AuthAnswerEvent = req.as_payload()?;
        let answer = SshAuthAnswer::from_code(payload.data.code).ok_or_else(|| anyhow!("rdp auth answer code invalid: {}", payload.data.code))?;
        let Some(client) = self.socket_id_2_session_id_2_client.get(&socket_id).and_then(|map| map.get(&session_id).map(|entry| Arc::clone(entry.value()))) else {
            return Err(anyhow!("rdp cert session not found: {}", session_id));
        };
        client.notify_cert_answer(payload.data.request_id, answer)?;
        Ok(())
    }

    async fn handle_frame_sync(&self, _socket: SocketRef, req: Arc<ReqSocketContext>) -> anyhow::Result<()> {
        let session_id = req.session_id.clone();
        if session_id.is_empty() {
            return Err(anyhow!("rdp frame sync req session_id is empty"));
        }
        let socket_id = _socket.id.to_string();
        let payload: FrameSyncEvent = req.as_payload()?;
        let Some((_, client)) = self.resolve_session(&socket_id, &session_id) else {
            debug!("[rdp] rdp 会话已失效, 丢弃帧同步 [{}]", session_id);
            return Ok(());
        };
        client.ack_frame(payload.data);
        Ok(())
    }

    async fn handle_focus(&self, socket: SocketRef, req: Arc<ReqSocketContext>) -> anyhow::Result<()> {
        let session_id = req.session_id.clone();
        if session_id.is_empty() {
            return Err(anyhow!("rdp focus req session_id is empty"));
        }
        let socket_id = socket.id.to_string();
        let payload: FocusEvent = req.as_payload()?;
        let Some((_, client)) = self.resolve_session(&socket_id, &session_id) else {
            debug!("[rdp] rdp 会话已失效, 丢弃焦点区变更 [{}]", session_id);
            return Ok(());
        };
        client.set_focus(payload.data)?;
        Ok(())
    }

    fn resolve_session(&self, socket_id: &str, session_id: &str) -> Option<(Arc<AsyncMutex<()>>, Arc<ClientRdp>)> {
        let session_lock = self.socket_id_2_session_locks.get(socket_id)?.get(session_id).map(|v| Arc::clone(v.value()))?;
        let client = self.socket_id_2_session_id_2_client.get(socket_id)?.get(session_id).map(|v| Arc::clone(v.value()))?;
        Some((session_lock, client))
    }
}
