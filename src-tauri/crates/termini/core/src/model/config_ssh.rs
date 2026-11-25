use sea_orm::JsonValue;
use termini_macros::Redact;
use termini_macros::base_target;
use serde::{Deserialize, Serialize};
use crate::model::config_term_base::ConfigTermBase;

pub const SSH_DEFAULT_PORT: u16 = 22;

#[base_target(table = "config_ssh", entity_type = 4, notify = 1)]
#[derive(Clone, Serialize, Deserialize, Redact)]
pub struct Model {
    #[encrypt]
    #[redact_base = "crate::model::config_term_base::ConfigTermBase"]
    pub base: JsonValue,
    pub use_gssapi: bool,
    #[serde(default)]
    pub device_id: String,
    pub use_ssh_agent: bool,
    pub share_session: bool,
    #[serde(default)]
    pub config_key_pair_id: String,
    #[encrypt]
    #[redact]
    pub passphrase: Option<String>,
    pub agent_key_id: Option<String>,
    pub jump_device_id: Option<String>,
    pub shell_idle_timeout_seconds: i64,
    pub preferred_auth_method: Option<i8>,
    #[encrypt]
    #[redact]
    pub keyboard_interactive_answers: Option<JsonValue>,
}

impl Default for Model {
    fn default() -> Self {
        let base = ConfigTermBase { port: SSH_DEFAULT_PORT, ..ConfigTermBase::default() };
        Self { base: serde_json::to_value(base).expect("ConfigBase to_value failed"), share_session: true, shell_idle_timeout_seconds: 300_i64, ..Self::blank() }
    }
}

impl Model {
    fn blank() -> Self {
        Self {
            passphrase: None,
            use_gssapi: false,
            id: String::new(),
            agent_key_id: None,
            share_session: true,
            use_ssh_agent: false,
            base: JsonValue::Null,
            jump_device_id: None,
            device_id: String::new(),
            preferred_auth_method: None,
            config_key_pair_id: String::new(),
            keyboard_interactive_answers: None,
            shell_idle_timeout_seconds: 0_i64,
        }
    }

    pub fn get_base(&self) -> anyhow::Result<ConfigTermBase> {
        Ok(serde_json::from_value(self.base.clone())?)
    }

    pub fn set_base(&mut self, base: ConfigTermBase) -> anyhow::Result<()> {
        self.base = serde_json::to_value(base)?;
        Ok(())
    }

    pub fn get_keyboard_interactive_answers(&self) -> anyhow::Result<Option<Vec<String>>> {
        let Some(v) = &self.keyboard_interactive_answers else {
            return Ok(None);
        };
        Ok(Some(serde_json::from_value(v.clone())?))
    }

    pub fn set_keyboard_interactive_answers(&mut self, answers: Option<Vec<String>>) -> anyhow::Result<()> {
        let Some(v) = answers else {
            self.keyboard_interactive_answers = None;
            return Ok(());
        };
        self.keyboard_interactive_answers = Some(serde_json::to_value(v)?);
        Ok(())
    }
}

pub type ConfigSsh = Model;
