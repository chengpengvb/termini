use crate::model::resp::resp_ws_context::RespSocketContext;

pub enum RespEventAiChat {
    OnChanged(String),
    OnDeleted(String),
}

impl RespEventAiChat {
    pub const ON_CHANGED: i16 = 90;
    pub const ON_DELETED: i16 = 91;

    pub fn to_type(&self) -> i16 {
        match self {
            RespEventAiChat::OnChanged(_) => Self::ON_CHANGED,
            RespEventAiChat::OnDeleted(_) => Self::ON_DELETED,
        }
    }

    pub fn chat_id(&self) -> &str {
        match self {
            RespEventAiChat::OnChanged(chat_id) => chat_id,
            RespEventAiChat::OnDeleted(chat_id) => chat_id,
        }
    }

    pub fn to_resp(&self) -> RespSocketContext {
        let chat_id = self.chat_id().to_string();
        RespSocketContext { r#type: self.to_type(), session_id: chat_id.clone(), payload: serde_json::to_value(chat_id).ok() }
    }
}
