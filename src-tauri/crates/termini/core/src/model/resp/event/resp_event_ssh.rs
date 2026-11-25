use crate::model::resp::event::resp_event_ssh_auth::{OpenSshStep, SshConnectEvent};

pub enum RespEventSsh {
    OnOpen,
    OnClose,
    OnError(String),
    OnData(Vec<u8>),
    OnOpenStep(OpenSshStep),
    OnDeviceOsType(i16),
}

impl SshConnectEvent for RespEventSsh {
    fn on_open_step(step: OpenSshStep) -> Self {
        RespEventSsh::OnOpenStep(step)
    }
    fn on_auth_banner(data: Vec<u8>) -> Option<Self> {
        Some(RespEventSsh::OnData(data))
    }
}

impl RespEventSsh {
    pub fn to_type(&self) -> i16 {
        match self {
            RespEventSsh::OnOpen => 20,
            RespEventSsh::OnClose => 21,
            RespEventSsh::OnError(_) => 22,
            RespEventSsh::OnData(_) => 23,
            RespEventSsh::OnOpenStep(_) => 24,
            RespEventSsh::OnDeviceOsType(_) => 25,
        }
    }
}
