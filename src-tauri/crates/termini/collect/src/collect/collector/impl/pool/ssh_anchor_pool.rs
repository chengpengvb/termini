use std::sync::Arc;
use dashmap::DashMap;
use parking_lot::Mutex;
use summer::tracing::debug;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use crate::util::id_factory::uuid;
use std::time::{Duration, Instant};
use summer::plugin::component::ComponentRef;
use crate::base::base_app::{AppContext, BaseApp};
use crate::service::service_config::ConfigService;
use crate::module::common::util::task_hub::TaskGroup;
use crate::module::ssh_shell::client_ssh_shell::ClientSshShell;
use crate::module::ssh::client_session_factory::ClientSessionFactory;
use crate::module::common::context::context_ssh_shell::ContextSshShell;
use crate::model::config::{JOB_SSH_POOL_IDLE_SECS_DEFAULT, JOB_SSH_POOL_MAX_SIZE_DEFAULT};

const SWEEP_INTERVAL: Duration = Duration::from_secs(1);

struct SshPoolClient {
    created_at: Instant,
    last_used: Mutex<Instant>,
    client: Arc<ClientSshShell>,
}

fn create_ssh_clients() -> Arc<DashMap<String, Arc<SshPoolClient>>> {
    Arc::new(DashMap::new())
}

#[base_app(sort = 250)]
#[derive(Clone, Service)]
pub struct SshAnchorPool {
    #[inject(component)]
    config_service: ConfigService,

    #[inject(func = create_ssh_clients())]
    ssh_clients: Arc<DashMap<String, Arc<SshPoolClient>>>,

    ssh_client_session_factory: ComponentRef<ClientSessionFactory>,
}

#[async_trait]
impl BaseApp for SshAnchorPool {
    fn name(&self) -> &'static str {
        "SshAnchorPool"
    }

    async fn start(&self, _ctx: &AppContext) -> anyhow::Result<()> {
        let pool = self.clone();
        TaskGroup::global().spawn("ssh-anchor-sweep", async move {
            pool.sweep_loop().await;
        });
        Ok(())
    }

    async fn on_exit(&self, _ctx: &AppContext) -> anyhow::Result<()> {
        let keys: Vec<String> = self.ssh_clients.iter().map(|entry| entry.key().clone()).collect();
        for key in keys {
            self.close_client(&key, "exit").await;
        }
        Ok(())
    }
}

impl SshAnchorPool {
    pub async fn touch(&self, context: &ContextSshShell) {
        let Some(key) = Self::client_key(context) else {
            return;
        };
        if let Some(entry) = self.ssh_clients.get(&key) {
            *entry.last_used.lock() = Instant::now();
            return;
        }
        let client = Arc::new(ClientSshShell::new(uuid(), self.ssh_client_session_factory.clone()));
        if let Err(error) = client.open(context, Some(true)).await {
            debug!("[ssh] client {} open failed: {}", key, error);
            return;
        }
        let now = Instant::now();
        self.ssh_clients.insert(key.clone(), Arc::new(SshPoolClient { created_at: now, client, last_used: Mutex::new(now) }));
        debug!("[ssh] client {} created, size={}", key, self.ssh_clients.len());
        self.evict_overflow().await;
    }

    fn client_key(context: &ContextSshShell) -> Option<String> {
        let host = context.device.read().host.clone();
        if host.trim().is_empty() {
            return None;
        }
        let port = context.config.read().get_base().ok()?.port;
        Some(format!("{}:{}", host, port))
    }

    async fn evict_overflow(&self) {
        let max_size = match self.config_service.select_config().await {
            None => JOB_SSH_POOL_MAX_SIZE_DEFAULT as usize,
            Some(config) => config.resolve_job_ssh_pool_max_size(),
        };
        if self.ssh_clients.len() <= max_size {
            return;
        }
        let mut ordered: Vec<(String, Instant, Instant)> = self.ssh_clients.iter().map(|entry| (entry.key().clone(), *entry.last_used.lock(), entry.created_at)).collect();
        ordered.sort_by(|left, right| left.1.cmp(&right.1).then(left.2.cmp(&right.2)));
        let overflow = ordered.len().saturating_sub(max_size);
        for (key, _, _) in ordered.into_iter().take(overflow) {
            self.close_client(&key, "overflow").await;
        }
    }

    async fn sweep_loop(&self) {
        let mut ticker = tokio::time::interval(SWEEP_INTERVAL);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            ticker.tick().await;
            self.sweep_expired().await;
        }
    }

    async fn sweep_expired(&self) {
        if self.ssh_clients.is_empty() {
            return;
        }
        let idle = match self.config_service.select_config().await {
            Some(config) => config.resolve_job_ssh_pool_idle(),
            None => Duration::from_secs(JOB_SSH_POOL_IDLE_SECS_DEFAULT as u64),
        };
        let expired: Vec<String> = self.ssh_clients.iter().filter(|entry| entry.last_used.lock().elapsed() >= idle).map(|entry| entry.key().clone()).collect();
        for key in expired {
            self.close_client(&key, "idle").await;
        }
    }

    async fn close_client(&self, key: &str, reason: &str) {
        let Some((_, entry)) = self.ssh_clients.remove(key) else {
            return;
        };
        let _ = entry.client.close().await;
        debug!("[ssh] client {} closed({}), size={}", key, reason, self.ssh_clients.len());
    }
}
