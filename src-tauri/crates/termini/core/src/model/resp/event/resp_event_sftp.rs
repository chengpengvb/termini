use crate::model::resp::resp_sftp::*;
use crate::model::resp::event::resp_event_ssh_auth::{OpenSshStep, SshConnectEvent};

pub enum RespEventSftp {
    OnOpen(RespSftpBase),
    OnClose(RespSftpBase),
    OnError(RespSftpError),
    OnOpenStep(RespSftpOpenStep),
    OnHomeDir(RespSftpBase),
    OnPath(RespSftpBase),
    OnListDir(RespSftpEntries),
    OnStat(RespSftpEntry),
    OnRenamed(RespSftpBase),
    OnChmodded(RespSftpBase),
    OnCreated(RespSftpBase),
    OnDeleted(RespSftpBase),
    OnTransfer(RespSftpBase),
    OnDeleteProgress(RespSftpDeleteProgress),
    OnStatProperties(RespSftpStatProperties),
    OnTransferConflict(RespSftpTransferConflict),
    OnTransferCount(RespSftpTransferCount),
    OnDeleteCount(RespSftpDeleteCount),
    OnTransferProgress(RespSftpTfProgress),
    OnFileOpened(RespSftpFileOpened),
    OnExported(RespSftpExported),
}

impl SshConnectEvent for RespEventSftp {
    fn on_open_step(step: OpenSshStep) -> Self {
        RespEventSftp::OnOpenStep(RespSftpOpenStep { client_transfer_id: String::new(), step })
    }
    fn on_session_closed() -> Option<Self> {
        Some(RespEventSftp::OnClose(RespSftpBase::new(String::new(), String::new())))
    }
}

impl RespEventSftp {
    pub fn to_type(&self) -> i16 {
        match self {
            RespEventSftp::OnOpenStep(_) => 24,
            RespEventSftp::OnOpen(_) => 40,
            RespEventSftp::OnClose(_) => 41,
            RespEventSftp::OnHomeDir(_) => 42,
            RespEventSftp::OnListDir(_) => 43,
            RespEventSftp::OnStat(_) => 44,
            RespEventSftp::OnRenamed(_) => 45,
            RespEventSftp::OnChmodded(_) => 46,
            RespEventSftp::OnCreated(_) => 47,
            RespEventSftp::OnDeleted(_) => 48,
            RespEventSftp::OnTransfer(_) => 49,
            RespEventSftp::OnDeleteProgress(_) => 50,
            RespEventSftp::OnError(_) => 51,
            RespEventSftp::OnStatProperties(_) => 52,
            RespEventSftp::OnPath(_) => 53,
            RespEventSftp::OnTransferConflict(_) => 54,
            RespEventSftp::OnTransferCount(_) => 55,
            RespEventSftp::OnDeleteCount(_) => 56,
            RespEventSftp::OnTransferProgress(_) => 57,
            RespEventSftp::OnFileOpened(_) => 58,
            RespEventSftp::OnExported(_) => 59,
        }
    }

    pub fn to_payload(&self) -> serde_json::Value {
        match self {
            RespEventSftp::OnOpen(p) => serde_json::to_value(p).unwrap(),
            RespEventSftp::OnClose(p) => serde_json::to_value(p).unwrap(),
            RespEventSftp::OnError(p) => serde_json::to_value(p).unwrap(),
            RespEventSftp::OnOpenStep(p) => serde_json::to_value(p).unwrap(),
            RespEventSftp::OnHomeDir(p) => serde_json::to_value(p).unwrap(),
            RespEventSftp::OnPath(p) => serde_json::to_value(p).unwrap(),
            RespEventSftp::OnListDir(p) => serde_json::to_value(p).unwrap(),
            RespEventSftp::OnStat(p) => serde_json::to_value(p).unwrap(),
            RespEventSftp::OnRenamed(p) => serde_json::to_value(p).unwrap(),
            RespEventSftp::OnChmodded(p) => serde_json::to_value(p).unwrap(),
            RespEventSftp::OnCreated(p) => serde_json::to_value(p).unwrap(),
            RespEventSftp::OnDeleted(p) => serde_json::to_value(p).unwrap(),
            RespEventSftp::OnTransfer(p) => serde_json::to_value(p).unwrap(),
            RespEventSftp::OnDeleteProgress(p) => serde_json::to_value(p).unwrap(),
            RespEventSftp::OnStatProperties(p) => serde_json::to_value(p).unwrap(),
            RespEventSftp::OnTransferConflict(p) => serde_json::to_value(p).unwrap(),
            RespEventSftp::OnTransferCount(p) => serde_json::to_value(p).unwrap(),
            RespEventSftp::OnDeleteCount(p) => serde_json::to_value(p).unwrap(),
            RespEventSftp::OnTransferProgress(p) => serde_json::to_value(p).unwrap(),
            RespEventSftp::OnFileOpened(p) => serde_json::to_value(p).unwrap(),
            RespEventSftp::OnExported(p) => serde_json::to_value(p).unwrap(),
        }
    }
}
