#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum SshChannelKind {
    Pty,
    Shell,
    Sftp,
}
