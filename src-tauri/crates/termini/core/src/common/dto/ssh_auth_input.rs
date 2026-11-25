#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum SshAuthAnswer {
    UsernameUpdated,
    PasswordUpdated,
    PassphraseUpdated,
    KeyPairUpdated,
    MethodSelectUpdated,
    KeyboardInteractiveUpdated,
    AgentKeySelectUpdated,
    ReselectRequested,
    HostFingerprintReject,
    HostFingerprintTrustOnce,
    HostFingerprintTrustAlways,
}

impl SshAuthAnswer {
    pub fn from_code(code: i16) -> Option<SshAuthAnswer> {
        match code {
            0 => Some(SshAuthAnswer::UsernameUpdated),
            1 => Some(SshAuthAnswer::PasswordUpdated),
            2 => Some(SshAuthAnswer::PassphraseUpdated),
            3 => Some(SshAuthAnswer::KeyPairUpdated),
            4 => Some(SshAuthAnswer::MethodSelectUpdated),
            5 => Some(SshAuthAnswer::KeyboardInteractiveUpdated),
            6 => Some(SshAuthAnswer::AgentKeySelectUpdated),
            7 => Some(SshAuthAnswer::ReselectRequested),
            8 => Some(SshAuthAnswer::HostFingerprintReject),
            9 => Some(SshAuthAnswer::HostFingerprintTrustOnce),
            10 => Some(SshAuthAnswer::HostFingerprintTrustAlways),
            _ => None,
        }
    }

    pub fn to_code(self) -> i16 {
        match self {
            SshAuthAnswer::UsernameUpdated => 0,
            SshAuthAnswer::PasswordUpdated => 1,
            SshAuthAnswer::PassphraseUpdated => 2,
            SshAuthAnswer::KeyPairUpdated => 3,
            SshAuthAnswer::MethodSelectUpdated => 4,
            SshAuthAnswer::KeyboardInteractiveUpdated => 5,
            SshAuthAnswer::AgentKeySelectUpdated => 6,
            SshAuthAnswer::ReselectRequested => 7,
            SshAuthAnswer::HostFingerprintReject => 8,
            SshAuthAnswer::HostFingerprintTrustOnce => 9,
            SshAuthAnswer::HostFingerprintTrustAlways => 10,
        }
    }
}
