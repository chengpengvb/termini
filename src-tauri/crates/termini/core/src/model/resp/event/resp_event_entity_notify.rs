use crate::model::resp::resp_ws_context::RespSocketContext;

pub enum RespEventEntityNotify {
    OnChanged(i32),
}

impl RespEventEntityNotify {
    pub const ON_CHANGED: i16 = 105;

    pub fn to_type(&self) -> i16 {
        match self {
            RespEventEntityNotify::OnChanged(_) => Self::ON_CHANGED,
        }
    }

    pub fn entity_type(&self) -> i32 {
        match self {
            RespEventEntityNotify::OnChanged(entity_type) => *entity_type,
        }
    }

    pub fn to_resp(&self) -> RespSocketContext {
        let entity_type = self.entity_type();
        RespSocketContext {
            r#type: self.to_type(),
            session_id: entity_type.to_string(),
            payload: Some(serde_json::Value::Number(serde_json::Number::from(entity_type))),
        }
    }
}
