use anyhow::anyhow;
use std::sync::Arc;
use dashmap::DashMap;
use crate::ContextSsh;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use summer::tracing::{debug, warn};
use tokio::sync::Mutex as AsyncMutex;
use crate::web::ws::ws_manager::WsManager;
use crate::service::service_ssh::SshService;
use summer::plugin::component::ComponentRef;
use crate::module::ssh::client_ssh::ClientSsh;
use crate::service::service_base::BaseService;
use crate::base::base_app::{AppContext, BaseApp};
use summer_web::socketioxide::extract::SocketRef;
use crate::service::service_device::DeviceService;
use crate::model::req::req_ws_event::ReqWsEventSsh;
use crate::web::ws::handle::handle_base::HandleBase;
use crate::model::req::req_ws_context::ReqSocketContext;
use crate::service::service_config_ssh::ConfigSshService;
use crate::model::resp::event::resp_event_ssh::RespEventSsh;
use crate::service::service_session_record::SessionRecordService;
use crate::module::ssh::client_session_factory::ClientSessionFactory;
use crate::model::req::event::ws_ssh::{OpenEvent, ResizeEvent, WriteEvent};
use crate::module::common::client_terminal::{ClientTerminal, EventHandler};

fn create_socket_id_2_session_locks() -> Arc<DashMap<String, Arc<DashMap<String, Arc<AsyncMutex<()>>>>>> {
    Arc::new(DashMap::new())
}

fn create_socket_id_2_session_id_2_client() -> Arc<DashMap<String, Arc<DashMap<String, Arc<ClientSsh>>>>> {
    Arc::new(DashMap::new())
}

#[base_app(sort = 904)]
#[derive(Clone, Service)]
pub struct HandleSsh {
    #[inject(component)]
    ws_manager: WsManager,

    #[inject(component)]
    device_service: DeviceService,

    #[inject(component)]
    config_ssh_service: ConfigSshService,

    #[inject(component)]
    session_record_service: SessionRecordService,

    ssh_service: ComponentRef<SshService>,

    client_session_factory: ComponentRef<ClientSessionFactory>,

    #[inject(func = create_socket_id_2_session_locks())]
    socket_id_2_session_locks: Arc<DashMap<String, Arc<DashMap<String, Arc<AsyncMutex<()>>>>>>,

    #[inject(func = create_socket_id_2_session_id_2_client())]
    socket_id_2_session_id_2_client: Arc<DashMap<String, Arc<DashMap<String, Arc<ClientSsh>>>>>,
}

#[async_trait]
impl BaseApp for HandleSsh {
    fn name(&self) -> &'static str {
        "HandleSsh"
    }
    async fn start(&self, _ctx: &AppContext) -> anyhow::Result<()> {
        let ref_self = Arc::new(self.clone());
        self.ws_manager.register(ref_self);
        Ok(())
    }
}

