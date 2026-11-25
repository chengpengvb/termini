use serde::Deserialize;
use crate::model::req::event::ws_base::WsBase;
use crate::common::context::context_base::WindowSize;

pub type WriteEvent = WsBase<String>;
pub type OpenEvent = WsBase<ShellOpen>;
pub type ResizeEvent = WsBase<WindowSize>;
pub type SnapshotEvent = WsBase<ShellSnapshotReport>;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShellOpen {
    pub shell_id: String,
    pub size: WindowSize,
    pub cwd: Option<String>,

    #[serde(default)]
    pub attach_process_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShellSnapshotReport {
    pub text: String,
    pub cursor_row: u16,
    pub cursor_col: u16,
}
