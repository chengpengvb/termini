use termini_macros::base_target;
use serde::{Deserialize, Serialize};

#[base_target(table = "snippet", entity_type = 16)]
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Model {
    pub name: String,
    pub command: String,
    pub sort: i16,

    #[serde(default)]
    pub remark: String,

    #[serde(default, deserialize_with = "crate::model::base_target::deserialize_null_default")]
    pub updated_at: i64,
}

pub type Snippet = Model;
