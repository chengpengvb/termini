use sea_orm::JsonValue;
use termini_macros::base_target;
use serde::{Deserialize, Serialize};

#[base_target(table = "ai_chat_memory", entity_type = 28)]
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Model {
    pub messages: JsonValue, // rig Message[] json
    pub timestamp: i64,
}

pub type AiChatMemory = Model;
