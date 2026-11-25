use termini_macros::base_target;
use serde::{Deserialize, Serialize};

#[base_target(table = "account_token", entity_type = 26)]
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Model {
    #[encrypt]
    pub token: String,
}

impl Model {
    pub const DEFAULT_ID: &'static str = "000000";
}

pub type AccountToken = Model;
