use anyhow::anyhow;
use std::sync::Arc;
use dashmap::DashMap;
use summer::tracing::warn;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use tokio::sync::Mutex as AsyncMutex;
use crate::web::ws::ws_manager::WsManager;
use crate::module::serial::available_ports;
use summer::plugin::component::ComponentRef;
use crate::base::base_app::{AppContext, BaseApp};
use summer_web::socketioxide::extract::SocketRef;
use crate::service::service_serial::SerialService;
use crate::web::ws::handle::handle_base::HandleBase;
use crate::model::req::req_ws_event::ReqWsEventSerial;
use crate::module::serial::client_serial::ClientSerial;
use crate::model::req::req_ws_context::ReqSocketContext;
use crate::module::common::client_terminal::EventHandler;
use crate::service::service_config_serial::ConfigSerialService;
use crate::model::req::event::ws_serial::{OpenEvent, WriteEvent};
use crate::service::service_session_record::SessionRecordService;
use crate::module::common::context::context_serial::ContextSerial;
use crate::model::resp::event::resp_event_serial::{RespEventSerial, RespEventSerialPorts};

fn create_socket_id_2_session_locks() -> Arc<DashMap<String, Arc<DashMap<String, Arc<AsyncMutex<()>>>>>> {
    Arc::new(DashMap::new())
}

fn create_socket_id_2_session_id_2_client() -> Arc<DashMap<String, Arc<DashMap<String, Arc<ClientSerial>>>>> {
    Arc::new(DashMap::new())
}

#[base_app(sort = 904)]
#[derive(Clone, Service)]
pub struct HandleSerial {
    #[inject(component)]
    ws_manager: WsManager,

    #[inject(component)]
    config_serial_service: ConfigSerialService,

    #[inject(component)]
    session_record_service: SessionRecordService,

    serial_service: ComponentRef<SerialService>,

    #[inject(func = create_socket_id_2_session_locks())]
    socket_id_2_session_locks: Arc<DashMap<String, Arc<DashMap<String, Arc<AsyncMutex<()>>>>>>,

    #[inject(func = create_socket_id_2_session_id_2_client())]
    socket_id_2_session_id_2_client: Arc<DashMap<String, Arc<DashMap<String, Arc<ClientSerial>>>>>,
}

#[async_trait]
impl BaseApp for HandleSerial {
    fn name(&self) -> &'static str {
        "HandleSerial"
    }
    async fn start(&self, _ctx: &AppContext) -> anyhow::Result<()> {
        let ref_self = Arc::new(self.clone());
        self.ws_manager.register(ref_self);
        self.watch_ports();
        Ok(())
    }
}

