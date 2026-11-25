use crate::model::resp::resp_ws_context::RespSocketContext;

pub enum RespEventSerial {
    OnOpen,
    OnClose,
    OnError(String),
    OnData(Vec<u8>),
}

impl RespEventSerial {
    pub fn to_type(&self) -> i16 {
        match self {
            RespEventSerial::OnOpen => 100,
            RespEventSerial::OnClose => 101,
            RespEventSerial::OnError(_) => 102,
            RespEventSerial::OnData(_) => 103,
        }
    }
}

pub enum RespEventSerialPorts {
    OnChanged(Vec<String>),
}

impl RespEventSerialPorts {
    pub const ON_CHANGED: i16 = 120;

    pub fn to_resp(&self) -> RespSocketContext {
        let RespEventSerialPorts::OnChanged(ports) = self;
        RespSocketContext { r#type: Self::ON_CHANGED, session_id: String::new(), payload: serde_json::to_value(ports).ok() }
    }
}
