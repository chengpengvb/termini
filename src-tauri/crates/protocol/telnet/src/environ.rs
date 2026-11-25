use crate::command::{
    TELNET_ENVIRON_ESC, TELNET_ENVIRON_INFO, TELNET_ENVIRON_IS, TELNET_ENVIRON_SEND, TELNET_ENVIRON_USERVAR, TELNET_ENVIRON_VALUE, TELNET_ENVIRON_VAR, TELNET_IAC, TELNET_SB, TELNET_SE,
    TELNET_TELOPT_NEW_ENVIRON,
};
use crate::subnegotiation::push_escaped_data;
use crate::error::{TelnetError, TelnetProtocolError};
use crate::event::{TelnetEnvironmentValue, TelnetEvent};

pub fn encode_new_environ(command: u8, values: &[TelnetEnvironmentValue]) -> Vec<u8> {
    encode_environ(TELNET_TELOPT_NEW_ENVIRON, command, values)
}

pub fn encode_environ(telopt: u8, command: u8, values: &[TelnetEnvironmentValue]) -> Vec<u8> {
    let payload = encode_environ_payload(command, values);
    let mut output = Vec::with_capacity(payload.len() + 5);
    output.extend_from_slice(&[TELNET_IAC, TELNET_SB, telopt]);
    push_escaped_data(&payload, &mut output);
    output.extend_from_slice(&[TELNET_IAC, TELNET_SE]);
    output
}

pub fn encode_environ_payload(command: u8, values: &[TelnetEnvironmentValue]) -> Vec<u8> {
    let mut output = vec![command];
    for value in values {
        output.push(value.kind);
        push_environ_value_bytes(&value.variable, &mut output);
        if !value.value.is_empty() {
            output.push(TELNET_ENVIRON_VALUE);
            push_environ_value_bytes(&value.value, &mut output);
        }
    }
    output
}

pub fn encode_environ_value(kind: u8, value: &[u8]) -> Vec<u8> {
    let mut output = vec![kind];
    push_environ_value_bytes(value, &mut output);
    output
}

pub fn begin_new_environ(command: u8) -> Vec<u8> {
    vec![TELNET_IAC, TELNET_SB, TELNET_TELOPT_NEW_ENVIRON, command]
}

pub fn append_new_environ_value(kind: u8, value: &[u8]) -> Vec<u8> {
    let mut output = Vec::with_capacity(value.len() + 2);
    output.push(kind);
    push_escaped_data(value, &mut output);
    output
}

pub fn parse_environ(telopt: u8, data: &[u8]) -> Result<TelnetEvent, TelnetProtocolError> {
    if data.is_empty() {
        return Err(TelnetProtocolError::new(TelnetError::Protocol, format!("telopt {} environ subnegotiation is empty", telopt)));
    }
    let command = data[0];
    if !is_valid_command(command) {
        return Err(TelnetProtocolError::new(TelnetError::Protocol, format!("telopt {} subneg has invalid command", telopt)));
    }
    if data.len() == 1 {
        return Ok(TelnetEvent::Environ { command, values: Vec::new() });
    }
    if !is_variable_type(data[1]) {
        return Err(TelnetProtocolError::new(TelnetError::Protocol, format!("telopt {} subneg missing variable type", telopt)));
    }
    if data[data.len() - 1] == TELNET_ENVIRON_ESC {
        return Err(TelnetProtocolError::new(TelnetError::Protocol, format!("telopt {} subneg ends with ESC", telopt)));
    }
    Ok(TelnetEvent::Environ { command, values: parse_values(&data[1..]) })
}

fn push_environ_value_bytes(input: &[u8], output: &mut Vec<u8>) {
    for byte in input {
        if *byte == TELNET_ENVIRON_VAR || *byte == TELNET_ENVIRON_VALUE || *byte == TELNET_ENVIRON_ESC || *byte == TELNET_ENVIRON_USERVAR {
            output.push(TELNET_ENVIRON_ESC);
        }
        output.push(*byte);
    }
}

fn parse_values(data: &[u8]) -> Vec<TelnetEnvironmentValue> {
    let mut values = Vec::new();
    let mut index = 0;
    while index < data.len() {
        if !is_variable_type(data[index]) {
            index += 1;
            continue;
        }
        let kind = data[index];
        index += 1;
        let variable = read_until_marker(data, &mut index, true);
        let mut value = Vec::new();
        if index < data.len() && data[index] == TELNET_ENVIRON_VALUE {
            index += 1;
            value = read_until_marker(data, &mut index, false);
        }
        values.push(TelnetEnvironmentValue { kind, variable, value });
    }
    values
}

fn read_until_marker(data: &[u8], index: &mut usize, stop_at_value: bool) -> Vec<u8> {
    let mut output = Vec::new();
    while *index < data.len() {
        let byte = data[*index];
        if is_variable_type(byte) || (stop_at_value && byte == TELNET_ENVIRON_VALUE) {
            break;
        }
        if byte == TELNET_ENVIRON_ESC {
            *index += 1;
        }
        if *index < data.len() {
            output.push(data[*index]);
            *index += 1;
        }
    }
    output
}

fn is_valid_command(command: u8) -> bool {
    command == TELNET_ENVIRON_SEND || command == TELNET_ENVIRON_IS || command == TELNET_ENVIRON_INFO
}

fn is_variable_type(byte: u8) -> bool {
    byte == TELNET_ENVIRON_VAR || byte == TELNET_ENVIRON_USERVAR
}
