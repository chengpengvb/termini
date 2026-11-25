use std::sync::Arc;
use std::time::Instant;
use rig::message::UserContent;
use tokio_util::sync::CancellationToken;
use crate::model::req::req_chat_context::ReqChatContext;
use crate::module::ai::dto::ai_run_context::SharedMemory;

pub struct ChatRun {
    pub req_id: String,
    pub prompt: String,
    pub llm_prompt: String,
    pub started_at: Instant,
    pub user_item_id: String,
    pub memory: SharedMemory,
    pub provide_model: String,
    pub context: Arc<ReqChatContext>,
    pub inline_images: Vec<UserContent>,
    pub cancel_token: CancellationToken,
}
