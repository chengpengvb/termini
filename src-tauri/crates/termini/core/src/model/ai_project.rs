use termini_macros::base_target;
use serde::{Deserialize, Serialize};

#[base_target(table = "ai_project", entity_type = 19)]
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Model {
    pub name: String,
    pub sort: i16,
    pub timestamp: i64,
}

pub type AiProject = Model;
