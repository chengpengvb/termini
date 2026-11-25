use termini_macros::Redact;
use termini_macros::base_target;
use serde::{Deserialize, Serialize};

pub const SNMP_PORT_DEFAULT: i32 = 161;
pub const SNMP_RETRIES_DEFAULT: i16 = 1;
pub const SNMP_VERSION_DEFAULT: i16 = 2;
pub const SNMP_TIMEOUT_SECONDS_DEFAULT: i16 = 3;
pub const SNMP_COMMUNITY_DEFAULT: &str = "public";

#[base_target(table = "config_snmp", entity_type = 15, notify = 1)]
#[derive(Clone, Serialize, Deserialize, Redact)]
pub struct Model {
    #[serde(default)]
    pub device_id: String,
    pub enabled: bool,
    pub version: i16,
    #[encrypt]
    #[redact]
    pub community: String,
    pub port: i32,
    pub retries: i16,
    pub timeout_seconds: i16,
}

impl Default for Model {
    fn default() -> Self {
        Self {
            id: String::new(),
            device_id: String::new(),
            enabled: false,
            version: SNMP_VERSION_DEFAULT,
            community: SNMP_COMMUNITY_DEFAULT.to_string(),
            port: SNMP_PORT_DEFAULT,
            timeout_seconds: SNMP_TIMEOUT_SECONDS_DEFAULT,
            retries: SNMP_RETRIES_DEFAULT,
        }
    }
}

pub type ConfigSnmp = Model;
