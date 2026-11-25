use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct RespPlatformInfo {
    pub os: String,
    pub mobile: bool,
    pub has_store: bool,
    pub has_serial: bool,
    pub has_web_login: bool,
    pub has_local_shell: bool,
}
