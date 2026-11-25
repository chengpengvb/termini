use termini_macros::base_target;
use serde::{Deserialize, Serialize};

#[base_target(table = "port_forward", entity_type = 8)]
#[derive(Clone, Serialize, Deserialize)]
pub struct Model {
    #[serde(default)]
    pub device_id: String,
    #[serde(default)]
    pub name: String,
    pub kind: i8,
    #[serde(default)]
    pub bind_address: String,
    pub bind_port: i32,
    #[serde(default)]
    pub target_host: String,
    #[serde(default)]
    pub target_port: i32,
    #[serde(default)]
    pub enabled: bool,
    #[sea_orm(ignore)]
    #[serde(default)]
    pub status: i16, // 运行态：查询填充 / WS 推送，不落库
}

pub const FORWARD_STATUS_IDLE: i16 = 0;
pub const FORWARD_STATUS_RUNNING: i16 = 1;
pub const FORWARD_STATUS_ERROR: i16 = 2;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ForwardKind {
    Local,
    Remote,
    Dynamic,
}

impl ForwardKind {
    pub fn from_i8(value: i8) -> Option<ForwardKind> {
        match value {
            0 => Some(ForwardKind::Local),
            1 => Some(ForwardKind::Remote),
            2 => Some(ForwardKind::Dynamic),
            _ => None,
        }
    }
}

impl Model {
    pub fn forward_kind(&self) -> Option<ForwardKind> {
        ForwardKind::from_i8(self.kind)
    }
}

pub type PortForward = Model;
