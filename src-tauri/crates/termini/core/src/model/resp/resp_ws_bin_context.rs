use bytes::Bytes;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct RespSocketBinContext {
    pub r#type: i16,
    pub session_id: String,
    pub payload: Bytes,
}
