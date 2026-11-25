use crate::model::resp::event::resp_event_ssh_auth::{OpenSshStep, SshConnectEvent};

pub enum RespEventForward {
    OnStarted(u16),
    OnStopped,
    OnError(String),
    OnOpenStep(OpenSshStep),
}

impl SshConnectEvent for RespEventForward {
    fn on_open_step(step: OpenSshStep) -> Self {
        RespEventForward::OnOpenStep(step)
    }
}

impl RespEventForward {
    pub fn to_type(&self) -> i16 {
        match self {
            RespEventForward::OnStarted(_) => 70,
            RespEventForward::OnStopped => 71,
            RespEventForward::OnError(_) => 72,
            RespEventForward::OnOpenStep(_) => 73,
        }
    }
}