#[async_trait]
impl HandleBase for HandleSerial {
    fn get_name(&self) -> &'static str {
        "HandleSerial"
    }

    fn contains_type(&self, type_code: i16) -> bool {
        ReqWsEventSerial::form_number(type_code).is_some()
    }

    async fn handle_close(&self, socket: SocketRef) -> anyhow::Result<()> {
        let socket_id = socket.id.to_string();
        let clients = self.socket_id_2_session_id_2_client.remove(&socket_id).map(|(_, v)| v);
        let Some(clients) = clients else {
            return Ok(());
        };
        self.socket_id_2_session_locks.remove(&socket_id);

        let clients: Vec<Arc<ClientSerial>> = clients.iter().map(|e| Arc::clone(e.value())).collect();
        for client in clients {
            if let Err(e) = client.close().await {
                warn!("[serial] Warning: failed to close client_serial: {}", e);
            }
        }
        Ok(())
    }

    async fn handle_msg(&self, socket: SocketRef, req: Arc<ReqSocketContext>) -> anyhow::Result<()> {
        let Some(event) = ReqWsEventSerial::form_number(req.r#type) else {
            return Err(anyhow!("invalid event type: {}", req.r#type));
        };
        match event {
            ReqWsEventSerial::Open => {
                self.handle_open(socket, req).await?;
            }
            ReqWsEventSerial::Close => {
                self.handle_close(socket, req).await?;
            }
            ReqWsEventSerial::Write => {
                self.handle_write(socket, req).await?;
            }
        }
        Ok(())
    }
}

impl HandleSerial {
    /// 轮询串口热插拔：端口集合变化时广播给前端，无需重启应用
    fn watch_ports(&self) {
        let ws_manager = self.ws_manager.clone();
        tokio::spawn(async move {
            let mut last = available_ports();
            last.sort();
            let mut ticker = tokio::time::interval(std::time::Duration::from_secs(2));
            ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            loop {
                ticker.tick().await;
                let mut current = available_ports();
                current.sort();
                if current == last {
                    continue;
                }
                last = current.clone();
                ws_manager.broadcast_all(&RespEventSerialPorts::OnChanged(current).to_resp());
            }
        });
    }

    async fn handle_open(&self, socket: SocketRef, req: Arc<ReqSocketContext>) -> anyhow::Result<()> {
        let session_id = req.session_id.clone();
        if session_id.is_empty() {
            return Err(anyhow!("serial open req session_id is empty"));
        }
        let socket_id = socket.id.to_string();
        let payload: OpenEvent = req.as_payload()?;
        let session_lock_map = self.socket_id_2_session_locks.entry(socket_id.clone()).or_insert_with(|| Arc::new(DashMap::new())).clone();
        let session_lock = session_lock_map.entry(session_id.clone()).or_insert_with(|| Arc::new(AsyncMutex::new(()))).clone();
        let _guard = session_lock.lock().await;

        let port_name = payload.data.port_name.clone();
        let config = self.config_serial_service.get_by_port_name(port_name.as_str()).ok_or_else(|| anyhow!("serial config not found for port: {}", port_name))?;
        let context_serial = Arc::new(ContextSerial { config });

        let serial_client = Arc::new(ClientSerial::new());
        let client_map = self.socket_id_2_session_id_2_client.entry(socket_id.clone()).or_insert_with(|| Arc::new(DashMap::new())).clone();
        if let Some(old_client) = client_map.insert(session_id.clone(), Arc::clone(&serial_client)) {
            if let Err(e) = old_client.close().await {
                warn!("[serial] Warning: failed to close previous serial session {}: {}", session_id, e);
            }
        }

        let handler_req = Arc::clone(&req);
        let socket_for_handler = socket.clone();
        let serial_service = self.serial_service.clone();
        let record_size = payload.data.size;
        let record_device_name = port_name.clone();
        let session_record_service = self.session_record_service.clone();
        let handler: EventHandler<RespEventSerial> = Arc::new(move |event| {
            let socket = socket_for_handler.clone();
            let handler_req = Arc::clone(&handler_req);
            let serial_service = serial_service.clone();
            let record_device_name = record_device_name.clone();
            let session_record_service = session_record_service.clone();
            Box::pin(async move {
                match &event {
                    RespEventSerial::OnOpen => session_record_service.start(handler_req.session_id.as_str(), "serial", record_device_name.as_str(), record_size.cols, record_size.rows).await,
                    RespEventSerial::OnData(data) => session_record_service.append(handler_req.session_id.as_str(), data),
                    RespEventSerial::OnClose => session_record_service.stop(handler_req.session_id.as_str()).await,
                    RespEventSerial::OnError(_) => {}
                }
                serial_service.handler_serial_event(socket, handler_req, event);
                Ok(())
            })
        });

        match serial_client.open(context_serial, handler).await {
            Ok(()) => {
                drop(_guard);
                Ok(())
            }
            Err(error) => {
                client_map.remove(&session_id);
                if client_map.is_empty() {
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
            return Err(anyhow!("serial close req session_id is empty"));
        }
        let socket_id = _socket.id.to_string();

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
            return Err(anyhow!("serial write req session_id is empty"));
        }
        let socket_id = _socket.id.to_string();
        let payload: WriteEvent = req.as_payload()?;
        let data = payload.data;

        let session_lock =
            self.socket_id_2_session_locks.get(&socket_id).and_then(|m| m.get(&session_id).map(|v| Arc::clone(v.value()))).ok_or_else(|| anyhow!("serial session not found: {}", session_id))?;
        let _guard = session_lock.lock().await;

        let client =
            self.socket_id_2_session_id_2_client.get(&socket_id).and_then(|m| m.get(&session_id).map(|v| Arc::clone(v.value()))).ok_or_else(|| anyhow!("serial session not found: {}", session_id))?;
        self.session_record_service.mark_input(session_id.as_str(), data.as_str());
        client.write(data).await?;
        Ok(())
    }
}
