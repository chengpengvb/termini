use std::sync::Arc;
use dashmap::DashMap;
use dashmap::DashSet;
use tokio::sync::watch;
use summer::plugin::Service;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use rig::completion::Message;
use termini_macros::base_app;
use crate::util::id_factory::uuid;
use crate::base::base_app::BaseApp;
use tokio_util::sync::CancellationToken;
use crate::module::ai::dto::ChatSessionCacheEntry;
use crate::model::req::req_chat_context::ConfirmMode;
use crate::model::dto::chat_memory::{ChatScene, MemoryTurn};

fn create_chats() -> Arc<DashMap<String, ChatSessionCacheEntry>> {
    Arc::new(DashMap::new())
}

fn create_deleting_set() -> Arc<DashSet<String>> {
    Arc::new(DashSet::new())
}

fn create_context_tokens() -> Arc<DashMap<String, u64>> {
    Arc::new(DashMap::new())
}

#[base_app(sort = 765)]
#[derive(Clone, Service)]
pub struct ChatSessionCacheManager {
    #[inject(func = create_deleting_set())]
    deleting_chat_ids: Arc<DashSet<String>>,

    #[inject(func = create_chats())]
    chats: Arc<DashMap<String, ChatSessionCacheEntry>>,

    #[inject(func = create_context_tokens())]
    context_tokens_by_chat: Arc<DashMap<String, u64>>,
}

#[async_trait]
impl BaseApp for ChatSessionCacheManager {
    fn name(&self) -> &'static str {
        "ChatSessionCacheManager"
    }
}

impl ChatSessionCacheManager {
    pub fn insert(&self, req_id: String, entry: ChatSessionCacheEntry) {
        self.chats.insert(req_id, entry);
    }

    pub fn remove(&self, req_id: &str) {
        self.chats.remove(req_id);
    }

    pub fn record_context_tokens(&self, req_id: &str, tokens: u64) {
        let Some(entry) = self.chats.get(req_id) else {
            return;
        };
        if tokens == 0 {
            return;
        }
        entry.run_trace.lock().context_tokens = tokens;
        self.context_tokens_by_chat.insert(entry.context.ai_chat_id.clone(), tokens);
    }

    pub fn context_tokens(&self, chat_id: &str) -> u64 {
        self.context_tokens_by_chat.get(chat_id).map(|value| *value).unwrap_or(0)
    }

    pub fn active_req_ids(&self) -> Vec<String> {
        self.chats.iter().map(|entry| entry.key().clone()).collect()
    }

    pub fn is_active_empty(&self) -> bool {
        self.chats.is_empty()
    }

