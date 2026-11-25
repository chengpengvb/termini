use anyhow::Result;
use std::sync::Arc;
use crate::util::time_util::now_millis;
use crate::model::resp::resp_shell::ShellSnapshot;
use crate::model::resp::resp_shell::ShellInputSource;
use crate::module::common::client_terminal::ClientTerminal;
use crate::module::shell::client_shell_pty::ClientShellPty;
use crate::model::req::event::ws_shell::ShellSnapshotReport;
use crate::module::common::context::context_base::WindowSize;
use crate::module::shell::shell_session_hub::ShellSessionRecord;

pub enum ShellSessionClient {
    Owned(Arc<ClientShellPty>),
    Attached(Arc<ShellSessionRecord>),
}

impl ShellSessionClient {
    pub fn owned(pty: Arc<ClientShellPty>) -> Self {
        Self::Owned(pty)
    }

    pub fn attached(record: Arc<ShellSessionRecord>) -> Self {
        Self::Attached(record)
    }

    pub async fn close(&self) -> Result<()> {
        match self {
            Self::Owned(pty) => pty.close().await,
            Self::Attached(record) => {
                record.detach();
                Ok(())
            }
        }
    }

    pub async fn write(&self, data: String) -> Result<()> {
        if let Self::Attached(record) = self {
            record.record_input(ShellInputSource::User, &data);
        }
        self.pty().write(data).await
    }

    pub async fn set_window_size(&self, size: WindowSize) -> Result<()> {
        self.pty().set_window_size(size).await
    }

    pub fn report_snapshot(&self, report: ShellSnapshotReport) {
        if let Self::Attached(record) = self {
            record.put_snapshot(ShellSnapshot { at_ms: now_millis(), text: report.text, cursor_row: report.cursor_row, cursor_col: report.cursor_col });
        }
    }

    fn pty(&self) -> &Arc<ClientShellPty> {
        match self {
            Self::Owned(pty) => pty,
            Self::Attached(record) => &record.pty,
        }
    }
}
