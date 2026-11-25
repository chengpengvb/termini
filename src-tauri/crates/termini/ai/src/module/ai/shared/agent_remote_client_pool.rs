use std::sync::Arc;
use dashmap::DashMap;
use std::sync::Mutex;
use summer::tracing::warn;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use crate::util::id_factory::uuid;
use std::time::{Duration, Instant};
use crate::model::device::DEVICE_LOCAL_ID;
use summer::plugin::component::ComponentRef;
use crate::service::service_base::BaseService;
use crate::base::base_app::{AppContext, BaseApp};
use crate::service::service_device::DeviceService;
use crate::module::ai::tools::ai_tool_error::AiToolError;
use crate::module::common::client_terminal::EventHandler;
use crate::module::sftp::client_transfer::ClientTransfer;
use crate::service::service_config_ftp::ConfigFtpService;
use crate::service::service_config_ssh::ConfigSshService;
use crate::module::common::context::context_ftp::ContextFtp;
use crate::module::common::context::context_base::WindowSize;
use crate::model::resp::event::resp_event_sftp::RespEventSftp;
use crate::module::ssh_shell::client_ssh_shell::ClientSshShell;
use crate::service::service_config_telnet::ConfigTelnetService;
use crate::model::resp::event::resp_event_ssh_auth::OpenSshStep;
use crate::module::sftp::client_transfer_ftp::ClientTransferFtp;
use crate::module::sftp::client_transfer_sftp::ClientTransferSftp;
use crate::module::ssh::client_session_factory::ClientSessionFactory;
use crate::model::resp::resp_ssh_open_failure::format_ssh_open_failure;
use crate::module::telnet_shell::client_telnet_shell::ClientTelnetShell;
use crate::module::ssh_shell::ssh_shell_exec_core::resolve_ssh_shell_context;
use crate::module::telnet_shell::telnet_shell_exec_core::resolve_telnet_shell_context;
use crate::web::ws::handle::r#impl::sftp::{ClientTransferFtpDyn, ClientTransferSftpDyn};

pub const AGENT_REMOTE_CLIENT_KIND_SSH_SHELL: i8 = 0;
pub const AGENT_REMOTE_CLIENT_KIND_SFTP: i8 = 1;
pub const AGENT_REMOTE_CLIENT_KIND_TELNET_SHELL: i8 = 2;
pub const AGENT_REMOTE_CLIENT_KIND_FTP: i8 = 3;

const MAX_CLIENTS_PER_KIND_PER_REQ: usize = 8;
const EXIT_CLOSE_WAIT: Duration = Duration::from_secs(3);

enum CachedClient {
    SshShell { opened_at: Instant, client: Arc<ClientSshShell> },
    Sftp { opened_at: Instant, client: Arc<ClientTransferSftpDyn> },
    TelnetShell { opened_at: Instant, client: Arc<ClientTelnetShell> },
    Ftp { opened_at: Instant, client: Arc<ClientTransferFtpDyn> },
}

impl CachedClient {
    fn kind(&self) -> i8 {
        match self {
            CachedClient::SshShell { .. } => AGENT_REMOTE_CLIENT_KIND_SSH_SHELL,
            CachedClient::Sftp { .. } => AGENT_REMOTE_CLIENT_KIND_SFTP,
            CachedClient::TelnetShell { .. } => AGENT_REMOTE_CLIENT_KIND_TELNET_SHELL,
            CachedClient::Ftp { .. } => AGENT_REMOTE_CLIENT_KIND_FTP,
        }
    }

    fn opened_at(&self) -> Instant {
        match self {
            CachedClient::SshShell { opened_at, .. } => *opened_at,
            CachedClient::Sftp { opened_at, .. } => *opened_at,
            CachedClient::TelnetShell { opened_at, .. } => *opened_at,
            CachedClient::Ftp { opened_at, .. } => *opened_at,
        }
    }
}

fn create_clients() -> Arc<DashMap<String, DashMap<String, CachedClient>>> {
    Arc::new(DashMap::new())
}

fn cache_key(kind: i8, device_id: &str) -> String {
    format!("{}:{}", kind, device_id)
}

#[base_app(sort = 761)]
#[derive(Clone, Service)]
pub struct AgentRemoteClientPool {
    #[inject(component)]
    device_service: DeviceService,

    #[inject(component)]
    config_ftp_service: ConfigFtpService,

    #[inject(component)]
    config_ssh_service: ConfigSshService,

    #[inject(component)]
    config_telnet_service: ConfigTelnetService,

    #[inject(func = create_clients())]
    clients: Arc<DashMap<String, DashMap<String, CachedClient>>>,

    ssh_client_session_factory: ComponentRef<ClientSessionFactory>,
}

