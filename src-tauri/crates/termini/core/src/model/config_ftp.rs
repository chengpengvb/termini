use sea_orm::JsonValue;
use termini_macros::Redact;
use termini_macros::base_target;
use serde::{Deserialize, Serialize};
use crate::model::config_term_base::ConfigTermBase;

pub const PROTOCOL_FTP: i16 = 7;
pub const PROTOCOL_SFTP: i16 = 3;
pub const FTP_DEFAULT_PORT: u16 = 21;
pub const FTP_SECURITY_NONE: i16 = 0;
pub const FTP_SECURITY_EXPLICIT: i16 = 1;
pub const FTP_SECURITY_IMPLICIT: i16 = 2;

#[base_target(table = "config_ftp", entity_type = 37, notify = 1)]
#[derive(Clone, Serialize, Deserialize, Redact)]
pub struct Model {
    #[encrypt]
    #[redact_base = "crate::model::config_term_base::ConfigTermBase"]
    pub base: JsonValue,
    #[serde(default)]
    pub device_id: String,
    #[serde(default)]
    pub security: i16,
    #[serde(default)]
    pub ignore_cert: bool,
}

impl Default for Model {
    fn default() -> Self {
        let base = ConfigTermBase { port: FTP_DEFAULT_PORT, ..ConfigTermBase::default() };
        Self { base: serde_json::to_value(base).expect("ConfigBase to_value failed"), ..Self::blank() }
    }
}

impl Model {
    fn blank() -> Self {
        Self { id: String::new(), base: JsonValue::Null, device_id: String::new(), security: FTP_SECURITY_NONE, ignore_cert: false }
    }

    pub fn get_base(&self) -> anyhow::Result<ConfigTermBase> {
        Ok(serde_json::from_value(self.base.clone())?)
    }

    pub fn set_base(&mut self, base: ConfigTermBase) -> anyhow::Result<()> {
        self.base = serde_json::to_value(base)?;
        Ok(())
    }
}

pub type ConfigFtp = Model;
