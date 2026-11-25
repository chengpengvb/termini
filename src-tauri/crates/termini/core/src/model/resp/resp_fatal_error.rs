use serde::{Deserialize, Serialize};

pub const FATAL_KIND_PANIC: i16 = 1;
pub const FATAL_KIND_CRYPTO: i16 = 2;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RespFatalError {
    pub kind: i16,
    pub message: String,
}
