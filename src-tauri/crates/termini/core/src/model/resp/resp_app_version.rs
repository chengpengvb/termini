use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RespAboutInfo {
    pub version: String,
    pub platform: String,
    pub website_url: Option<String>,
    pub privacy_url: Option<String>,
    pub terms_url: Option<String>,
}
