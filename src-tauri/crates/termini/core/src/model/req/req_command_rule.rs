use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReqCreateCommandRule {
    pub pattern: String,
    pub decision: i16,

    #[serde(default)]
    pub device_id: Option<String>,

    #[serde(default)]
    pub justification: Option<String>,
}
