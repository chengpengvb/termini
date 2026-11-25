use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct RespSocketContext {
    pub r#type: i16,
    pub session_id: String,
    pub payload: Option<serde_json::Value>,
}
