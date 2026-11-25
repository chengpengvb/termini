use crate::command::{TELNET_DO, TELNET_DONT, TELNET_TELOPT_BINARY, TELNET_TELOPT_ECHO, TELNET_TELOPT_EOR, TELNET_TELOPT_NAWS, TELNET_TELOPT_SGA, TELNET_TELOPT_TTYPE, TELNET_WILL, TELNET_WONT};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct TelnetOptionSupport {
    pub telopt: u8,
    pub us: u8,
    pub him: u8,
}

impl TelnetOptionSupport {
    pub const fn new(telopt: u8, us: u8, him: u8) -> Self {
        Self { telopt, us, him }
    }

    pub fn supports_us(&self) -> bool {
        self.us == TELNET_WILL
    }

    pub fn supports_him(&self) -> bool {
        self.him == TELNET_DO
    }
}

pub const DEFAULT_TELNET_OPTIONS: &[TelnetOptionSupport] = &[
    TelnetOptionSupport::new(TELNET_TELOPT_BINARY, TELNET_WILL, TELNET_DO),
    TelnetOptionSupport::new(TELNET_TELOPT_ECHO, TELNET_WONT, TELNET_DO),
    TelnetOptionSupport::new(TELNET_TELOPT_SGA, TELNET_WONT, TELNET_DO),
    TelnetOptionSupport::new(TELNET_TELOPT_TTYPE, TELNET_WILL, TELNET_DONT),
    TelnetOptionSupport::new(TELNET_TELOPT_NAWS, TELNET_WILL, TELNET_DONT),
    TelnetOptionSupport::new(TELNET_TELOPT_EOR, TELNET_WONT, TELNET_DO),
];

pub fn supports_us(telopts: &[TelnetOptionSupport], telopt: u8) -> bool {
    match telopts.iter().find(|item| item.telopt == telopt) {
        Some(item) => item.supports_us(),
        None => false,
    }
}

pub fn supports_him(telopts: &[TelnetOptionSupport], telopt: u8) -> bool {
    match telopts.iter().find(|item| item.telopt == telopt) {
        Some(item) => item.supports_him(),
        None => false,
    }
}
