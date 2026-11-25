use anyhow::anyhow;
use std::sync::Arc;
use dashmap::DashMap;
use summer::tracing::warn;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use tokio::sync::Mutex as AsyncMutex;
use crate::model::dto::shared_ptr::Shared;
use crate::web::ws::ws_manager::WsManager;
use summer::plugin::component::ComponentRef;
use self::session_client::ShellSessionClient;
use crate::service::service_shell::ShellService;
use crate::base::base_app::{AppContext, BaseApp};
use summer_web::socketioxide::extract::SocketRef;
use crate::service::service_device::DeviceService;
use crate::web::ws::handle::handle_base::HandleBase;
use crate::model::req::req_ws_event::ReqWsEventShell;
use crate::model::req::req_ws_context::ReqSocketContext;
use crate::module::shell::client_shell_pty::ClientShellPty;
use crate::module::shell::shell_session_hub::ShellSessionHub;
use crate::model::resp::event::resp_event_shell::RespEventShell;
use crate::service::service_session_record::SessionRecordService;
use crate::module::common::client_terminal::{ClientTerminal, EventHandler};
use crate::module::common::context::context_shell::{ConfigShell, ContextShell};
use crate::model::req::event::ws_shell::{OpenEvent, ResizeEvent, SnapshotEvent, WriteEvent};

pub mod session_client;

fn create_socket_id_2_session_locks() -> Arc<DashMap<String, Arc<DashMap<String, Arc<AsyncMutex<()>>>>>> {
    Arc::new(DashMap::new())
}

fn create_socket_id_2_session_id_2_client() -> Arc<DashMap<String, Arc<DashMap<String, Arc<ShellSessionClient>>>>> {
    Arc::new(DashMap::new())
}

#[base_app(sort = 902)]
#[derive(Clone, Service)]
pub struct HandleShell {
    #[inject(component)]
    ws_manager: WsManager,

    #[inject(component)]
    device_service: DeviceService,

    #[inject(component)]
    shell_session_hub: ShellSessionHub,

    shell_service: ComponentRef<ShellService>,

    #[inject(component)]
    session_record_service: SessionRecordService,

    #[inject(func = create_socket_id_2_session_locks())]
    socket_id_2_session_locks: Arc<DashMap<String, Arc<DashMap<String, Arc<AsyncMutex<()>>>>>>,

    #[inject(func = create_socket_id_2_session_id_2_client())]
    socket_id_2_session_id_2_client: Arc<DashMap<String, Arc<DashMap<String, Arc<ShellSessionClient>>>>>,
}

#[async_trait]
impl BaseApp for HandleShell {
    fn name(&self) -> &'static str {
        "HandleShell"
    }
    async fn start(&self, _ctx: &AppContext) -> anyhow::Result<()> {
        let ref_self = Arc::new(self.clone());
        self.ws_manager.register(ref_self);
        Ok(())
    }
}

