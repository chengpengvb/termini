use thiserror::Error;

#[derive(Debug, Error)]
pub enum SerialError {
    #[error("invalid serial port configuration: {0}")]
    InvalidConfig(String),

    #[error("serial port unavailable: {0}")]
    Unavailable(String),

    #[error("serial ports are not supported on this platform")]
    Unsupported,

    #[error("serial port I/O error: {0}")]
    Io(#[from] std::io::Error),
}
