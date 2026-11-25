use crate::model::device::Device;
use serde::{Deserialize, Serialize};
use crate::model::dto::shared_ptr::Shared;

#[derive(Debug, Default, Copy, Clone, Serialize, Deserialize)]
pub struct WindowSize {
    pub dpi: u16,
    pub cols: u16,
    pub rows: u16,
    pub pixel_width: u16,
    pub pixel_height: u16,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ContextBase<T> {
    pub size: WindowSize,
    pub config: Shared<T>,
    pub device: Shared<Device>,
}
