use serde::Deserialize;
use crate::model::req::event::ws_base::WsBase;
use crate::common::context::context_base::WindowSize;

pub type FrameSyncEvent = WsBase<u64>;
pub type OpenEvent = WsBase<RdpOpen>; //ContextRdp
pub type VisibilityEvent = WsBase<bool>;
pub type ResizeEvent = WsBase<WindowSize>;
pub type InputEvent = WsBase<Vec<RdpInput>>;
pub type FocusEvent = WsBase<Option<RdpFocus>>;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RdpOpen {
    pub size: WindowSize,
    pub device_id: String,
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct RdpFocus {
    pub left: u16,
    pub top: u16,
    pub right: u16,
    pub bottom: u16,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum RdpInput {
    #[serde(rename_all = "camelCase")]
    Scancode { code: u16, pressed: bool, extended: bool },
    #[serde(rename_all = "camelCase")]
    Unicode { code: u16, pressed: bool },
    #[serde(rename_all = "camelCase")]
    Mouse { x: u16, y: u16, button: u8, pressed: bool },
    #[serde(rename_all = "camelCase")]
    Move { x: u16, y: u16 },
    #[serde(rename_all = "camelCase")]
    MoveRelative { x: i16, y: i16 },
    #[serde(rename_all = "camelCase")]
    Wheel { x: u16, y: u16, delta: i16, horizontal: bool },
    #[serde(rename_all = "camelCase")]
    Sync { scroll_lock: bool, num_lock: bool, caps_lock: bool, kana_lock: bool },
}
