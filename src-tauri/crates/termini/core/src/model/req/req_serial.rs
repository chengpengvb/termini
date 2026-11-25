use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct ReqSerialConfig {
    pub port_name: String,
}
