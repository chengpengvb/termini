use crate::model::resp::resp_ws_context::RespSocketContext;

pub enum RespEventJobAlert {
    OnChanged(String),
}

impl RespEventJobAlert {
    pub const ON_CHANGED: i16 = 96;

    pub fn to_type(&self) -> i16 {
        match self {
            RespEventJobAlert::OnChanged(_) => Self::ON_CHANGED,
        }
    }

    pub fn alert_id(&self) -> &str {
        match self {
            RespEventJobAlert::OnChanged(id) => id,
        }
    }

    pub fn to_resp(&self) -> RespSocketContext {
        let alert_id = self.alert_id().to_string();
        RespSocketContext { r#type: self.to_type(), session_id: alert_id.clone(), payload: serde_json::to_value(alert_id).ok() }
    }
}
