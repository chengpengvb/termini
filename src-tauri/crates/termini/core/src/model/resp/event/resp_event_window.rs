use crate::model::resp::resp_ws_context::RespSocketContext;

pub const TRAY_ACTION_HOME: i16 = 0;
pub const TRAY_ACTION_NEW_CHAT: i16 = 1;

pub enum RespEventWindow {
    OnMaximizedChanged(bool),
    OnCloseAsk,
    OnTrayAction(i16),
    OnOpenHere { path: String, seq: u64 },
}

impl RespEventWindow {
    pub const ON_CLOSE_ASK: i16 = 94;
    pub const ON_OPEN_HERE: i16 = 96;
    pub const ON_TRAY_ACTION: i16 = 95;
    pub const ON_MAXIMIZED_CHANGED: i16 = 93;

    pub fn to_type(&self) -> i16 {
        match self {
            RespEventWindow::OnCloseAsk => Self::ON_CLOSE_ASK,
            RespEventWindow::OnTrayAction(_) => Self::ON_TRAY_ACTION,
            RespEventWindow::OnOpenHere { .. } => Self::ON_OPEN_HERE,
            RespEventWindow::OnMaximizedChanged(_) => Self::ON_MAXIMIZED_CHANGED,
        }
    }

    pub fn to_resp(&self) -> RespSocketContext {
        let payload = match self {
            RespEventWindow::OnMaximizedChanged(maximized) => serde_json::to_value(maximized).ok(),
            RespEventWindow::OnCloseAsk => None,
            RespEventWindow::OnTrayAction(action) => serde_json::to_value(action).ok(),
            RespEventWindow::OnOpenHere { path, seq } => Some(serde_json::json!({ "path": path, "seq": seq })),
        };
        RespSocketContext { r#type: self.to_type(), session_id: "main".to_string(), payload }
    }
}
