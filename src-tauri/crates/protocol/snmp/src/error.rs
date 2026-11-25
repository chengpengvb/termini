use thiserror::Error;

#[derive(Debug, Error)]
pub enum SnmpError {
    #[error("BER encoding overflow: {0}")]
    Encode(String),

    #[error("BER decode failed: {0}")]
    Decode(String),

    #[error("invalid OID format: {0}")]
    InvalidOid(String),

    #[error("request timed out")]
    Timeout,

    #[error("SNMP error: {name}(status={status}, index={index})")]
    ErrorStatus { status: i32, index: i32, name: &'static str },

    #[error("operation not supported by protocol version: {0}")]
    UnsupportedOperation(String),

    #[error("network error: {0}")]
    Io(#[from] std::io::Error),
}

pub fn error_status_name(status: i32) -> &'static str {
    match status {
        0 => "noError",
        1 => "tooBig",
        2 => "noSuchName",
        3 => "badValue",
        4 => "readOnly",
        5 => "genErr",
        6 => "noAccess",
        7 => "wrongType",
        8 => "wrongLength",
        9 => "wrongEncoding",
        10 => "wrongValue",
        11 => "noCreation",
        12 => "inconsistentValue",
        13 => "resourceUnavailable",
        14 => "commitFailed",
        15 => "undoFailed",
        16 => "authorizationError",
        17 => "notWritable",
        18 => "inconsistentName",
        _ => "unknown",
    }
}

pub const ERROR_STATUS_NO_SUCH_NAME: i32 = 2;
