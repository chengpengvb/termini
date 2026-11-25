use dashmap::DashMap;
use parking_lot::Mutex;
use std::time::Duration;
use tokio::sync::Notify;
use summer::tracing::debug;
use std::sync::{Arc, OnceLock};
use std::sync::atomic::{AtomicUsize, Ordering};

pub const TELNET_GATE_WAIT: Duration = Duration::from_secs(60);

static TELNET_SESSION_GATE: OnceLock<TelnetSessionGate> = OnceLock::new();

struct GateEntry {
    notify: Notify,
    active: Mutex<usize>,
    waiting: AtomicUsize,
}

struct WaitGuard(Arc<GateEntry>);

impl Drop for WaitGuard {
    fn drop(&mut self) {
        self.0.waiting.fetch_sub(1, Ordering::Relaxed);
    }
}

pub struct TelnetSessionGate {
    entries: DashMap<String, Arc<GateEntry>>,
}

pub struct TelnetPermit {
    key: String,
    entry: Arc<GateEntry>,
}

impl TelnetPermit {
    pub fn key(&self) -> &str {
        &self.key
    }
}

impl Drop for TelnetPermit {
    fn drop(&mut self) {
        {
            let mut active = self.entry.active.lock();
            *active = active.saturating_sub(1);
        }
        self.entry.notify.notify_one();
        debug!("[telnet] {} released", self.key);
    }
}

impl TelnetSessionGate {
    pub fn global() -> &'static TelnetSessionGate {
        TELNET_SESSION_GATE.get_or_init(|| TelnetSessionGate { entries: DashMap::new() })
    }

    pub async fn acquire(&self, host: &str, port: u16, limit: usize, wait: Duration) -> anyhow::Result<TelnetPermit> {
        let key = Self::key(host, port);
        let entry = self.entry(&key);
        entry.waiting.fetch_add(1, Ordering::Relaxed);
        let _wait_guard = WaitGuard(entry.clone());
        let deadline = tokio::time::Instant::now() + wait;
        loop {
            let notified = entry.notify.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            if let Some(permit) = Self::take(&key, &entry, limit) {
                return Ok(permit);
            }
            if tokio::time::timeout_at(deadline, notified).await.is_err() {
                anyhow::bail!("telnet {} reached the concurrent connection limit {}, no slot acquired after waiting {}s", key, limit, wait.as_secs());
            }
        }
    }

    pub fn try_acquire(&self, host: &str, port: u16, limit: usize) -> Option<TelnetPermit> {
        let key = Self::key(host, port);
        let entry = self.entry(&key);
        Self::take(&key, &entry, limit)
    }

    fn take(key: &str, entry: &Arc<GateEntry>, limit: usize) -> Option<TelnetPermit> {
        let mut active = entry.active.lock();
        if *active >= limit.max(1) {
            return None;
        }
        *active += 1;
        debug!("[telnet] {} acquired {}/{}", key, *active, limit.max(1));
        Some(TelnetPermit { key: key.to_string(), entry: entry.clone() })
    }

    /// 采集侧的常驻会话据此让出名额，否则 max_sessions=1 的设备永远轮不到人工登录
    pub fn has_waiters(&self, key: &str) -> bool {
        self.entries.get(key).is_some_and(|entry| entry.waiting.load(Ordering::Relaxed) > 0)
    }

    fn entry(&self, key: &str) -> Arc<GateEntry> {
        self.entries.entry(key.to_string()).or_insert_with(|| Arc::new(GateEntry { notify: Notify::new(), active: Mutex::new(0), waiting: AtomicUsize::new(0) })).value().clone()
    }

    pub fn key(host: &str, port: u16) -> String {
        format!("{}:{}", host, port)
    }
}
