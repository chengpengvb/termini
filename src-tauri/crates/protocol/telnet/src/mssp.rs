use crate::event::{TelnetEvent, TelnetMsspValue};
use crate::error::{TelnetError, TelnetProtocolError};
use crate::command::{TELNET_MSSP_VAL, TELNET_MSSP_VAR};

pub fn parse_mssp(data: &[u8]) -> Result<TelnetEvent, TelnetProtocolError> {
    if data.is_empty() {
        return Err(TelnetProtocolError::new(TelnetError::Protocol, "MSSP subnegotiation is empty".to_string()));
    }
    if data[0] != TELNET_MSSP_VAR {
        return Err(TelnetProtocolError::new(TelnetError::Protocol, "MSSP subnegotiation has invalid data".to_string()));
    }
    Ok(TelnetEvent::Mssp { values: parse_values(data)? })
}

fn parse_values(data: &[u8]) -> Result<Vec<TelnetMsspValue>, TelnetProtocolError> {
    let mut values = Vec::new();
    let mut variable: Option<Vec<u8>> = None;
    let mut next_type = data[0];
    let mut index = 1;
    while index <= data.len() {
        let start = index;
        while index < data.len() && data[index] != TELNET_MSSP_VAR && data[index] != TELNET_MSSP_VAL {
            index += 1;
        }
        let segment = data[start..index].to_vec();
        if next_type == TELNET_MSSP_VAR {
            variable = Some(segment);
        } else if next_type == TELNET_MSSP_VAL {
            match variable.clone() {
                Some(name) => values.push(TelnetMsspValue { variable: name, value: segment }),
                None => {
                    return Err(TelnetProtocolError::new(TelnetError::Protocol, "invalid MSSP subnegotiation data".to_string()));
                }
            }
        } else {
            return Err(TelnetProtocolError::new(TelnetError::Protocol, "invalid MSSP subnegotiation data".to_string()));
        }
        if index == data.len() {
            break;
        }
        next_type = data[index];
        index += 1;
    }
    Ok(values)
}
