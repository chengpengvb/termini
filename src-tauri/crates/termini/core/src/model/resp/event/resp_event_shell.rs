pub enum RespEventShell {
    OnOpen,
    OnClose,
    OnError(String),
    OnData(Vec<u8>),
    Exit { code: Option<i32> },
    SnapshotRequest,
}

impl RespEventShell {
    pub fn to_type(&self) -> i16 {
        match self {
            RespEventShell::OnOpen => 0,
            RespEventShell::OnClose => 1,
            RespEventShell::OnError(_) => 2,
            RespEventShell::OnData(_) => 3,
            RespEventShell::Exit { .. } => 4,
            RespEventShell::SnapshotRequest => 5,
        }
    }
}
