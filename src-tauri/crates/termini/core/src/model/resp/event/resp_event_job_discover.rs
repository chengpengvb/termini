use crate::model::resp::resp_ws_context::RespSocketContext;

pub enum RespEventJobDiscover {
    OnChanged(String),
}

impl RespEventJobDiscover {
    pub const ON_CHANGED: i16 = 92;

    pub fn to_type(&self) -> i16 {
        match self {
            RespEventJobDiscover::OnChanged(_) => Self::ON_CHANGED,
        }
    }

    pub fn discover_id(&self) -> &str {
        match self {
            RespEventJobDiscover::OnChanged(id) => id,
        }
    }

    pub fn to_resp(&self) -> RespSocketContext {
        let discover_id = self.discover_id().to_string();
        RespSocketContext { r#type: self.to_type(), session_id: discover_id.clone(), payload: serde_json::to_value(discover_id).ok() }
    }
}
