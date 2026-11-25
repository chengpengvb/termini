use crate::model::resp::resp_ws_context::RespSocketContext;

pub enum RespEventAccount {
    OnLogin(String),
    OnLogout,
    OnLoginError(String),
}

impl RespEventAccount {
    pub const ON_LOGIN: i16 = 140;
    pub const ON_LOGOUT: i16 = 141;
    pub const ON_LOGIN_ERROR: i16 = 143;

    pub fn to_resp(&self) -> RespSocketContext {
        match self {
            RespEventAccount::OnLogout => RespSocketContext { r#type: Self::ON_LOGOUT, session_id: "account".to_string(), payload: None },
            RespEventAccount::OnLogin(token) => RespSocketContext { r#type: Self::ON_LOGIN, session_id: "account".to_string(), payload: Some(serde_json::Value::String(token.clone())) },
            RespEventAccount::OnLoginError(message) => RespSocketContext { r#type: Self::ON_LOGIN_ERROR, session_id: "account".to_string(), payload: Some(serde_json::Value::String(message.clone())) },
        }
    }
}
