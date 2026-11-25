use russh::MethodKind;

pub mod auth_method_code {
    pub const NONE: i8 = 0;
    pub const PASSWORD: i8 = 1;
    pub const PUBLIC_KEY: i8 = 2;
    pub const KEYBOARD_INTERACTIVE: i8 = 3;
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum SshAuthMethod {
    None,
    Password,
    PublicKey,
    KeyboardInteractive,
}

impl SshAuthMethod {
    pub fn to_code(self) -> i8 {
        match self {
            Self::None => auth_method_code::NONE,
            Self::Password => auth_method_code::PASSWORD,
            Self::PublicKey => auth_method_code::PUBLIC_KEY,
            Self::KeyboardInteractive => auth_method_code::KEYBOARD_INTERACTIVE,
        }
    }

    pub fn from_code(code: i8) -> Option<Self> {
        match code {
            auth_method_code::PASSWORD => Some(Self::Password),
            auth_method_code::PUBLIC_KEY => Some(Self::PublicKey),
            auth_method_code::KEYBOARD_INTERACTIVE => Some(Self::KeyboardInteractive),
            _ => None,
        }
    }

    pub fn to_method_kind(self) -> Option<MethodKind> {
        match self {
            Self::Password => Some(MethodKind::Password),
            Self::PublicKey => Some(MethodKind::PublicKey),
            Self::KeyboardInteractive => Some(MethodKind::KeyboardInteractive),
            Self::None => None,
        }
    }

    pub fn from_method_kind(kind: &MethodKind) -> Option<Self> {
        match kind {
            MethodKind::Password => Some(Self::Password),
            MethodKind::PublicKey => Some(Self::PublicKey),
            MethodKind::KeyboardInteractive => Some(Self::KeyboardInteractive),
            _ => None,
        }
    }
}

pub enum SshAuthAttempt {
    Reselect,
    Rejected(String),
    Success(SshAuthMethod),
    PartialSuccess { remaining: Vec<MethodKind> },
}
