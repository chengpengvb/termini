use serde::Deserialize;
use crate::model::req::event::ws_base::WsBase;
use crate::common::context::context_base::WindowSize;

pub type WriteEvent = WsBase<String>;
pub type OpenEvent = WsBase<SerialOpen>;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SerialOpen {
    pub port_name: String,
    pub size: WindowSize,
}
