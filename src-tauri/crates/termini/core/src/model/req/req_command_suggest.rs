use serde::Deserialize;

pub const SUGGEST_REQ_COUNT_MIN: i16 = 1;
pub const SUGGEST_REQ_COUNT_MAX: i16 = 10;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReqCommandSuggest {
    #[serde(default)]
    pub alt_screen: bool,

    pub device_id: String,

    pub suggest_count: i16,

    pub terminal_kind: String,

    #[serde(default)]
    pub current_line: String,

    #[serde(default)]
    pub recent_output: String,

    #[serde(default)]
    pub history: Vec<String>,

    #[serde(default)]
    pub instruction: Option<String>,

    #[serde(default)]
    pub shell_kind: Option<String>,
}
