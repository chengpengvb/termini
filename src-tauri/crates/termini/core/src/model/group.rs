use termini_macros::base_target;
use serde::{Deserialize, Serialize};

pub const DEFAULT_GROUP_ID: &str = "000000";

#[base_target(table = "group", entity_type = 2)]
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Model {
    pub name: String, // 组名
}

pub type Group = Model;
