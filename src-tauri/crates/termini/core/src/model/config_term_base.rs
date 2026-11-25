use termini_macros::Redact;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Serialize, Deserialize, Redact)]
pub struct ConfigTermBase {
    #[serde(default)]
    pub port: u16,
    #[serde(default)]
    pub term: String,
    pub username: Option<String>,
    #[redact]
    pub password: Option<String>,
    pub keepalive_interval_seconds: u64,
    pub connect_timeout_seconds: Option<u64>,
}
