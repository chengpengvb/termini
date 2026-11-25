use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ReqFileEditRead {
    pub path: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReqFileEditWrite {
    pub path: String,
    pub content: String,
    #[serde(default)]
    pub create: bool,
}
