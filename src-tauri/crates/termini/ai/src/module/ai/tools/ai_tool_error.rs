use thiserror::Error;

#[derive(Debug, Error)]
pub enum AiToolError {
    #[error("tool cancelled: {0}")]
    Cancelled(String),

    #[error("tool denied: {0}")]
    Denied(String),

    #[error("{0}")]
    Execution(String),
}

impl AiToolError {
    pub fn detail(&self) -> &str {
        match self {
            AiToolError::Cancelled(message) => message.as_str(),
            AiToolError::Denied(message) => message.as_str(),
            AiToolError::Execution(message) => message.as_str(),
        }
    }
}