#[async_trait]
impl BaseApp for AgentRemoteClientPool {
    fn name(&self) -> &'static str {
        "AgentRemoteClientPool"
    }

    async fn on_exit(&self, _ctx: &AppContext) -> anyhow::Result<()> {
        let req_ids: Vec<String> = self.clients.iter().map(|entry| entry.key().clone()).collect();
        let mut cached = Vec::new();
        for req_id in req_ids {
            let Some((_, by_device)) = self.clients.remove(&req_id) else {
                continue;
            };
            for entry in by_device.into_iter() {
                cached.push(entry.1);
            }
        }
        let closes = cached.into_iter().map(Self::close_cached);
        let _ = tokio::time::timeout(EXIT_CLOSE_WAIT, futures::future::join_all(closes)).await;
        Ok(())
    }
}

impl AgentRemoteClientPool {
    pub fn release(&self, req_id: &str) {
        let Some((_, by_device)) = self.clients.remove(req_id) else {
            return;
        };
        for entry in by_device.iter() {
            Self::spawn_close(entry.value().clone_client());
        }
    }

    pub async fn get_or_open_ssh_shell(&self, req_id: &str, device_id: &str) -> Result<Arc<ClientSshShell>, AiToolError> {
        match self.get_or_open(AGENT_REMOTE_CLIENT_KIND_SSH_SHELL, req_id, device_id).await? {
            CachedClient::SshShell { client, .. } => Ok(client),
            _ => Err(AiToolError::Execution("agent remote client kind mismatch".to_string())),
        }
    }

    pub async fn get_or_open_sftp(&self, req_id: &str, device_id: &str) -> Result<Arc<ClientTransferSftpDyn>, AiToolError> {
        match self.get_or_open(AGENT_REMOTE_CLIENT_KIND_SFTP, req_id, device_id).await? {
            CachedClient::Sftp { client, .. } => Ok(client),
            _ => Err(AiToolError::Execution("agent remote client kind mismatch".to_string())),
        }
    }

    pub async fn get_or_open_telnet_shell(&self, req_id: &str, device_id: &str) -> Result<Arc<ClientTelnetShell>, AiToolError> {
        match self.get_or_open(AGENT_REMOTE_CLIENT_KIND_TELNET_SHELL, req_id, device_id).await? {
            CachedClient::TelnetShell { client, .. } => Ok(client),
            _ => Err(AiToolError::Execution("agent remote client kind mismatch".to_string())),
        }
    }

    pub async fn get_or_open_ftp(&self, req_id: &str, device_id: &str) -> Result<Arc<ClientTransferFtpDyn>, AiToolError> {
        match self.get_or_open(AGENT_REMOTE_CLIENT_KIND_FTP, req_id, device_id).await? {
            CachedClient::Ftp { client, .. } => Ok(client),
            _ => Err(AiToolError::Execution("agent remote client kind mismatch".to_string())),
        }
    }

    async fn get_or_open(&self, kind: i8, req_id: &str, device_id: &str) -> Result<CachedClient, AiToolError> {
        let key = cache_key(kind, device_id);
        if let Some(by_device) = self.clients.get(req_id) {
            if let Some(cached) = by_device.get(&key) {
                return Ok(cached.clone_client());
            }
        }
        let opened = self.open_client(kind, device_id).await?;
        let by_device = self.clients.entry(req_id.to_string()).or_default();
        if let Some(existing) = by_device.get(&key).map(|entry| entry.clone_client()) {
            drop(by_device);
            Self::spawn_close(opened);
            return Ok(existing);
        }
        by_device.insert(key, opened.clone_client());
        drop(by_device);
        self.evict_overflow(req_id, kind);
        Ok(opened)
    }

