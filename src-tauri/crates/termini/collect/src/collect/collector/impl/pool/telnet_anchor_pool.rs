use std::sync::Arc;
use dashmap::DashMap;
use parking_lot::Mutex;
use summer::tracing::debug;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use crate::util::id_factory::uuid;
use dashmap::mapref::entry::Entry;
use std::time::{Duration, Instant};
use crate::base::base_app::{AppContext, BaseApp};
use crate::service::service_config::ConfigService;
use crate::module::common::util::task_hub::TaskGroup;
use crate::module::telnet_shell::client_telnet_shell::ClientTelnetShell;
use crate::module::common::context::context_telnet_shell::ContextTelnetShell;
use crate::model::config::{JOB_SSH_POOL_IDLE_SECS_DEFAULT, JOB_SSH_POOL_MAX_SIZE_DEFAULT};
use crate::module::telnet::gate::telnet_session_gate::{TelnetSessionGate, TELNET_GATE_WAIT};

const SWEEP_INTERVAL: Duration = Duration::from_secs(1);

struct TelnetPoolClient {
    created_at: Instant,
    last_used: Mutex<Instant>,
    client: Arc<ClientTelnetShell>,
}

fn create_telnet_clients() -> Arc<DashMap<String, Arc<TelnetPoolClient>>> {
    Arc::new(DashMap::new())
}

#[base_app(sort = 255)]
#[derive(Clone, Service)]
pub struct TelnetAnchorPool {
    #[inject(component)]
    config_service: ConfigService,

    #[inject(func = create_telnet_clients())]
    telnet_clients: Arc<DashMap<String, Arc<TelnetPoolClient>>>,
}

#[async_trait]
impl BaseApp for TelnetAnchorPool {
    fn name(&self) -> &'static str {
        "TelnetAnchorPool"
    }

    async fn start(&self, _ctx: &AppContext) -> anyhow::Result<()> {
        let pool = self.clone();
        TaskGroup::global().spawn("telnet-anchor-sweep", async move {
            pool.sweep_loop().await;
        });
        Ok(())
    }

    async fn on_exit(&self, _ctx: &AppContext) -> anyhow::Result<()> {
        let keys: Vec<String> = self.telnet_clients.iter().map(|entry| entry.key().clone()).collect();
        for key in keys {
            self.close_client(&key, "exit").await;
        }
        Ok(())
    }
}

impl TelnetAnchorPool {
    pub async fn exec(&self, context: &ContextTelnetShell, script: String, exec_timeout: Duration, wait: bool) -> anyhow::Result<Option<String>> {
        let key = Self::client_key(context)?;
        if let Some(client) = self.hit(&key) {
            let result = client.exec(script, exec_timeout, None).await;
            return match result {
                Ok(output) => {
                    self.touch(&key);
                    Ok(Some(output))
                }
                Err(error) => {
                    self.close_client(&key, "exec failed").await;
                    Err(error)
                }
            };
        }
        let Some(client) = self.open_client(&key, context, wait).await? else {
            return Ok(None);
        };
        let output = client.exec(script, exec_timeout, None).await?;
        self.touch(&key);
        Ok(Some(output))
    }

    fn hit(&self, key: &str) -> Option<Arc<ClientTelnetShell>> {
        let entry = self.telnet_clients.get(key)?;
        *entry.last_used.lock() = Instant::now();
        Some(entry.client.clone())
    }

    fn touch(&self, key: &str) {
        if let Some(entry) = self.telnet_clients.get(key) {
            *entry.last_used.lock() = Instant::now();
        }
    }

    async fn open_client(&self, key: &str, context: &ContextTelnetShell, wait: bool) -> anyhow::Result<Option<Arc<ClientTelnetShell>>> {
        let host = context.device.read().host.clone();
        let (port, limit) = {
            let config = context.config.read();
            (config.get_base()?.port, config.resolve_max_sessions())
        };
        let permit = if wait {
            TelnetSessionGate::global().acquire(&host, port, limit, TELNET_GATE_WAIT).await?
        } else {
            match TelnetSessionGate::global().try_acquire(&host, port, limit) {
                Some(permit) => permit,
                None => return Ok(None),
            }
        };
        let client = Arc::new(ClientTelnetShell::new(uuid()));
        client.open_with_permit(context, permit).await?;

        let now = Instant::now();
        let pooled = Arc::new(TelnetPoolClient { created_at: now, client: client.clone(), last_used: Mutex::new(now) });
        let existing = match self.telnet_clients.entry(key.to_string()) {
            Entry::Occupied(occupied) => Some(occupied.get().client.clone()),
            Entry::Vacant(vacant) => {
                vacant.insert(pooled);
                None
            }
        };
        if let Some(existing) = existing {
            let _ = client.close().await;
            return Ok(Some(existing));
        }
        debug!("[telnet] client {} created, size={}", key, self.telnet_clients.len());
        self.evict_overflow().await;
        Ok(Some(client))
    }

    fn client_key(context: &ContextTelnetShell) -> anyhow::Result<String> {
        let host = context.device.read().host.clone();
        if host.trim().is_empty() {
            anyhow::bail!("device Telnet host address is empty");
        }
        let port = context.config.read().get_base()?.port;
        Ok(TelnetSessionGate::key(&host, port))
    }

    async fn evict_overflow(&self) {
        let max_size = match self.config_service.select_config().await {
            Some(config) => config.resolve_job_ssh_pool_max_size(),
            None => JOB_SSH_POOL_MAX_SIZE_DEFAULT as usize,
        };
        if self.telnet_clients.len() <= max_size {
            return;
        }
        let mut ordered: Vec<(String, Instant, Instant)> = self.telnet_clients.iter().map(|entry| (entry.key().clone(), *entry.last_used.lock(), entry.created_at)).collect();
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
        if self.telnet_clients.is_empty() {
            return;
        }
        let crowded: Vec<String> = self.telnet_clients.iter().filter(|entry| TelnetSessionGate::global().has_waiters(entry.key())).map(|entry| entry.key().clone()).collect();
        for key in crowded {
            self.close_client(&key, "gate waiter").await;
        }
        let idle = match self.config_service.select_config().await {
            Some(config) => config.resolve_job_ssh_pool_idle(),
            None => Duration::from_secs(JOB_SSH_POOL_IDLE_SECS_DEFAULT as u64),
        };
        let expired: Vec<String> = self.telnet_clients.iter().filter(|entry| entry.last_used.lock().elapsed() >= idle).map(|entry| entry.key().clone()).collect();
        for key in expired {
            self.close_client(&key, "idle").await;
        }
    }

    async fn close_client(&self, key: &str, reason: &str) {
        let Some((_, entry)) = self.telnet_clients.remove(key) else {
            return;
        };
        let _ = entry.client.close().await;
        debug!("[telnet] client {} closed({}), size={}", key, reason, self.telnet_clients.len());
    }
}
