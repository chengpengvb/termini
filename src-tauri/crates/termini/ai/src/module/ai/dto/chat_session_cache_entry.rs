use std::sync::Arc;
use dashmap::DashMap;
use parking_lot::Mutex;
use tokio::sync::watch;
use rig::completion::Message;
use std::sync::atomic::AtomicBool;
use crate::model::ai_provide::AiProvide;
use tokio_util::sync::CancellationToken;
use crate::module::ai::dto::run_trace::RunTrace;
use crate::module::ai::tools::tool_base::ToolBase;
use crate::module::ai::dto::ai_run_context::SharedMemory;
use crate::model::dto::chat_memory::{ChatScene, MemoryTurn};
use crate::model::req::req_chat_context::{ConfirmMode, ReqChatContext};

pub struct ChatSessionCacheEntry {
    pub memory: SharedMemory,
    pub user_item_id: String,
    pub ai_provide: AiProvide,
    pub summary: Mutex<String>,
    pub token_limit: Mutex<usize>,
    pub auto_continue: AtomicBool,
    pub run_trace: Mutex<RunTrace>,
    pub context: Arc<ReqChatContext>,
    pub turns: Mutex<Vec<MemoryTurn>>,
    pub tools: Vec<Arc<dyn ToolBase>>,
    pub cancel_token: CancellationToken,
    pub scene: Mutex<Option<ChatScene>>,
    pub tool_ids: DashMap<String, String>,
    pub changed_paths: Mutex<Vec<String>>,
    pub addon_messages: Mutex<Vec<Message>>,
    pub todo_snapshot: Mutex<Option<String>>,
    pub parent_req_id: Mutex<Option<String>>,
    pub tool_item_ids: DashMap<String, String>,
    pub tool_done_call_ids: DashMap<String, ()>,
    pub confirm_mode: watch::Sender<ConfirmMode>,
    pub assist_provide: Mutex<Option<AiProvide>>,
    pub agents_docs: Mutex<Vec<(String, String)>>,
    pub tool_cancel_tokens: DashMap<String, DashMap<String, CancellationToken>>,
}

impl ChatSessionCacheEntry {
    pub fn new(
        summary: String,
        token_limit: usize,
        memory: SharedMemory,
        ai_provide: AiProvide,
        user_item_id: String,
        turns: Vec<MemoryTurn>,
        scene: Option<ChatScene>,
        context: Arc<ReqChatContext>,
        tools: Vec<Arc<dyn ToolBase>>,
        cancel_token: CancellationToken,
    ) -> Self {
        let confirm_mode = watch::channel(context.confirm_mode).0;
        Self {
            tools,
            memory,
            context,
            ai_provide,
            confirm_mode,
            cancel_token,
            user_item_id,
            tool_ids: DashMap::new(),
            scene: Mutex::new(scene),
            turns: Mutex::new(turns),
            summary: Mutex::new(summary),
            tool_item_ids: DashMap::new(),
            todo_snapshot: Mutex::new(None),
            parent_req_id: Mutex::new(None),
            assist_provide: Mutex::new(None),
            tool_cancel_tokens: DashMap::new(),
            tool_done_call_ids: DashMap::new(),
            agents_docs: Mutex::new(Vec::new()),
            token_limit: Mutex::new(token_limit),
            changed_paths: Mutex::new(Vec::new()),
            auto_continue: AtomicBool::new(false),
            addon_messages: Mutex::new(Vec::new()),
            run_trace: Mutex::new(RunTrace::default()),
        }
    }

    pub fn light_provide(&self) -> AiProvide {
        self.assist_provide.lock().clone().unwrap_or_else(|| self.ai_provide.clone())
    }

    pub fn messages_without_addon(&self) -> Vec<Message> {
        let messages = self.memory.lock().clone();
        let addons = self.addon_messages.lock().clone();
        if addons.is_empty() {
            return messages;
        }
        messages.into_iter().filter(|message| !addons.iter().any(|addon| addon == message)).collect()
    }
}
