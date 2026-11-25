use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct RespLoginUrl {
    pub url: String,
}
