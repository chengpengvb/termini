use thiserror::Error;

pub type RdpResult<T> = Result<T, RdpProtocolError>;

#[derive(Debug, Clone, Copy, Eq, Error, PartialEq)]
pub enum RdpError {
    #[error("buffer underrun")]
    Underrun,
    #[error("invalid sequence of protocol bytes")]
    Protocol,
    #[error("feature not supported")]
    Unsupported,
    #[error("encoding failure")]
    Encoding,
    #[error("authentication failure")]
    Authentication,
    #[error("connection rejected by server")]
    Rejected,
}

#[derive(Debug, Clone, Eq, Error, PartialEq)]
#[error("{kind}: {message}")]
pub struct RdpProtocolError {
    pub kind: RdpError,
    pub message: String,
}

impl RdpProtocolError {
    pub fn new(kind: RdpError, message: String) -> Self {
        Self { kind, message }
    }

    pub fn protocol(message: impl Into<String>) -> Self {
        Self::new(RdpError::Protocol, message.into())
    }

    pub fn underrun(message: impl Into<String>) -> Self {
        Self::new(RdpError::Underrun, message.into())
    }

    pub fn unsupported(message: impl Into<String>) -> Self {
        Self::new(RdpError::Unsupported, message.into())
    }

    pub fn encoding(message: impl Into<String>) -> Self {
        Self::new(RdpError::Encoding, message.into())
    }

    pub fn authentication(message: impl Into<String>) -> Self {
        Self::new(RdpError::Authentication, message.into())
    }

    pub fn rejected(message: impl Into<String>) -> Self {
        Self::new(RdpError::Rejected, message.into())
    }
}
