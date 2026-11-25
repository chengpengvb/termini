use crate::event::TelnetEvent;
use crate::error::{TelnetError, TelnetProtocolError};
use crate::command::{TELNET_IAC, TELNET_SB, TELNET_SE, TELNET_TELOPT_NAWS, TELNET_TELOPT_TTYPE, TELNET_TTYPE_IS, TELNET_TTYPE_SEND};

pub fn parse_terminal_type(data: &[u8]) -> Result<TelnetEvent, TelnetProtocolError> {
    if data.is_empty() {
        return Err(TelnetProtocolError::new(TelnetError::Protocol, "TTYPE subnegotiation is empty".to_string()));
    }
    let command = data[0];
    if command == TELNET_TTYPE_SEND {
        return Ok(TelnetEvent::TerminalType { command, name: None });
    }
    if command == TELNET_TTYPE_IS {
        return Ok(TelnetEvent::TerminalType { command, name: Some(data[1..].to_vec()) });
    }
    Err(TelnetProtocolError::new(TelnetError::Protocol, format!("TTYPE subnegotiation has invalid command: {}", command)))
}

pub fn encode_terminal_type_send() -> Vec<u8> {
    vec![TELNET_IAC, TELNET_SB, TELNET_TELOPT_TTYPE, TELNET_TTYPE_SEND, TELNET_IAC, TELNET_SE]
}

pub fn encode_terminal_type_is(term: &[u8]) -> Vec<u8> {
    let mut output = Vec::with_capacity(term.len() + 6);
    output.extend_from_slice(&[TELNET_IAC, TELNET_SB, TELNET_TELOPT_TTYPE, TELNET_TTYPE_IS]);
    push_escaped_data(term, &mut output);
    output.extend_from_slice(&[TELNET_IAC, TELNET_SE]);
    output
}

pub fn encode_naws(columns: u16, rows: u16) -> Vec<u8> {
    let mut output = Vec::with_capacity(9);
    output.extend_from_slice(&[TELNET_IAC, TELNET_SB, TELNET_TELOPT_NAWS]);
    push_naws_value(columns, &mut output);
    push_naws_value(rows, &mut output);
    output.extend_from_slice(&[TELNET_IAC, TELNET_SE]);
    output
}

pub fn push_escaped_data(input: &[u8], output: &mut Vec<u8>) {
    for byte in input {
        output.push(*byte);
        if *byte == TELNET_IAC {
            output.push(TELNET_IAC);
        }
    }
}

fn push_naws_value(value: u16, output: &mut Vec<u8>) {
    let high = (value >> 8) as u8;
    let low = value as u8;
    output.push(high);
    if high == TELNET_IAC {
        output.push(TELNET_IAC);
    }
    output.push(low);
    if low == TELNET_IAC {
        output.push(TELNET_IAC);
    }
}
