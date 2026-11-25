use termini_macros::base_target;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub enum AiChatType {
    Chat = 1,
    Discover = 2,
    Alert = 3,
    Plan = 4,
}

impl AiChatType {
    pub fn from_i16(value: i16) -> Option<Self> {
        match value {
            1 => Some(AiChatType::Chat),
            2 => Some(AiChatType::Discover),
            3 => Some(AiChatType::Alert),
            4 => Some(AiChatType::Plan),
            _ => None,
        }
    }
}

#[base_target(table = "ai_chat", entity_type = 18)]
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Model {
    pub sort: i16,
    pub title: String,
    pub timestamp: i64,
    pub r#type: i16,
}

pub type AiChat = Model;
