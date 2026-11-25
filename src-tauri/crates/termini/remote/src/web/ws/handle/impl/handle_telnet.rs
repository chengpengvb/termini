use anyhow::anyhow;
use std::sync::Arc;
use dashmap::DashMap;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use summer::tracing::{debug, warn};
use tokio::sync::Mutex as AsyncMutex;
use crate::web::ws::ws_manager::WsManager;
use summer::plugin::component::ComponentRef;
use crate::service::service_base::BaseService;
use crate::base::base_app::{AppContext, BaseApp};
use summer_web::socketioxide::extract::SocketRef;
use crate::service::service_device::DeviceService;
use crate::service::service_telnet::TelnetService;
use crate::web::ws::handle::handle_base::HandleBase;
use crate::model::req::req_ws_event::ReqWsEventTelnet;
use crate::module::telnet::client_telnet::ClientTelnet;
use crate::model::req::req_ws_context::ReqSocketContext;
use crate::service::service_config_telnet::ConfigTelnetService;
use crate::service::service_session_record::SessionRecordService;
use crate::model::resp::event::resp_event_telnet::RespEventTelnet;
use crate::module::common::context::context_telnet::ContextTelnet;
use crate::module::common::client_terminal::{ClientTerminal, EventHandler};
use crate::model::req::event::ws_telnet::{OpenEvent, ResizeEvent, WriteEvent};

fn create_socket_id_2_session_locks() -> Arc<DashMap<String, Arc<DashMap<String, Arc<AsyncMutex<()>>>>>> {
    Arc::new(DashMap::new())
}

fn create_socket_id_2_session_id_2_client() -> Arc<DashMap<String, Arc<DashMap<String, Arc<ClientTelnet>>>>> {
    Arc::new(DashMap::new())
}

#[base_app(sort = 903)]
#[derive(Clone, Service)]
pub struct HandleTelnet {
    #[inject(component)]
    ws_manager: WsManager,

    #[inject(component)]
    device_service: DeviceService,

    #[inject(component)]
    config_telnet_service: ConfigTelnetService,

    telnet_service: ComponentRef<TelnetService>,

    #[inject(component)]
    session_record_service: SessionRecordService,

    #[inject(func = create_socket_id_2_session_locks())]
    socket_id_2_session_locks: Arc<DashMap<String, Arc<DashMap<String, Arc<AsyncMutex<()>>>>>>,

    #[inject(func = create_socket_id_2_session_id_2_client())]
    socket_id_2_session_id_2_client: Arc<DashMap<String, Arc<DashMap<String, Arc<ClientTelnet>>>>>,
}

#[async_trait]
impl BaseApp for HandleTelnet {
    fn name(&self) -> &'static str {
        "HandleTelnet"
    }
    async fn start(&self, _ctx: &AppContext) -> anyhow::Result<()> {
        let ref_self = Arc::new(self.clone());
        self.ws_manager.register(ref_self);
        Ok(())
    }
}

