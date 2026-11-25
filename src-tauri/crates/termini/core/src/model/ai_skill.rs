use termini_macros::base_target;
use serde::{Deserialize, Serialize};

pub const AI_SKILL_DESCRIPTION_MAX_LEN: usize = 255;

#[base_target(table = "ai_skill", entity_type = 12)]
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Model {
    pub sort: i16,
    #[serde(default, deserialize_with = "crate::model::base_target::deserialize_null_default")]
    pub name: String,
    #[serde(default, deserialize_with = "crate::model::base_target::deserialize_null_default")]
    pub content: String,
    #[sea_orm(string_len = 255)]
    #[serde(default, deserialize_with = "crate::model::base_target::deserialize_null_default")]
    pub description: String,
}

pub type AiSkill = Model;
