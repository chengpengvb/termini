use crate::error::TelnetProtocolError;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum TelnetEvent {
    Data(Vec<u8>),
    Send(Vec<u8>),
    Eor,
    Iac(u8),
    Will(u8),
    Wont(u8),
    Do(u8),
    Dont(u8),
    Subnegotiation { telopt: u8, data: Vec<u8> },
    Compress { enabled: bool },
    Zmp { argv: Vec<Vec<u8>> },
    TerminalType { command: u8, name: Option<Vec<u8>> },
    Environ { command: u8, values: Vec<TelnetEnvironmentValue> },
    Mssp { values: Vec<TelnetMsspValue> },
    Warning(TelnetProtocolError),
    Error(TelnetProtocolError),
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TelnetEnvironmentValue {
    pub kind: u8,
    pub variable: Vec<u8>,
    pub value: Vec<u8>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TelnetMsspValue {
    pub variable: Vec<u8>,
    pub value: Vec<u8>,
}