#[async_trait]
impl HandleBase for HandleTelnet {
    fn get_name(&self) -> &'static str {
        "HandleTelnet"
    }

    fn contains_type(&self, type_code: i16) -> bool {
        ReqWsEventTelnet::form_number(type_code).is_some()
    }

    async fn handle_close(&self, socket: SocketRef) -> anyhow::Result<()> {
        let socket_id = socket.id.to_string();
        let clients = self.socket_id_2_session_id_2_client.remove(&socket_id).map(|(_, v)| v);
        let Some(clients) = clients else {
            return Ok(());
        };
        self.socket_id_2_session_locks.remove(&socket_id);

        let clients: Vec<Arc<ClientTelnet>> = clients.iter().map(|e| Arc::clone(e.value())).collect();
        for client in clients {
            client.cancel_open();
            if let Err(e) = client.close().await {
                warn!("[telnet] Warning: failed to close client_telnet: {}", e);
            }
        }
        Ok(())
    }

    async fn handle_msg(&self, socket: SocketRef, req: Arc<ReqSocketContext>) -> anyhow::Result<()> {
        let Some(event) = ReqWsEventTelnet::form_number(req.r#type) else {
            return Err(anyhow!("invalid event type: {}", req.r#type));
        };
        match event {
            ReqWsEventTelnet::Open => {
                self.handle_open(socket, req).await?;
            }
            ReqWsEventTelnet::Close => {
                self.handle_close(socket, req).await?;
            }
            ReqWsEventTelnet::Write => {
                self.handle_write(socket, req).await?;
            }
            ReqWsEventTelnet::Resize => {
                self.handle_resize(socket, req).await?;
            }
        }
        Ok(())
    }
}

impl HandleTelnet {
    async fn handle_open(&self, socket: SocketRef, req: Arc<ReqSocketContext>) -> anyhow::Result<()> {
        let session_id = req.session_id.clone();
        if session_id.is_empty() {
            return Err(anyhow!("telnet open req session_id is empty"));
        }
        let socket_id = socket.id.to_string();
        let payload: OpenEvent = req.as_payload()?;
        let session_lock_map = self.socket_id_2_session_locks.entry(socket_id.clone()).or_insert_with(|| Arc::new(DashMap::new())).clone();
        let session_lock = session_lock_map.entry(session_id.clone()).or_insert_with(|| Arc::new(AsyncMutex::new(()))).clone();
        let _guard = session_lock.lock().await;

        let device_id = payload.data.device_id.clone();
        let device = self.device_service.find_by_id(device_id.clone()).await?.ok_or_else(|| anyhow!("device not found for device_id: {}", device_id))?;
        let config = self.config_telnet_service.get_by_device_id(device_id.clone()).ok_or_else(|| anyhow!("telnet config not found for device_id: {}", device_id))?;
        let context_telnet = Arc::new(ContextTelnet { size: payload.data.size, config, device });

        let telnet_client = Arc::new(ClientTelnet::new());
        let client_map = self.socket_id_2_session_id_2_client.entry(socket_id.clone()).or_insert_with(|| Arc::new(DashMap::new())).clone();
        if let Some(old_client) = client_map.insert(session_id.clone(), Arc::clone(&telnet_client)) {
            if let Err(e) = old_client.close().await {
                warn!("[telnet] Warning: failed to close previous telnet session {}: {}", session_id, e);
            }
        }

        let handler_req = Arc::clone(&req);
        let socket_for_handler = socket.clone();
        let context_handle = telnet_client.context();
        let telnet_service = self.telnet_service.clone();
        let record_size = payload.data.size;
        let record_device_name = context_telnet.device.read().name.clone();
        let session_record_service = self.session_record_service.clone();
        let handler: EventHandler<RespEventTelnet> = Arc::new(move |event| {
            let socket = socket_for_handler.clone();
            let handler_req = Arc::clone(&handler_req);
            let telnet_service = telnet_service.clone();
            let context_handle = Arc::clone(&context_handle);
            let record_device_name = record_device_name.clone();
            let session_record_service = session_record_service.clone();
            Box::pin(async move {
                match &event {
                    RespEventTelnet::OnOpen => session_record_service.start(handler_req.session_id.as_str(), "telnet", record_device_name.as_str(), record_size.cols, record_size.rows).await,
                    RespEventTelnet::OnData(data) => session_record_service.append(handler_req.session_id.as_str(), data),
                    RespEventTelnet::OnClose => session_record_service.stop(handler_req.session_id.as_str()).await,
                    _ => {}
                }
                telnet_service.handler_telnet_event(socket, handler_req, context_handle, event);
                Ok(())
            })
        });

        match telnet_client.open(context_telnet, handler).await {
            Ok(()) => {
                if telnet_client.is_open_cancelled() {
                    if let Err(e) = telnet_client.close().await {
                        warn!("[telnet] Warning: failed to close cancelled telnet session {}: {}", session_id, e);
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
            return Err(anyhow!("telnet close req session_id is empty"));
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

    async fn handle_write(&self, _socket: SocketRef, req: Arc<ReqSocketContext>) -> anyhow::Result<()> {
        let session_id = req.session_id.clone();
        if session_id.is_empty() {
            return Err(anyhow!("telnet write req session_id is empty"));
        }
        let socket_id = _socket.id.to_string();
        let payload: WriteEvent = req.as_payload()?;
        let data = payload.data;

        let Some((session_lock, client)) = self.resolve_session(&socket_id, &session_id) else {
            debug!("[telnet] telnet 会话已失效, 丢弃写入 [{}]", session_id);
            return Ok(());
        };
        let _guard = session_lock.lock().await;
        self.session_record_service.mark_input(session_id.as_str(), data.as_str());
        client.write(data).await?;
        Ok(())
    }

    async fn handle_resize(&self, _socket: SocketRef, req: Arc<ReqSocketContext>) -> anyhow::Result<()> {
        let session_id = req.session_id.clone();
        if session_id.is_empty() {
            return Err(anyhow!("telnet resize req session_id is empty"));
        }
        let socket_id = _socket.id.to_string();
        let payload: ResizeEvent = req.as_payload()?;
        let window_size = payload.data;

        let Some((session_lock, client)) = self.resolve_session(&socket_id, &session_id) else {
            debug!("[telnet] telnet 会话已失效, 丢弃窗口大小变更 [{}]", session_id);
            return Ok(());
        };
        let _guard = session_lock.lock().await;
        client.set_window_size(window_size).await?;
        Ok(())
    }

    fn resolve_session(&self, socket_id: &str, session_id: &str) -> Option<(Arc<AsyncMutex<()>>, Arc<ClientTelnet>)> {
        let session_lock = self.socket_id_2_session_locks.get(socket_id)?.get(session_id).map(|v| Arc::clone(v.value()))?;
        let client = self.socket_id_2_session_id_2_client.get(socket_id)?.get(session_id).map(|v| Arc::clone(v.value()))?;
        Some((session_lock, client))
    }
}
