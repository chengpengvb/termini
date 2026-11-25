use std::pin::Pin;
use std::sync::Arc;
use parking_lot::Mutex;
use std::future::Future;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use crate::base::base_app::BaseApp;
use crate::web::ws::ws_manager::WsManager;
use std::sync::atomic::{AtomicBool, Ordering};
use crate::model::resp::resp_ws_context::RespSocketContext;
use crate::model::resp::event::resp_event_agent::RespEventAgent;
use crate::module::ai::shared::wake_bus::{WakeBus, WakeSourceKind};
use crate::module::common::dto::shell_chunk_callback::ShellChunkCallback;
use crate::module::ai::tools::common::shell::local_shell_registry::LocalShellRegistry;
use crate::model::resp::resp_agent::{RespAgentShellDaemon, RespAgentShellDaemonChunk, RespAgentShellDaemonRow};

#[base_app(sort = 761)]
#[derive(Clone, Service)]
pub struct ShellDaemonPush {
    #[inject(component)]
    wake_bus: WakeBus,

    #[inject(component)]
    ws_manager: WsManager,

    #[inject(component)]
    local_shell_registry: LocalShellRegistry,
}

#[async_trait]
impl BaseApp for ShellDaemonPush {
    fn name(&self) -> &'static str {
        "ShellDaemonPush"
    }
}

impl ShellDaemonPush {
    pub fn rows(&self, chat_id: &str) -> Vec<RespAgentShellDaemonRow> {
        let mut rows: Vec<RespAgentShellDaemonRow> = self.local_shell_registry.list_running_resident(chat_id).iter().map(|entry| entry.to_daemon_row()).collect();
        rows.extend(self.wake_bus.list_kind(chat_id, WakeSourceKind::Loop).iter().map(|source| source.to_daemon_row()));
        rows.sort_by_key(|row| row.started_ms);
        rows
    }

    pub fn push_daemon_list(&self, chat_id: &str) {
        self.broadcast(chat_id, RespEventAgent::OnShellDaemon(RespAgentShellDaemon { chat_id: chat_id.to_string(), daemons: self.rows(chat_id) }));
    }

    pub fn push_daemon_chunk(&self, chat_id: &str, process_id: &str, delta: String) {
        self.broadcast(chat_id, RespEventAgent::OnShellDaemonChunk(RespAgentShellDaemonChunk { delta, chat_id: chat_id.to_string(), process_id: process_id.to_string() }));
    }

    pub fn wrap_chunk_callback(&self, on_chunk: ShellChunkCallback, buffer: Arc<Mutex<String>>, chat_id: String, process_id: String, resident: Arc<AtomicBool>) -> ShellChunkCallback {
        let push = self.clone();
        Arc::new(move |bytes: Vec<u8>| {
            let push = push.clone();
            let buffer = buffer.clone();
            let chat_id = chat_id.clone();
            let on_chunk = on_chunk.clone();
            let resident = resident.clone();
            let process_id = process_id.clone();
            Box::pin(async move {
                let before = buffer.lock().len();
                on_chunk(bytes).await;
                if !resident.load(Ordering::Relaxed) {
                    return;
                }
                let delta = buffer.lock()[before..].to_string();
                if delta.is_empty() {
                    return;
                }
                push.push_daemon_chunk(&chat_id, &process_id, delta);
            }) as Pin<Box<dyn Future<Output = ()> + Send>>
        })
    }

    fn broadcast(&self, chat_id: &str, event: RespEventAgent) {
        let item = event.to_ai_chat_item(chat_id.to_string(), None, None);
        let resp = RespSocketContext { r#type: item.r#type, session_id: item.chat_id.clone(), payload: serde_json::to_value(&item).ok() };
        self.ws_manager.broadcast(chat_id, &resp);
    }
}
