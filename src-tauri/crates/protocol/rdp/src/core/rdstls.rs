use crate::core::logon::AutoReconnectCookie;
use crate::core::stream::{ReadCursor, WriteCursor};
use crate::core::logon::AUTO_RECONNECT_COOKIE_LENGTH;
use crate::core::error::{RdpProtocolError, RdpResult};

pub const CAPABILITIES_LENGTH: usize = 8;
pub const AUTHENTICATION_RESPONSE_LENGTH: usize = 10;

const RDSTLS_VERSION_1: u16 = 0x0001;

const RDSTLS_TYPE_CAPABILITIES: u16 = 0x0001;
const RDSTLS_TYPE_AUTHREQ: u16 = 0x0002;
const RDSTLS_TYPE_AUTHRSP: u16 = 0x0004;

const RDSTLS_DATA_CAPABILITIES: u16 = 0x0001;
const RDSTLS_DATA_PASSWORD_CREDS: u16 = 0x0001;
const RDSTLS_DATA_AUTORECONNECT_COOKIE: u16 = 0x0002;
const RDSTLS_DATA_RESULT_CODE: u16 = 0x0001;

const RESULT_SUCCESS: u32 = 0x0000_0000;
const RESULT_ACCESS_DENIED: u32 = 0x0000_0005;
const RESULT_LOGON_FAILURE: u32 = 0x0000_052E;
const RESULT_LOGON_TYPE_NOT_GRANTED: u32 = 0x0000_0569;

pub struct RdstlsSettings {
    pub username: String,
    pub domain: String,
    pub session_id: u32,
    pub password: Option<Vec<u8>>,
    pub redirection_guid: Option<Vec<u8>>,
    pub auto_reconnect: Option<AutoReconnectCookie>,
}

pub fn parse_capabilities(payload: &[u8]) -> RdpResult<u16> {
    let mut cursor = ReadCursor::new(payload);
    let version = cursor.read_u16_le()?;
    let pdu_type = cursor.read_u16_le()?;
    if pdu_type != RDSTLS_TYPE_CAPABILITIES {
        return Err(RdpProtocolError::protocol(format!("unexpected rdstls pdu type: 0x{:04x}", pdu_type)));
    }
    let data_type = cursor.read_u16_le()?;
    if data_type != RDSTLS_DATA_CAPABILITIES {
        return Err(RdpProtocolError::protocol(format!("unexpected rdstls capabilities data type: 0x{:04x}", data_type)));
    }
    let supported = cursor.read_u16_le()?;
    if supported & RDSTLS_VERSION_1 == 0 {
        return Err(RdpProtocolError::unsupported(format!("unsupported rdstls version: 0x{:04x}", supported)));
    }
    Ok(version)
}

pub fn build_authentication_request(version: u16, settings: &RdstlsSettings) -> RdpResult<Vec<u8>> {
    let mut cursor = WriteCursor::with_capacity(256);
    cursor.write_u16_le(version);
    cursor.write_u16_le(RDSTLS_TYPE_AUTHREQ);
    match (&settings.password, settings.auto_reconnect) {
        (Some(password), _) => {
            cursor.write_u16_le(RDSTLS_DATA_PASSWORD_CREDS);
            write_data(&mut cursor, settings.redirection_guid.as_deref().unwrap_or_default())?;
            write_string(&mut cursor, &settings.username)?;
            write_string(&mut cursor, &settings.domain)?;
            write_data(&mut cursor, password)?;
        }
        (None, Some(cookie)) => {
            cursor.write_u16_le(RDSTLS_DATA_AUTORECONNECT_COOKIE);
            cursor.write_u32_le(settings.session_id);
            cursor.write_u16_le(AUTO_RECONNECT_COOKIE_LENGTH as u16);
            cursor.write_u32_le(AUTO_RECONNECT_COOKIE_LENGTH);
            cursor.write_u32_le(cookie.version);
            cursor.write_u32_le(cookie.logon_id);
            cursor.write_slice(&cookie.random_bits);
        }
        (None, None) => return Err(RdpProtocolError::authentication("rdstls has neither a redirection password nor an auto-reconnect cookie".to_string())),
    }
    Ok(cursor.into_vec())
}

pub fn parse_authentication_response(payload: &[u8]) -> RdpResult<()> {
    let mut cursor = ReadCursor::new(payload);
    cursor.read_u16_le()?;
    let pdu_type = cursor.read_u16_le()?;
    if pdu_type != RDSTLS_TYPE_AUTHRSP {
        return Err(RdpProtocolError::protocol(format!("unexpected rdstls pdu type: 0x{:04x}", pdu_type)));
    }
    let data_type = cursor.read_u16_le()?;
    if data_type != RDSTLS_DATA_RESULT_CODE {
        return Err(RdpProtocolError::protocol(format!("unexpected rdstls result data type: 0x{:04x}", data_type)));
    }
    let result = cursor.read_u32_le()?;
    if result != RESULT_SUCCESS {
        return Err(RdpProtocolError::rejected(describe_result(result)));
    }
    Ok(())
}

fn write_data(cursor: &mut WriteCursor, data: &[u8]) -> RdpResult<()> {
    cursor.write_u16_le(u16::try_from(data.len()).map_err(|_| RdpProtocolError::encoding(format!("rdstls data too long: {}", data.len())))?);
    cursor.write_slice(data);
    Ok(())
}

fn write_string(cursor: &mut WriteCursor, value: &str) -> RdpResult<()> {
    let length = (value.encode_utf16().count() + 1) * 2;
    cursor.write_u16_le(u16::try_from(length).map_err(|_| RdpProtocolError::encoding(format!("rdstls string too long: {}", length)))?);
    cursor.write_utf16_le(value);
    cursor.write_u16_le(0);
    Ok(())
}

fn describe_result(result: u32) -> String {
    let reason = match result {
        RESULT_ACCESS_DENIED => "server denied access",
        RESULT_LOGON_FAILURE => "incorrect user name or password",
        RESULT_LOGON_TYPE_NOT_GRANTED => "the user has not been granted the requested logon type",
        _ => "rdstls authentication failed",
    };
    format!("{} (resultCode=0x{:08x})", reason, result)
}
