use anyhow::Result;
use std::sync::Arc;
use parking_lot::Mutex;
use rig::completion::Message;
use rig::message::UserContent;
use crate::util::id_factory::uuid;
use std::sync::atomic::AtomicUsize;
use tokio_util::sync::CancellationToken;
use crate::module::ai::provide::ai_agent::AiAgent;
use crate::module::ai::tools::common::task::session::subagent_addon::SubagentRunAddon;

pub type RebuildAgent = Arc<dyn Fn(&str) -> Result<AiAgent> + Send + Sync>;

pub type SharedMemory = Arc<Mutex<Vec<Message>>>;
pub type FoldGeneration = Arc<AtomicUsize>;

#[derive(Clone, Debug)]
pub struct AiRunEmit {
    pub log: bool,
    pub text: bool,
    pub pause: bool,
    pub status: bool,
    pub reasoning: bool,
    pub proxy_error: bool,
}

impl Default for AiRunEmit {
    fn default() -> Self {
        Self { text: true, reasoning: true, pause: true, status: true, log: true, proxy_error: true }
    }
}

pub struct AiRunContext {
    pub req_id: String,
    pub run_id: String,
    pub prompt: String,
    pub agent: AiAgent,
    pub emit: AiRunEmit,
    pub max_turns: usize,
    pub memory: SharedMemory,
    pub tool_concurrency: usize,
    pub chat_id: Option<String>,
    pub rebuild_agent: RebuildAgent,
    pub addon_messages: Vec<Message>,
    pub cancel_token: CancellationToken,
    pub finish_token: CancellationToken,
    pub fold_generation: FoldGeneration,
    pub prompt_images: Mutex<Vec<UserContent>>,
    pub subagent: Option<Arc<SubagentRunAddon>>,
}

impl AiRunContext {
    pub fn new(
        req_id: String,
        agent: AiAgent,
        prompt: String,
        emit: AiRunEmit,
        max_turns: usize,
        memory: SharedMemory,
        tool_concurrency: usize,
        chat_id: Option<String>,
        rebuild_agent: RebuildAgent,
        addon_messages: Vec<Message>,
        cancel_token: CancellationToken,
    ) -> Self {
        Self {
            emit,
            req_id,
            chat_id,
            agent,
            prompt,
            memory,
            max_turns,
            cancel_token,
            rebuild_agent,
            run_id: uuid(),
            subagent: None,
            addon_messages,
            tool_concurrency,
            finish_token: CancellationToken::new(),
            prompt_images: Mutex::new(Vec::new()),
            fold_generation: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn with_rebuilt_agent(&self) -> Result<Self> {
        Ok(Self {
            emit: self.emit.clone(),
            max_turns: self.max_turns,
            req_id: self.req_id.clone(),
            run_id: self.run_id.clone(),
            prompt: self.prompt.clone(),
            memory: self.memory.clone(),
            chat_id: self.chat_id.clone(),
            subagent: self.subagent.clone(),
            cancel_token: self.cancel_token.clone(),
            finish_token: self.finish_token.clone(),
            tool_concurrency: self.tool_concurrency,
            rebuild_agent: self.rebuild_agent.clone(),
            agent: (self.rebuild_agent)(&self.req_id)?,
            addon_messages: self.addon_messages.clone(),
            fold_generation: self.fold_generation.clone(),
            prompt_images: Mutex::new(self.prompt_images.lock().clone()),
        })
    }

    pub fn with_prompt_images(self, images: Vec<UserContent>) -> Self {
        *self.prompt_images.lock() = images;
        self
    }

    pub fn drop_prompt_images(&self) -> usize {
        let mut images = self.prompt_images.lock();
        let dropped = images.len();
        images.clear();
        dropped
    }

    pub fn prompt_message(&self) -> Message {
        let images = self.prompt_images.lock();
        if images.is_empty() {
            return Message::user(self.prompt.clone());
        }
        let mut parts = Vec::with_capacity(images.len() + 1);
        parts.push(UserContent::text(self.prompt.clone()));
        parts.extend(images.iter().cloned());
        Message::User { content: parts }
    }

    pub fn with_subagent(mut self, addon: Arc<SubagentRunAddon>) -> Self {
        self.subagent = Some(addon);
        self
    }
}
