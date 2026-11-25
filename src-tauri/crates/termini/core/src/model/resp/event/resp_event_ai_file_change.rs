use crate::model::resp::resp_ws_context::RespSocketContext;

pub enum RespEventAiFileChange {
    OnChanged(String),
}

impl RespEventAiFileChange {
    pub const ON_CHANGED: i16 = 97;

    pub fn to_type(&self) -> i16 {
        match self {
            RespEventAiFileChange::OnChanged(_) => Self::ON_CHANGED,
        }
    }

    pub fn chat_id(&self) -> &str {
        match self {
            RespEventAiFileChange::OnChanged(chat_id) => chat_id,
        }
    }

    pub fn to_resp(&self) -> RespSocketContext {
        let chat_id = self.chat_id().to_string();
        RespSocketContext { r#type: self.to_type(), session_id: chat_id.clone(), payload: serde_json::to_value(chat_id).ok() }
    }
}