#[async_trait]
impl HandleBase for HandleSsh {
    fn get_name(&self) -> &'static str {
        "HandleSsh"
    }

    fn contains_type(&self, type_code: i16) -> bool {
        ReqWsEventSsh::form_number(type_code).is_some()
    }

    async fn handle_close(&self, socket: SocketRef) -> anyhow::Result<()> {
        let socket_id = socket.id.to_string();
        let clients = self.socket_id_2_session_id_2_client.remove(&socket_id).map(|(_, v)| v);
        let Some(clients) = clients else {
            return Ok(());
        };
        self.socket_id_2_session_locks.remove(&socket_id);

        let clients: Vec<Arc<ClientSsh>> = clients.iter().map(|e| Arc::clone(e.value())).collect();
        for client in clients {
            if let Err(e) = client.close().await {
                warn!("[ssh] Warning: failed to close client_ssh: {}", e);
            }
        }
        Ok(())
    }

    async fn handle_msg(&self, socket: SocketRef, req: Arc<ReqSocketContext>) -> anyhow::Result<()> {
        let Some(event) = ReqWsEventSsh::form_number(req.r#type) else {
            return Err(anyhow!("invalid event type: {}", req.r#type));
        };
        match event {
            ReqWsEventSsh::Open => {
                self.handle_open(socket, req).await?;
            }
            ReqWsEventSsh::Close => {
                self.handle_close(socket, req).await?;
            }
            ReqWsEventSsh::Write => {
                self.handle_write(socket, req).await?;
            }
            ReqWsEventSsh::Resize => {
                self.handle_resize(socket, req).await?;
            }
        }
        Ok(())
    }
}

impl HandleSsh {
    async fn handle_open(&self, socket: SocketRef, req: Arc<ReqSocketContext>) -> anyhow::Result<()> {
        let session_id = req.session_id.clone();
        if session_id.is_empty() {
            return Err(anyhow!("ssh open req session_id is empty"));
        }
        let payload: OpenEvent = req.as_payload()?;
        let device_id = payload.data.device_id.clone();
        let device = self.device_service.find_by_id(device_id.clone()).await?.ok_or_else(|| anyhow!("device not found for device_id: {}", device_id))?;
        let config = self.config_ssh_service.get_by_device_id(device_id.clone()).ok_or_else(|| anyhow!("ssh config not found for device_id: {}", device_id))?;
        let context_ssh = Arc::new(ContextSsh { size: payload.data.size, config, device });
        self.open_ssh_session(socket, req, session_id, context_ssh).await
    }

    async fn open_ssh_session(&self, socket: SocketRef, req: Arc<ReqSocketContext>, session_id: String, context_ssh: Arc<ContextSsh>) -> anyhow::Result<()> {
        let socket_id = socket.id.to_string();
        let session_lock_map = self.socket_id_2_session_locks.entry(socket_id.clone()).or_insert_with(|| Arc::new(DashMap::new())).clone();
        let session_lock = session_lock_map.entry(session_id.clone()).or_insert_with(|| Arc::new(AsyncMutex::new(()))).clone();
        let _guard = session_lock.lock().await;

        match self.connect_ssh_client(socket.clone(), session_id.clone(), Arc::clone(&req), context_ssh).await {
            Ok(ssh_client) => {
                let client_map = self.socket_id_2_session_id_2_client.entry(socket_id).or_insert_with(|| Arc::new(DashMap::new())).clone();
                let replaced = client_map.insert(session_id.clone(), Arc::clone(&ssh_client));
                drop(_guard);
                if let Some(old) = replaced {
                    if let Err(e) = old.close().await {
                        warn!("[ssh] Warning: failed to close previous ssh session {}: {}", session_id, e);
                    }
                }
                Ok(())
            }
            Err(error) => {
                let empty = self.socket_id_2_session_id_2_client.get(&socket_id).map(|m| m.value().is_empty()).unwrap_or(true);
                if empty {
                    self.socket_id_2_session_locks.remove(&socket_id);
                    self.socket_id_2_session_id_2_client.remove(&socket_id);
                }
                Err(error)
            }
        }
    }

    async fn connect_ssh_client(&self, socket: SocketRef, session_id: String, req: Arc<ReqSocketContext>, context_ssh: Arc<ContextSsh>) -> anyhow::Result<Arc<ClientSsh>> {
        let ssh_service = self.ssh_service.clone();
        let record_size = context_ssh.size;
        let record_device_name = context_ssh.device.read().name.clone();
        let session_record_service = self.session_record_service.clone();
        let ssh_client = Arc::new(ClientSsh::new(session_id, self.client_session_factory.clone()));
        let handler: EventHandler<RespEventSsh> = Arc::new(move |event| {
            let socket = socket.clone();
            let req = Arc::clone(&req);
            let ssh_service = ssh_service.clone();
            let record_device_name = record_device_name.clone();
            let session_record_service = session_record_service.clone();
            Box::pin(async move {
                match &event {
                    RespEventSsh::OnOpen => session_record_service.start(req.session_id.as_str(), "ssh", record_device_name.as_str(), record_size.cols, record_size.rows).await,
                    RespEventSsh::OnData(data) => session_record_service.append(req.session_id.as_str(), data),
                    RespEventSsh::OnClose => session_record_service.stop(req.session_id.as_str()).await,
                    _ => {}
                }
                ssh_service.handler_ssh_event(socket, req, event).await;
                Ok(())
            })
        });
        ssh_client.open(context_ssh, handler).await?;
        Ok(ssh_client)
    }

    async fn handle_close(&self, socket: SocketRef, req: Arc<ReqSocketContext>) -> anyhow::Result<()> {
        let session_id = req.session_id.clone();
        if session_id.is_empty() {
            return Err(anyhow!("ssh close req session_id is empty"));
        }
        let socket_id = socket.id.to_string();
        self.client_session_factory.cancel_connect(&session_id);

        let Some(session_lock_map) = self.socket_id_2_session_locks.get(&socket_id).map(|v| Arc::clone(v.value())) else {
            return Ok(());
        };
        let Some(client_map) = self.socket_id_2_session_id_2_client.get(&socket_id).map(|v| Arc::clone(v.value())) else {
            return Ok(());
        };
        let Some((session_lock, client)) = self.resolve_session(&socket_id, &session_id) else {
            return Ok(());
        };
        let _guard = session_lock.lock().await;
        client.close().await?;

        client_map.remove(&session_id);
        session_lock_map.remove(&session_id);
        if client_map.is_empty() {
            self.socket_id_2_session_id_2_client.remove(&socket_id);
            self.socket_id_2_session_locks.remove(&socket_id);
        }
        Ok(())
    }

    async fn handle_write(&self, socket: SocketRef, req: Arc<ReqSocketContext>) -> anyhow::Result<()> {
        let session_id = req.session_id.clone();
        if session_id.is_empty() {
            return Err(anyhow!("ssh write req session_id is empty"));
        }
        let socket_id = socket.id.to_string();
        let payload: WriteEvent = req.as_payload()?;
        let data = payload.data;

        let Some((session_lock, client)) = self.resolve_session(&socket_id, &session_id) else {
            debug!("[ssh] ssh 会话已失效, 丢弃写入 [{}]", session_id);
            return Ok(());
        };
        let _guard = session_lock.lock().await;
        self.session_record_service.mark_input(session_id.as_str(), data.as_str());
        client.write(data).await?;
        Ok(())
    }

    async fn handle_resize(&self, socket: SocketRef, req: Arc<ReqSocketContext>) -> anyhow::Result<()> {
        let session_id = req.session_id.clone();
        if session_id.is_empty() {
            return Err(anyhow!("ssh resize req session_id is empty"));
        }
        let socket_id = socket.id.to_string();
        let payload: ResizeEvent = req.as_payload()?;
        let window_size = payload.data;

        let Some((session_lock, client)) = self.resolve_session(&socket_id, &session_id) else {
            debug!("[ssh] ssh 会话已失效, 丢弃窗口大小变更 [{}]", session_id);
            return Ok(());
        };
        let _guard = session_lock.lock().await;
        client.set_window_size(window_size).await?;
        Ok(())
    }

    fn resolve_session(&self, socket_id: &str, session_id: &str) -> Option<(Arc<AsyncMutex<()>>, Arc<ClientSsh>)> {
        let session_lock = self.socket_id_2_session_locks.get(socket_id)?.get(session_id).map(|v| Arc::clone(v.value()))?;
        let client = self.socket_id_2_session_id_2_client.get(socket_id)?.get(session_id).map(|v| Arc::clone(v.value()))?;
        Some((session_lock, client))
    }
}
