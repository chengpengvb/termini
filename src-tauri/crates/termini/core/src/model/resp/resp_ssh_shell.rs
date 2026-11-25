use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RespSshShellExec {
    pub exit_code: i32,
    pub output: String,
}
