use serde::Deserialize;
use crate::model::req::event::ws_base::WsBase;
use crate::common::context::context_base::WindowSize;

pub type WriteEvent = WsBase<String>;
pub type OpenEvent = WsBase<SshOpen>; //ContextSsh
pub type ResizeEvent = WsBase<WindowSize>;
pub type AuthAnswerEvent = WsBase<SshAuthAnswerData>;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SshOpen {
    pub size: WindowSize,
    pub device_id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SshAuthAnswerData {
    pub code: i16,
    pub request_id: String,
}
