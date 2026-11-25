use std::collections::VecDeque;
use tokio::sync::Mutex as AsyncMutex;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum SftpTransferAnswer {
    Overwrite,
    OverwriteAll,
    Skip,
    SkipAll,
}

impl SftpTransferAnswer {
    pub fn from_code(code: i16) -> Option<SftpTransferAnswer> {
        match code {
            0 => Some(SftpTransferAnswer::Overwrite),
            1 => Some(SftpTransferAnswer::OverwriteAll),
            2 => Some(SftpTransferAnswer::Skip),
            3 => Some(SftpTransferAnswer::SkipAll),
            _ => None,
        }
    }
}

pub struct SftpTransferAnswerEntry {
    pub request_id: String,
    pub answer: SftpTransferAnswer,
}

pub struct SftpTransferInbox {
    pub notify: tokio::sync::Notify,
    pub queue: AsyncMutex<VecDeque<SftpTransferAnswerEntry>>,
}

impl SftpTransferInbox {
    pub fn new() -> Self {
        Self { notify: tokio::sync::Notify::new(), queue: AsyncMutex::new(VecDeque::new()) }
    }
}
