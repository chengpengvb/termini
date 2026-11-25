use sea_orm::JsonValue;
use termini_macros::base_target;
use serde::{Deserialize, Serialize};

#[base_target(table = "ai_chat_item", entity_type = 22)]
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Model {
    pub r#type: i16,         // 类型
    pub chat_id: String,     // 所属聊天id
    pub timestamp: i64,      // 创建时间戳
    pub context: JsonValue,  // 内容 json字符串 RespEventAgent
    pub pid: Option<String>, //父id
}

pub type AiChatItem = Model;
