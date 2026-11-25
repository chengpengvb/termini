use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct RespUserCloudSync {
    pub r#type: i8,
    pub payload: String,
}
