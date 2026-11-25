use anyhow::Result;
use std::sync::Arc;
use dashmap::DashMap;
use summer::tracing::warn;
use summer::plugin::Service;
use async_trait::async_trait;
use tokio::task::AbortHandle;
use std::time::{Duration, Instant};
use crate::model::device::DEVICE_LOCAL_ID;
use summer::plugin::component::ComponentRef;
use crate::service::service_base::BaseService;
use crate::model::resp::resp_model::ResultCode;
use tokio::sync::{Mutex as AsyncMutex, Notify};
use crate::base::base_app::{AppContext, BaseApp};
use crate::service::service_device::DeviceService;
use crate::service::service_config_ssh::ConfigSshService;
use crate::module::common::context::context_base::WindowSize;
use crate::module::ssh_shell::client_ssh_shell::ClientSshShell;
use crate::module::ssh::client_session_factory::ClientSessionFactory;
use crate::module::common::context::context_ssh_shell::ContextSshShell;
use crate::module::common::dto::shell_chunk_callback::ShellChunkCallback;

const DEFAULT_IDLE_SECONDS: u64 = 180;
const MAX_IDLE_SECONDS: u64 = 30 * 60;

struct ShellEntry {
    notify: Arc<Notify>,
    watcher: AbortHandle,
    client: Arc<ClientSshShell>,
    last_active: Arc<AsyncMutex<Instant>>,
}

fn create_clients() -> Arc<DashMap<String, Arc<ShellEntry>>> {
    Arc::new(DashMap::new())
}

#[derive(Clone, Service)]
pub struct SshShellService {
    #[inject(component)]
    device_service: DeviceService,

    #[inject(component)]
    service_config_ssh: ConfigSshService,

    ssh_client_session_factory: ComponentRef<ClientSessionFactory>,

    #[inject(func = create_clients())]
    clients: Arc<DashMap<String, Arc<ShellEntry>>>,
}

#[async_trait]
impl BaseApp for SshShellService {
    fn name(&self) -> &'static str {
        "SshShellService"
    }

    async fn start(&self, _ctx: &AppContext) -> Result<()> {
        Ok(())
    }
}

impl SshShellService {
    pub async fn open(&self, session_id: String, device_id: String) -> Result<()> {
        if session_id.is_empty() {
            return Err(ResultCode::SessionIdEmpty.into());
        }
        if device_id.is_empty() {
            return Err(ResultCode::DeviceIdEmpty.into());
        }
        if device_id == DEVICE_LOCAL_ID {
            return Err(ResultCode::DeviceIdLocal.into());
        }
        if self.clients.contains_key(&session_id) {
            return Ok(());
        }
        let (context, idle_timeout) = self.build_context(&device_id).await?;
        let client = Arc::new(ClientSshShell::new(session_id.clone(), self.ssh_client_session_factory.clone()));
        client.open(&context, None).await?;
        let entry = self.spawn_entry(session_id.clone(), client, idle_timeout);
        self.clients.insert(session_id, Arc::new(entry));
        Ok(())
    }

    pub async fn exec(&self, session_id: &str, command: String, exec_timeout: Duration) -> Result<(i32, String)> {
        self.exec_with_chunk(session_id, command, exec_timeout, None).await
    }

    pub async fn exec_with_chunk(&self, session_id: &str, command: String, exec_timeout: Duration, on_chunk: Option<ShellChunkCallback>) -> Result<(i32, String)> {
        let entry = self.clients.get(session_id).ok_or_else(|| anyhow::Error::from(ResultCode::SshShellNotOpened))?.value().clone();
        let result = entry.client.exec(command, exec_timeout, on_chunk).await;
        match &result {
            Ok(_) => self.touch(&entry).await,
            Err(_) => self.discard(session_id).await,
        }
        result
    }

    pub async fn close(&self, session_id: &str) -> Result<()> {
        self.discard(session_id).await;
        Ok(())
    }

    async fn touch(&self, entry: &ShellEntry) {
        *entry.last_active.lock().await = Instant::now();
        entry.notify.notify_one();
    }

    async fn discard(&self, session_id: &str) {
        let Some((_, entry)) = self.clients.remove(session_id) else {
            return;
        };
        entry.watcher.abort();
        if let Err(error) = entry.client.close().await {
            warn!("[ssh] ssh shell close failed for session_id {}: {}", session_id, error);
        }
    }

    async fn build_context(&self, device_id: &str) -> Result<(ContextSshShell, Duration)> {
        let device = self.device_service.find_by_id(device_id.to_string()).await?.ok_or_else(|| anyhow::Error::from(ResultCode::DeviceNotFound))?;
        let config = self.service_config_ssh.get_by_device_id(device_id.to_string()).ok_or_else(|| anyhow::Error::from(ResultCode::SshConfigNotFound))?;
        let raw_idle_seconds = config.read().shell_idle_timeout_seconds as u64;
        let idle_seconds = Self::resolve_idle_seconds(raw_idle_seconds);
        let context = ContextSshShell { size: WindowSize::default(), config, device };
        Ok((context, Duration::from_secs(idle_seconds)))
    }

    fn resolve_idle_seconds(raw: u64) -> u64 {
        if raw <= 0 || raw > MAX_IDLE_SECONDS {
            return DEFAULT_IDLE_SECONDS;
        }
        raw
    }

    fn spawn_entry(&self, session_id: String, client: Arc<ClientSshShell>, idle: Duration) -> ShellEntry {
        let notify = Arc::new(Notify::new());
        let clients = Arc::clone(&self.clients);
        let watch_client = Arc::clone(&client);
        let watch_notify = Arc::clone(&notify);
        let last_active = Arc::new(AsyncMutex::new(Instant::now()));
        let watch_last_active = Arc::clone(&last_active);
        let join_handle = tokio::spawn(async move {
            Self::watch_idle(session_id, watch_client, watch_last_active, watch_notify, clients, idle).await;
        });
        ShellEntry { client, last_active, notify, watcher: join_handle.abort_handle() }
    }

    async fn watch_idle(session_id: String, client: Arc<ClientSshShell>, last_active: Arc<AsyncMutex<Instant>>, notify: Arc<Notify>, clients: Arc<DashMap<String, Arc<ShellEntry>>>, idle: Duration) {
        loop {
            let elapsed = Instant::now().duration_since(*last_active.lock().await);
            let Some(remaining) = idle.checked_sub(elapsed) else {
                break;
            };
            tokio::select! {
                _ = tokio::time::sleep(remaining) => {
                    let now_elapsed = Instant::now().duration_since(*last_active.lock().await);
                    if now_elapsed >= idle {
                        break;
                    }
                }
                _ = notify.notified() => {}
            }
        }
        clients.remove(&session_id);
        if let Err(error) = client.close().await {
            warn!("[ssh] Ssh shell idle close failed for session_id {}: {}", session_id, error);
        }
    }
}
