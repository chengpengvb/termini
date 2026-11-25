use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RespDiagnosticsExport {
    pub path: String,
    pub files: usize,
    pub bytes: u64,
}
