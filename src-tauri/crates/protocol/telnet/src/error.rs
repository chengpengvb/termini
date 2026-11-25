use thiserror::Error;

#[derive(Debug, Clone, Copy, Eq, Error, PartialEq)]
pub enum TelnetError {
    #[error("no error")]
    Ok,
    #[error("invalid parameter or API misuse")]
    BadValue,
    #[error("memory allocation failure")]
    NoMemory,
    #[error("data exceeds buffer size")]
    Overflow,
    #[error("invalid sequence of special bytes")]
    Protocol,
    #[error("compression error")]
    Compress,
}

#[derive(Debug, Clone, Eq, Error, PartialEq)]
#[error("{kind}: {message}")]
pub struct TelnetProtocolError {
    pub kind: TelnetError,
    pub message: String,
}

impl TelnetProtocolError {
    pub fn new(kind: TelnetError, message: String) -> Self {
        Self { kind, message }
    }
}
