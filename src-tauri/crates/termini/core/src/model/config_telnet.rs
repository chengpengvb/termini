use sea_orm::JsonValue;
use termini_macros::Redact;
use std::collections::HashMap;
use termini_macros::base_target;
use serde::{Deserialize, Serialize};
use crate::model::config_term_base::ConfigTermBase;

pub static MORE_RESPONSE: &str = " ";
pub static PROMPT_PRI_PATTERN: &str = r"(?m)#\s*$";
pub static PROMPT_ALT_PATTERN: &str = r"(?m)[>\]]\s*$";
pub static PASSWORD_PATTERN: &str = r"(?i)assword";
pub static USERNAME_PATTERN: &str = r"(?i)(?:user:|username|login|user name)";
pub static MORE_PATTERN: &str = r"(?i)-{2,}\s*\(?\s*more[^\r\n-]*\)?\s*-{2,}";

pub const TELNET_DEFAULT_PORT: u16 = 23;
pub const MAX_SESSIONS_DEFAULT: i16 = 5;
pub const CMD_VERIFY_TIMEOUT_SECONDS_DEFAULT: i16 = 10;

fn default_max_sessions() -> i16 {
    MAX_SESSIONS_DEFAULT
}

#[base_target(table = "config_telnet", entity_type = 5, notify = 1)]
#[derive(Clone, Serialize, Deserialize, Redact)]
pub struct Model {
    pub nvt_eol: bool,
    #[encrypt]
    #[redact_base = "crate::model::config_term_base::ConfigTermBase"]
    pub base: JsonValue,
    #[serde(default)]
    pub device_id: String,
    #[serde(default = "default_max_sessions")]
    pub max_sessions: i16,
    pub cmd_verify: Option<bool>,                //脚本/AI 执行时是否做命令回显校验（同 Netmiko cmd_verify）；None 视为 true
    pub more_pattern: Option<String>,            //分页提示正则，命中后自动送 more_response 续翻，默认见 MORE_PATTERN
    pub more_response: Option<String>,           //命中分页提示后送出的续翻字符，默认空格
    pub environment: Option<JsonValue>,          // JSON: Option<HashMap<String, String>>
    pub username_pattern: Option<String>,        //用户名提示正则，默认见 USERNAME_PATTERN
    pub password_pattern: Option<String>,        //密码提示正则，默认见 PASSWORD_PATTERN
    pub terminal_types: Option<JsonValue>,       // JSON: Option<Vec<String>>
    pub prompt_pri_pattern: Option<String>,      //主提示符正则（特权/管理态，如 "Switch#"），默认见 PROMPT_PRI_PATTERN
    pub prompt_alt_pattern: Option<String>,      //备用提示符正则（普通/视图态，如 "<Switch>" "[Switch]"），默认见 PROMPT_ALT_PATTERN
    pub disable_paging_command: Option<String>,  //关闭分页的命令，留空则按设备 os_type 自动推断
    pub cmd_verify_timeout_seconds: Option<i16>, //命令回显校验的独立超时（秒）；None 视为默认 10s
}

impl Default for Model {
    fn default() -> Self {
        let base = ConfigTermBase { port: TELNET_DEFAULT_PORT, ..ConfigTermBase::default() };
        Self { base: serde_json::to_value(base).expect("ConfigBase to_value failed"), ..Self::blank() }
    }
}

impl Model {
    fn blank() -> Self {
        Self {
            nvt_eol: false,
            id: String::new(),
            more_pattern: None,
            environment: None,
            more_response: None,
            terminal_types: None,
            username_pattern: None,
            password_pattern: None,
            base: JsonValue::Null,
            device_id: String::new(),
            prompt_pri_pattern: None,
            prompt_alt_pattern: None,
            disable_paging_command: None,
            max_sessions: MAX_SESSIONS_DEFAULT,
            cmd_verify: None,
            cmd_verify_timeout_seconds: None,
        }
    }

    pub fn resolve_max_sessions(&self) -> usize {
        let size = if self.max_sessions <= 0 { MAX_SESSIONS_DEFAULT } else { self.max_sessions };
        size as usize
    }

    pub fn resolve_cmd_verify(&self) -> bool {
        self.cmd_verify.unwrap_or(true)
    }

    pub fn resolve_cmd_verify_timeout_seconds(&self) -> i16 {
        let v = self.cmd_verify_timeout_seconds.unwrap_or(CMD_VERIFY_TIMEOUT_SECONDS_DEFAULT);
        if v <= 0 {
            return CMD_VERIFY_TIMEOUT_SECONDS_DEFAULT;
        }
        v.clamp(1, 120)
    }

    pub fn get_base(&self) -> anyhow::Result<ConfigTermBase> {
        Ok(serde_json::from_value(self.base.clone())?)
    }

    pub fn set_base(&mut self, base: ConfigTermBase) -> anyhow::Result<()> {
        self.base = serde_json::to_value(base)?;
        Ok(())
    }

    pub fn get_terminal_types(&self) -> anyhow::Result<Option<Vec<String>>> {
        let Some(v) = &self.terminal_types else { return Ok(None) };
        Ok(Some(serde_json::from_value(v.clone())?))
    }

    pub fn set_terminal_types(&mut self, terminal_types: Option<Vec<String>>) -> anyhow::Result<()> {
        let Some(v) = terminal_types else {
            self.terminal_types = None;
            return Ok(());
        };
        self.terminal_types = Some(serde_json::to_value(v)?);
        Ok(())
    }

    pub fn get_environment(&self) -> anyhow::Result<Option<HashMap<String, String>>> {
        let Some(v) = &self.environment else { return Ok(None) };
        Ok(Some(serde_json::from_value(v.clone())?))
    }

    pub fn set_environment(&mut self, environment: Option<HashMap<String, String>>) -> anyhow::Result<()> {
        let Some(v) = environment else {
            self.environment = None;
            return Ok(());
        };
        self.environment = Some(serde_json::to_value(v)?);
        Ok(())
    }
}

pub type ConfigTelnet = Model;