    async fn open_client(&self, kind: i8, device_id: &str) -> Result<CachedClient, AiToolError> {
        if kind == AGENT_REMOTE_CLIENT_KIND_TELNET_SHELL {
            let context = resolve_telnet_shell_context(device_id, &self.device_service, &self.config_telnet_service).await.map_err(AiToolError::Execution)?;
            let client = Arc::new(ClientTelnetShell::new(uuid()));
            if let Err(error) = client.open(&context).await {
                return Err(AiToolError::Execution(error.to_string()));
            }
            return Ok(CachedClient::TelnetShell { opened_at: Instant::now(), client });
        }
        if kind == AGENT_REMOTE_CLIENT_KIND_FTP {
            if device_id.is_empty() {
                return Err(AiToolError::Execution("device_id is empty".to_string()));
            }
            if device_id == DEVICE_LOCAL_ID {
                return Err(AiToolError::Execution("cannot run remote FTP against the local device".to_string()));
            }
            let device = self
                .device_service
                .find_by_id(device_id.to_string())
                .await
                .map_err(|error| AiToolError::Execution(error.to_string()))?
                .ok_or_else(|| AiToolError::Execution(format!("device not found: {}", device_id)))?;
            let config = self.config_ftp_service.get_by_device_id(device_id.to_string()).ok_or_else(|| AiToolError::Execution(format!("ftp open config_ftp not found for device_id={}", device_id)))?;
            let context = ContextFtp { size: WindowSize::default(), config, device };
            let client: Arc<ClientTransferFtpDyn> = Arc::new(ClientTransferFtp::new());
            let handler: EventHandler<RespEventSftp> = Arc::new(move |_event| Box::pin(async move { Ok(()) }));
            if let Err(error) = client.open(&context, handler).await {
                return Err(AiToolError::Execution(error.to_string()));
            }
            return Ok(CachedClient::Ftp { opened_at: Instant::now(), client });
        }
        let context = resolve_ssh_shell_context(device_id, &self.device_service, &self.config_ssh_service).await.map_err(AiToolError::Execution)?;
        if kind == AGENT_REMOTE_CLIENT_KIND_SSH_SHELL {
            let client = Arc::new(ClientSshShell::new(uuid(), self.ssh_client_session_factory.clone()));
            if let Err(error) = client.open(&context, Some(true)).await {
                return Err(AiToolError::Execution(error.to_string()));
            }
            return Ok(CachedClient::SshShell { opened_at: Instant::now(), client });
        }
        if kind == AGENT_REMOTE_CLIENT_KIND_SFTP {
            let client: Arc<ClientTransferSftpDyn> = Arc::new(ClientTransferSftp::new(uuid(), self.ssh_client_session_factory.clone()));
            let steps = Arc::new(Mutex::new(Vec::<OpenSshStep>::new()));
            let steps_for_handler = steps.clone();
            let handler: EventHandler<RespEventSftp> = Arc::new(move |event| {
                let steps_for_handler = steps_for_handler.clone();
                Box::pin(async move {
                    if let RespEventSftp::OnOpenStep(payload) = event {
                        steps_for_handler.lock().expect("sftp open steps lock").push(payload.step);
                    }
                    Ok(())
                })
            });
            if let Err(error) = client.open(&context, handler).await {
                let collected = steps.lock().expect("sftp open steps lock").clone();
                return Err(AiToolError::Execution(format_ssh_open_failure(device_id, &error.to_string(), &collected)));
            }
            return Ok(CachedClient::Sftp { opened_at: Instant::now(), client });
        }
        Err(AiToolError::Execution(format!("unknown agent remote client kind={}", kind)))
    }

    fn evict_overflow(&self, req_id: &str, kind: i8) {
        let Some(by_device) = self.clients.get(req_id) else {
            return;
        };
        let mut oldest: Vec<(String, Instant)> = by_device.iter().filter(|entry| entry.value().kind() == kind).map(|entry| (entry.key().clone(), entry.value().opened_at())).collect();
        let over = oldest.len().saturating_sub(MAX_CLIENTS_PER_KIND_PER_REQ);
        if over == 0 {
            return;
        }
        drop(by_device);
        oldest.sort_by_key(|(_, opened_at)| *opened_at);
        for (key, _) in oldest.into_iter().take(over) {
            let Some(by_device) = self.clients.get(req_id) else {
                break;
            };
            let Some((_, cached)) = by_device.remove(&key) else {
                continue;
            };
            drop(by_device);
            Self::spawn_close(cached);
        }
    }

    fn spawn_close(cached: CachedClient) {
        tokio::spawn(Self::close_cached(cached));
    }

    async fn close_cached(cached: CachedClient) {
        match cached {
            CachedClient::SshShell { client, .. } => {
                if let Err(error) = client.close().await {
                    warn!("[ssh] ssh shell close failed error={}", error);
                }
            }
            CachedClient::Sftp { client, .. } => {
                if let Err(error) = ClientTransfer::close(client.as_ref()).await {
                    warn!("[sftp] sftp close failed error={}", error);
                }
            }
            CachedClient::TelnetShell { client, .. } => {
                if let Err(error) = client.close().await {
                    warn!("[telnet] telnet shell close failed error={}", error);
                }
            }
            CachedClient::Ftp { client, .. } => {
                if let Err(error) = ClientTransfer::close(client.as_ref()).await {
                    warn!("[ftp] ftp close failed error={}", error);
                }
            }
        }
    }
}

impl CachedClient {
    fn clone_client(&self) -> CachedClient {
        match self {
            CachedClient::SshShell { opened_at, client } => CachedClient::SshShell { opened_at: *opened_at, client: client.clone() },
            CachedClient::Sftp { opened_at, client } => CachedClient::Sftp { opened_at: *opened_at, client: client.clone() },
            CachedClient::TelnetShell { opened_at, client } => CachedClient::TelnetShell { opened_at: *opened_at, client: client.clone() },
            CachedClient::Ftp { opened_at, client } => CachedClient::Ftp { opened_at: *opened_at, client: client.clone() },
        }
    }
}
