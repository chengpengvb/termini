use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RespFileEditRead {
    pub path: String,
    pub name: String,
    pub content: String,
}
