use termini_macros::Redact;
use termini_macros::base_target;
use serde::{Deserialize, Serialize};

#[base_target(table = "config_key_pair", entity_type = 7)]
#[derive(Clone, Default, Serialize, Deserialize, Redact)]
pub struct Model {
    pub name: String,
    pub public_key: Option<String>,
    #[encrypt]
    #[redact]
    pub private_key: Option<String>,
    #[encrypt]
    #[redact]
    pub passphrase: Option<String>,
    #[encrypt]
    #[redact]
    pub certificate: Option<String>,
}

pub type ConfigKeyPair = Model;
