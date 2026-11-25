use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReqShellDaemonStop {
    pub chat_id: String,
    pub process_id: String,
}
