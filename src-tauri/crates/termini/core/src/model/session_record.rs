use termini_macros::base_target;
use serde::{Deserialize, Serialize};

#[base_target(table = "session_record", entity_type = 31)]
#[derive(Clone, Serialize, Deserialize)]
pub struct Model {
    pub device_name: String,
    pub protocol: String,
    pub file_name: String,
    pub width: i32,
    pub height: i32,
    pub started_at: i64,
    pub ended_at: i64,
    pub size_bytes: i64,
}

impl Default for Model {
    fn default() -> Self {
        Self {
            id: String::new(),
            device_name: String::new(),
            protocol: String::new(),
            file_name: String::new(),
            width: 0,
            height: 0,
            started_at: 0,
            ended_at: 0,
            size_bytes: 0,
        }
    }
}

pub type SessionRecord = Model;
