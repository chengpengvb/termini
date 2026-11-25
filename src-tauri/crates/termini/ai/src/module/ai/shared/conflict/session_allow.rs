use std::sync::Arc;
use dashmap::DashMap;
use summer::tracing::info;
use std::collections::HashSet;

#[derive(Clone, Default)]
pub struct SessionAllowStore {
    allows: Arc<DashMap<String, HashSet<String>>>,
}

impl SessionAllowStore {
    pub fn is_allowed(&self, chat_id: &str, key: &str) -> bool {
        self.allows.get(chat_id).is_some_and(|keys| keys.contains(key))
    }

    pub fn remember(&self, chat_id: &str, key: &str) {
        self.allows.entry(chat_id.to_string()).or_default().insert(key.to_string());
        info!("[ai] 本会话内记住同前缀放行 chat_id={} key={}", chat_id, key);
    }

    pub fn forget(&self, chat_id: &str) {
        self.allows.remove(chat_id);
    }
}
