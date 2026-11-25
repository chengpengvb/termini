use std::sync::Arc;
use dashmap::DashMap;
use std::time::Duration;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use crate::util::time_util::now_millis;
use crate::base::base_app::{AppContext, BaseApp};
use crate::module::common::util::task_hub::TaskGroup;

const SWEEP_INTERVAL: Duration = Duration::from_secs(1);
pub const DEFAULT_INTERVAL: Duration = Duration::from_secs(60 * 15);

struct ThrottleEntry {
    text: String,
    at_millis: i64,
    interval_millis: i64,
}

fn create_entries() -> Arc<DashMap<String, ThrottleEntry>> {
    Arc::new(DashMap::new())
}

#[base_app(sort = 110)]
#[derive(Clone, Service)]
pub struct LogThrottleService {
    #[inject(func = create_entries())]
    entries: Arc<DashMap<String, ThrottleEntry>>,
}

#[async_trait]
impl BaseApp for LogThrottleService {
    fn name(&self) -> &'static str {
        "LogThrottleService"
    }

    async fn start(&self, _ctx: &AppContext) -> anyhow::Result<()> {
        let service = self.clone();
        TaskGroup::global().spawn("log-throttle-sweep", async move {
            service.sweep_loop().await;
        });
        Ok(())
    }

    async fn on_exit(&self, _ctx: &AppContext) -> anyhow::Result<()> {
        self.entries.clear();
        Ok(())
    }
}

impl LogThrottleService {
    pub fn allow(&self, key: String, text: &str) -> bool {
        self.allow_every(key, text, DEFAULT_INTERVAL)
    }

    pub fn allow_every(&self, key: String, text: &str, interval: Duration) -> bool {
        let now = now_millis();
        let interval_millis = interval.as_millis() as i64;
        let mut entry = self.entries.entry(key).or_insert_with(|| ThrottleEntry { text: String::new(), at_millis: 0, interval_millis });
        if entry.text == text && now - entry.at_millis < entry.interval_millis {
            return false;
        }
        entry.text = text.to_string();
        entry.at_millis = now;
        entry.interval_millis = interval_millis;
        true
    }

    async fn sweep_loop(&self) {
        let mut ticker = tokio::time::interval(SWEEP_INTERVAL);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            ticker.tick().await;
            self.sweep_expired();
        }
    }

    fn sweep_expired(&self) {
        if self.entries.is_empty() {
            return;
        }
        let now = now_millis();
        self.entries.retain(|_, entry| now - entry.at_millis < entry.interval_millis);
    }
}
