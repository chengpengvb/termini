use crate::model::config_ssh::ConfigSsh;

#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub struct SshSessionKey {
    pub port: u16,
    pub host: String,
}

impl SshSessionKey {
    pub fn from_config(host: &str, config: &ConfigSsh) -> Option<Self> {
        let base = config.get_base().ok()?;
        Some(Self { host: host.to_string(), port: base.port })
    }
}
