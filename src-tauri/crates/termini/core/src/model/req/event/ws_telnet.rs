use serde::Deserialize;
use crate::model::req::event::ws_base::WsBase;
use crate::common::context::context_base::WindowSize;

pub type WriteEvent = WsBase<String>;
pub type OpenEvent = WsBase<TelnetOpen>; //ContextTelnet
pub type ResizeEvent = WsBase<WindowSize>;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TelnetOpen {
    pub size: WindowSize,
    pub device_id: String,
}
