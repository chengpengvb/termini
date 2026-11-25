use termini_macros::base_target;
use serde::{Deserialize, Serialize};

#[base_target(table = "cloud_sync_state", entity_type = 32)]
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Model {
    pub version: String,
}

impl Model {
    pub const DEFAULT_ID: &'static str = "000000";
}

pub type CloudSyncState = Model;
