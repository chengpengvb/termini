use termini_macros::base_target;
use serde::{Deserialize, Serialize};

#[base_target(table = "ai_chat_2_project", entity_type = 33)]
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Model {
    pub chat_id: String,
    pub project_id: String,
}

pub type AiChat2Project = Model;
