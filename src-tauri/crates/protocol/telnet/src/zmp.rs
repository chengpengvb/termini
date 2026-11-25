use crate::event::TelnetEvent;
use crate::subnegotiation::push_escaped_data;
use crate::error::{TelnetError, TelnetProtocolError};
use crate::command::{TELNET_IAC, TELNET_SB, TELNET_SE, TELNET_TELOPT_ZMP};

pub fn parse_zmp(data: &[u8]) -> Result<TelnetEvent, TelnetProtocolError> {
    if data.is_empty() || data[data.len() - 1] != 0 {
        return Err(TelnetProtocolError::new(TelnetError::Protocol, "incomplete ZMP frame".to_string()));
    }
    let mut argv = Vec::new();
    let mut start = 0;
    for index in 0..data.len() {
        if data[index] == 0 {
            argv.push(data[start..index].to_vec());
            start = index + 1;
        }
    }
    Ok(TelnetEvent::Zmp { argv })
}

pub fn encode_zmp(argv: &[Vec<u8>]) -> Vec<u8> {
    let mut output = begin_zmp(&argv[0]);
    for argument in argv.iter().skip(1) {
        append_zmp_argument(&mut output, argument);
    }
    finish_zmp(&mut output);
    output
}

pub fn encode_zmp_payload(argv: &[Vec<u8>]) -> Vec<u8> {
    let mut output = Vec::new();
    for argument in argv {
        output.extend_from_slice(argument);
        output.push(0);
    }
    output
}

pub fn begin_zmp(command: &[u8]) -> Vec<u8> {
    let mut output = vec![TELNET_IAC, TELNET_SB, TELNET_TELOPT_ZMP];
    append_zmp_argument(&mut output, command);
    output
}

pub fn append_zmp_argument(output: &mut Vec<u8>, argument: &[u8]) {
    push_escaped_data(argument, output);
    output.push(0);
}

pub fn finish_zmp(output: &mut Vec<u8>) {
    output.extend_from_slice(&[TELNET_IAC, TELNET_SE]);
}
