use sea_orm::JsonValue;
use termini_macros::base_target;
use serde::{Deserialize, Serialize};

#[base_target(table = "ai_chat_memory_snapshot", entity_type = 36, notify = 28)]
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Model {
    pub chat_id: String,     // 所属对话
    pub item_id: String,     // 对应的用户消息 item id
    pub messages: JsonValue, // MemoryPayload 快照(turns + summary)
    pub timestamp: i64,
}

pub type AiChatMemorySnapshot = Model;