#[async_trait]
impl HandleBase for HandleShell {
    fn get_name(&self) -> &'static str {
        "HandleShell"
    }

    fn contains_type(&self, type_code: i16) -> bool {
        ReqWsEventShell::form_number(type_code).is_some()
    }

    async fn handle_close(&self, socket: SocketRef) -> anyhow::Result<()> {
        let socket_id = socket.id.to_string();
        let clients = self.socket_id_2_session_id_2_client.remove(&socket_id).map(|(_, v)| v);
        let Some(clients) = clients else {
            return Ok(());
        };
        self.socket_id_2_session_locks.remove(&socket_id);

        let clients: Vec<Arc<ShellSessionClient>> = clients.iter().map(|e| Arc::clone(e.value())).collect();
        for client in clients {
            if let Err(e) = client.close().await {
                warn!("[shell] Warning: failed to close client_shell: {}", e);
            }
        }
        Ok(())
    }

    async fn handle_msg(&self, _socket: SocketRef, req: Arc<ReqSocketContext>) -> anyhow::Result<()> {
        let Some(event) = ReqWsEventShell::form_number(req.r#type) else {
            return Err(anyhow!("invalid event type: {}", req.r#type));
        };
        match event {
            ReqWsEventShell::Open => {
                self.handle_open(_socket, req).await?;
            }
            ReqWsEventShell::Close => {
                self.handle_close(_socket, req).await?;
            }
            ReqWsEventShell::Write => {
                self.handle_write(_socket, req).await?;
            }
            ReqWsEventShell::Resize => {
                self.handle_resize(_socket, req).await?;
            }
            ReqWsEventShell::Snapshot => {
                self.handle_snapshot(_socket, req).await?;
            }
        }
        Ok(())
    }
}

impl HandleShell {
    async fn handle_open(&self, socket: SocketRef, req: Arc<ReqSocketContext>) -> anyhow::Result<()> {
        let session_id = req.session_id.clone();
        let socket_id = socket.id.to_string();
        if session_id.is_empty() {
            return Err(anyhow!("shell open req session_id is empty"));
        }
        let payload: OpenEvent = req.as_payload()?;
        let session_lock_map = self.socket_id_2_session_locks.entry(socket_id.clone()).or_insert_with(|| Arc::new(DashMap::new())).clone();
        let session_lock = session_lock_map.entry(session_id.clone()).or_insert_with(|| Arc::new(AsyncMutex::new(()))).clone();
        let _guard = session_lock.lock().await;

        let shell_open = payload.data;
        if let Some(process_id) = shell_open.attach_process_id.as_deref() {
            let record = self.shell_session_hub.get(process_id).ok_or_else(|| anyhow!("interactive process not found: {}", process_id))?;
            let viewer = self.build_viewer(socket.clone(), Arc::clone(&req));
            let scrollback = record.scrollback();
            self.shell_service.handler_shell_event(socket.clone(), Arc::clone(&req), RespEventShell::OnOpen);
            if !scrollback.is_empty() {
                self.shell_service.handler_shell_event(socket.clone(), Arc::clone(&req), RespEventShell::OnData(scrollback));
            }
            record.attach(viewer);
            let client = Arc::new(ShellSessionClient::attached(record));
            let client_map = self.socket_id_2_session_id_2_client.entry(socket_id).or_insert_with(|| Arc::new(DashMap::new())).clone();
            if let Some(old_client) = client_map.insert(session_id.clone(), client) {
                old_client.close().await?;
            }
            return Ok(());
        }
        let local_device = self.device_service.select_local_device();
        let shell_descr = self.shell_service.find_by_shell_id(&shell_open.shell_id).ok_or_else(|| anyhow!("shell not found: {}", shell_open.shell_id))?;
        let config_shell = Shared::new(ConfigShell { cwd: shell_open.cwd, shell_descr, env: vec![], cpu_percent: 0 });
        let context_shell = Arc::new(ContextShell { size: shell_open.size, config: config_shell, device: local_device });

        let opened_client = (|| async {
            let socket = socket.clone();
            let handler_req = Arc::clone(&req);
            let shell_client = Arc::new(ClientShellPty::new());
            let shell_service = self.shell_service.clone();
            let record_size = context_shell.size;
            let record_shell_id = context_shell.config.read().shell_descr.shell_id.clone();
            let session_record_service = self.session_record_service.clone();
            let handler: EventHandler<RespEventShell> = Arc::new(move |event| {
                let socket = socket.clone();
                let handler_req = Arc::clone(&handler_req);
                let shell_service = shell_service.clone();
                let record_shell_id = record_shell_id.clone();
                let session_record_service = session_record_service.clone();
                Box::pin(async move {
                    match &event {
                        RespEventShell::OnOpen => session_record_service.start(handler_req.session_id.as_str(), "shell", record_shell_id.as_str(), record_size.cols, record_size.rows).await,
                        RespEventShell::OnData(data) => session_record_service.append(handler_req.session_id.as_str(), data),
                        RespEventShell::OnClose => session_record_service.stop(handler_req.session_id.as_str()).await,
                        RespEventShell::Exit { .. } => session_record_service.stop(handler_req.session_id.as_str()).await,
                        _ => {}
                    }
                    shell_service.handler_shell_event(socket, handler_req, event);
                    Ok(())
                })
            });
            shell_client.open(context_shell, handler).await?;
            Ok::<_, anyhow::Error>(shell_client)
        })()
        .await;

        match opened_client {
            Ok(shell_client) => {
                let client_map = self.socket_id_2_session_id_2_client.entry(socket_id).or_insert_with(|| Arc::new(DashMap::new())).clone();
                let replaced_client = client_map.insert(session_id.clone(), Arc::new(ShellSessionClient::owned(shell_client)));
                drop(_guard);
                if let Some(old_client) = replaced_client {
                    if let Err(e) = old_client.close().await {
                        warn!("[shell] Warning: failed to close previous shell session {}: {}", session_id, e);
                    }
                }
                Ok(())
            }
            Err(error) => Err(error),
        }
    }

    async fn handle_close(&self, _socket: SocketRef, req: Arc<ReqSocketContext>) -> anyhow::Result<()> {
        let session_id = req.session_id.clone();
        if session_id.is_empty() {
            return Err(anyhow!("shell close req session_id is empty"));
        }
        let socket_id = _socket.id.to_string();

        let session_lock_map = self.socket_id_2_session_locks.get(&socket_id).map(|v| Arc::clone(v.value())).ok_or_else(|| anyhow!("shell session not found: {}", session_id))?;
        let session_lock = session_lock_map.get(&session_id).map(|v| Arc::clone(v.value())).ok_or_else(|| anyhow!("shell session not found: {}", session_id))?;
        let _guard = session_lock.lock().await;

        let client_map = self.socket_id_2_session_id_2_client.get(&socket_id).map(|v| Arc::clone(v.value())).ok_or_else(|| anyhow!("shell session not found: {}", session_id))?;
        let client = client_map.get(&session_id).map(|v| Arc::clone(v.value())).ok_or_else(|| anyhow!("shell session not found: {}", session_id))?;
        client.close().await?;
        client_map.remove(&session_id);
        Ok(())
    }

    async fn handle_write(&self, _socket: SocketRef, req: Arc<ReqSocketContext>) -> anyhow::Result<()> {
        let session_id = req.session_id.clone();
        if session_id.is_empty() {
            return Err(anyhow!("shell write req session_id is empty"));
        }
        let socket_id = _socket.id.to_string();
        let payload: WriteEvent = req.as_payload()?;
        let data = payload.data;

        let session_lock_map = self.socket_id_2_session_locks.get(&socket_id).map(|v| Arc::clone(v.value())).ok_or_else(|| anyhow!("shell session not found: {}", session_id))?;
        let session_lock = session_lock_map.get(&session_id).map(|v| Arc::clone(v.value())).ok_or_else(|| anyhow!("shell session not found: {}", session_id))?;
        let _guard = session_lock.lock().await;

        let client_map = self.socket_id_2_session_id_2_client.get(&socket_id).map(|v| Arc::clone(v.value())).ok_or_else(|| anyhow!("shell session not found: {}", session_id))?;
        let client = client_map.get(&session_id).map(|v| Arc::clone(v.value())).ok_or_else(|| anyhow!("shell session not found: {}", session_id))?;
        self.session_record_service.mark_input(session_id.as_str(), data.as_str());
        client.write(data).await?;
        Ok(())
    }

    async fn handle_resize(&self, _socket: SocketRef, req: Arc<ReqSocketContext>) -> anyhow::Result<()> {
        let session_id = req.session_id.clone();
        if session_id.is_empty() {
            return Err(anyhow!("shell resize req session_id is empty"));
        }
        let socket_id = _socket.id.to_string();
        let payload: ResizeEvent = req.as_payload()?;
        let window_size = payload.data;

        let session_lock_map = self.socket_id_2_session_locks.get(&socket_id).map(|v| Arc::clone(v.value())).ok_or_else(|| anyhow!("shell session not found: {}", session_id))?;
        let session_lock = session_lock_map.get(&session_id).map(|v| Arc::clone(v.value())).ok_or_else(|| anyhow!("shell session not found: {}", session_id))?;
        let _guard = session_lock.lock().await;

        let client_map = self.socket_id_2_session_id_2_client.get(&socket_id).map(|v| Arc::clone(v.value())).ok_or_else(|| anyhow!("shell session not found: {}", session_id))?;
        let client = client_map.get(&session_id).map(|v| Arc::clone(v.value())).ok_or_else(|| anyhow!("shell session not found: {}", session_id))?;
        client.set_window_size(window_size).await?;
        Ok(())
    }

    async fn handle_snapshot(&self, socket: SocketRef, req: Arc<ReqSocketContext>) -> anyhow::Result<()> {
        let session_id = req.session_id.clone();
        if session_id.is_empty() {
            return Err(anyhow!("shell snapshot req session_id is empty"));
        }
        let socket_id = socket.id.to_string();
        let payload: SnapshotEvent = req.as_payload()?;
        let client_map = self.socket_id_2_session_id_2_client.get(&socket_id).map(|v| Arc::clone(v.value())).ok_or_else(|| anyhow!("shell session not found: {}", session_id))?;
        let client = client_map.get(&session_id).map(|v| Arc::clone(v.value())).ok_or_else(|| anyhow!("shell session not found: {}", session_id))?;
        client.report_snapshot(payload.data);
        Ok(())
    }

    fn build_viewer(&self, socket: SocketRef, req: Arc<ReqSocketContext>) -> EventHandler<RespEventShell> {
        let shell_service = self.shell_service.clone();
        Arc::new(move |event| {
            let socket = socket.clone();
            let req = Arc::clone(&req);
            let shell_service = shell_service.clone();
            Box::pin(async move {
                shell_service.handler_shell_event(socket, req, event);
                Ok(())
            })
        })
    }
}
