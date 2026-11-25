use termini_macros::base_target;
use serde::{Deserialize, Serialize};

pub const FILE_CHANGE_STATE_PENDING: i16 = 0;
pub const FILE_CHANGE_STATE_ACCEPTED: i16 = 1;
pub const FILE_CHANGE_STATE_REVERTED: i16 = 2;

#[base_target(table = "ai_file_change", entity_type = 29)]
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Model {
    pub state: i16,
    pub path: String,
    pub req_id: String,
    pub timestamp: i64,
    pub chat_id: String,
    pub after_text: String,
    pub device_id: Option<String>,
    pub before_text: Option<String>,
}

pub type AiFileChange = Model;
