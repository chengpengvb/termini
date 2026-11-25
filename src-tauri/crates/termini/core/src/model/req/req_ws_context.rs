use anyhow::{anyhow, Result};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ReqSocketContext {
    pub r#type: i16,
    pub session_id: String,
    pub payload: Option<serde_json::Value>,
}

impl ReqSocketContext {
    pub fn as_payload<T: DeserializeOwned>(&self) -> Result<T> {
        let value = self.payload.clone().ok_or_else(|| anyhow!("payload is empty"))?;
        serde_json::from_value(value).map_err(|error| anyhow!("payload invalid: {}", error))
    }
}