    pub fn get(&self, req_id: &str) -> Option<dashmap::mapref::one::Ref<'_, String, ChatSessionCacheEntry>> {
        self.chats.get(req_id)
    }

    pub fn chat_id(&self, req_id: &str) -> Option<String> {
        self.chats.get(req_id).map(|entry| entry.context.ai_chat_id.clone())
    }

    pub fn parent_req_id(&self, req_id: &str) -> Option<String> {
        self.chats.get(req_id).and_then(|entry| entry.parent_req_id.lock().clone())
    }

    pub fn get_req_id_by_chat_id(&self, chat_id: &str) -> Option<String> {
        for entry in self.chats.iter() {
            if entry.value().context.ai_chat_id == chat_id {
                return Some(entry.key().clone());
            }
        }
        None
    }

    pub fn confirm_mode(&self, req_id: &str) -> Option<ConfirmMode> {
        self.chats.get(req_id).map(|entry| *entry.confirm_mode.borrow())
    }

    pub fn watch_confirm_mode(&self, req_id: &str) -> Option<watch::Receiver<ConfirmMode>> {
        self.chats.get(req_id).map(|entry| entry.confirm_mode.subscribe())
    }

    pub fn set_confirm_mode(&self, chat_id: &str, mode: ConfirmMode) -> Vec<String> {
        let mut changed = Vec::new();
        for entry in self.chats.iter() {
            if entry.value().context.ai_chat_id != chat_id {
                continue;
            }
            let modified = entry.value().confirm_mode.send_if_modified(|current| {
                if *current == mode {
                    return false;
                }
                *current = mode;
                true
            });
            if modified {
                changed.push(entry.key().clone());
            }
        }
        changed
    }

    pub fn cancel(&self, req_id: &str) -> Result<()> {
        let Some(entry) = self.chats.get(req_id) else {
            return Err(anyhow!("unknown req_id: {}", req_id));
        };
        for run_tokens in &entry.tool_cancel_tokens {
            for tool_token in run_tokens.value() {
                tool_token.value().cancel();
            }
        }
        entry.cancel_token.cancel();
        Ok(())
    }

    pub fn insert_tool_cancel(&self, req_id: &str, run_id: String, call_id: String) -> Result<()> {
        let Some(entry) = self.chats.get(req_id) else {
            return Err(anyhow!("unknown req_id: {}", req_id));
        };
        entry.tool_cancel_tokens.entry(run_id).or_default().insert(call_id, CancellationToken::new());
        Ok(())
    }

    pub fn remove_tool_cancel(&self, req_id: &str, run_id: &str, call_id: &str) -> Result<()> {
        let Some(entry) = self.chats.get(req_id) else {
            return Err(anyhow!("unknown req_id: {}", req_id));
        };
        let Some(run_tokens) = entry.tool_cancel_tokens.get(run_id) else {
            return Err(anyhow!("unknown run_id: {}", run_id));
        };
        if run_tokens.remove(call_id).is_none() {
            return Err(anyhow!("unknown call_id: {}", call_id));
        }
        drop(run_tokens);
        if let Some(run_tokens) = entry.tool_cancel_tokens.get(run_id) {
            if run_tokens.is_empty() {
                drop(run_tokens);
                entry.tool_cancel_tokens.remove(run_id);
            }
        }
        Ok(())
    }

    pub fn get_tool_cancel(&self, req_id: &str, run_id: &str, call_id: &str) -> Option<CancellationToken> {
        let entry = self.chats.get(req_id)?;
        let run_tokens = entry.tool_cancel_tokens.get(run_id)?;
        run_tokens.get(call_id).map(|token| token.clone())
    }

    pub fn cancel_tool(&self, req_id: &str, run_id: &str, call_id: &str) -> Result<()> {
        let Some(token) = self.get_tool_cancel(req_id, run_id, call_id) else {
            return Err(anyhow!("unknown tool cancel: req_id={} run_id={} call_id={}", req_id, run_id, call_id));
        };
        token.cancel();
        let _ = self.remove_tool_cancel(req_id, run_id, call_id);
        Ok(())
    }

    pub fn ensure_tool_item_id(&self, req_id: &str, call_id: &str) -> Result<String> {
        let Some(entry) = self.chats.get(req_id) else {
            return Err(anyhow!("unknown req_id: {}", req_id));
        };
        let item_id = entry.tool_item_ids.entry(call_id.to_string()).or_insert_with(uuid).clone();
        Ok(item_id)
    }

    pub fn set_tool_id(&self, req_id: &str, call_id: &str, tool_id: &str) {
        let Some(entry) = self.chats.get(req_id) else {
            return;
        };
        entry.tool_ids.insert(call_id.to_string(), tool_id.to_string());
    }

    pub fn get_tool_id(&self, req_id: &str, call_id: &str) -> Option<String> {
        let entry = self.chats.get(req_id)?;
        entry.tool_ids.get(call_id).map(|tool_id| tool_id.clone())
    }

    pub fn get_tool_item_id(&self, req_id: &str, call_id: &str) -> Option<String> {
        let entry = self.chats.get(req_id)?;
        entry.tool_item_ids.get(call_id).map(|item_id| item_id.clone())
    }

    pub fn remove_tool_item_id(&self, req_id: &str, call_id: &str) -> Option<String> {
        let entry = self.chats.get(req_id)?;
        entry.tool_item_ids.remove(call_id).map(|(_, item_id)| item_id)
    }

    pub fn mark_tool_done(&self, req_id: &str, call_id: &str) -> Result<bool> {
        let Some(entry) = self.chats.get(req_id) else {
            return Err(anyhow!("unknown req_id: {}", req_id));
        };
        Ok(entry.tool_done_call_ids.insert(call_id.to_string(), ()).is_none())
    }

    pub fn add_changed_path(&self, req_id: &str, path: &str) {
        let Some(entry) = self.chats.get(req_id) else {
            return;
        };
        let mut paths = entry.changed_paths.lock();
        if paths.iter().any(|existing| existing == path) {
            return;
        }
        paths.push(path.to_string());
    }

    pub fn changed_paths(&self, req_id: &str) -> Vec<String> {
        let Some(entry) = self.chats.get(req_id) else {
            return Vec::new();
        };
        let paths = entry.changed_paths.lock().clone();
        paths
    }

    pub fn add_agents_doc(&self, req_id: &str, path: &str, content: String) {
        let Some(entry) = self.chats.get(req_id) else {
            return;
        };
        let mut docs = entry.agents_docs.lock();
        if docs.iter().any(|(existing, _)| existing == path) {
            return;
        }
        docs.push((path.to_string(), content));
    }

    pub fn agents_docs(&self, req_id: &str) -> Vec<(String, String)> {
        let Some(entry) = self.chats.get(req_id) else {
            return Vec::new();
        };
        let docs = entry.agents_docs.lock().clone();
        docs
    }

    pub fn set_todo_snapshot(&self, req_id: &str, rendered: String) {
        let Some(entry) = self.chats.get(req_id) else {
            return;
        };
        *entry.todo_snapshot.lock() = Some(rendered);
    }

    pub fn todo_snapshot(&self, req_id: &str) -> Option<String> {
        let entry = self.chats.get(req_id)?;
        let snapshot = entry.todo_snapshot.lock().clone();
        snapshot
    }

    pub fn set_addon_messages(&self, req_id: &str, messages: Vec<Message>) {
        let Some(entry) = self.chats.get(req_id) else {
            return;
        };
        *entry.addon_messages.lock() = messages;
    }

    pub fn summary(&self, req_id: &str) -> String {
        let Some(entry) = self.chats.get(req_id) else {
            return String::new();
        };
        let summary = entry.summary.lock().clone();
        summary
    }

    pub fn set_summary(&self, req_id: &str, summary: String) {
        let Some(entry) = self.chats.get(req_id) else {
            return;
        };
        *entry.summary.lock() = summary;
    }

    pub fn turns(&self, req_id: &str) -> Vec<MemoryTurn> {
        let Some(entry) = self.chats.get(req_id) else {
            return Vec::new();
        };
        let turns = entry.turns.lock().clone();
        turns
    }

    pub fn set_turns(&self, req_id: &str, turns: Vec<MemoryTurn>) {
        let Some(entry) = self.chats.get(req_id) else {
            return;
        };
        *entry.turns.lock() = turns;
    }

    pub fn scene(&self, req_id: &str) -> Option<ChatScene> {
        let entry = self.chats.get(req_id)?;
        let scene = entry.scene.lock().clone();
        scene
    }

    pub fn set_scene(&self, req_id: &str, scene: ChatScene) {
        let Some(entry) = self.chats.get(req_id) else {
            return;
        };
        *entry.scene.lock() = Some(scene);
    }

    pub fn mark_deleting(&self, chat_id: &str) {
        self.deleting_chat_ids.insert(chat_id.to_string());
    }

    pub fn is_deleting(&self, chat_id: &str) -> bool {
        self.deleting_chat_ids.contains(chat_id)
    }

    pub fn unmark_deleting(&self, chat_id: &str) {
        self.deleting_chat_ids.remove(chat_id);
    }
}
