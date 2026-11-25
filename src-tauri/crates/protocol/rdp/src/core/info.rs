use crate::codec::bulk;
use crate::core::timezone;
use crate::core::security::hmac_md5;
use crate::core::stream::WriteCursor;
use crate::core::error::{RdpProtocolError, RdpResult};
use crate::core::logon::{AutoReconnectCookie, AUTO_RECONNECT_COOKIE_LENGTH};

pub const INFO_MOUSE: u32 = 0x0000_0001;
pub const INFO_DISABLE_CTRL_ALT_DEL: u32 = 0x0000_0002;
pub const INFO_AUTOLOGON: u32 = 0x0000_0008;
pub const INFO_UNICODE: u32 = 0x0000_0010;
pub const INFO_MAXIMIZE_SHELL: u32 = 0x0000_0020;
pub const INFO_LOGON_NOTIFY: u32 = 0x0000_0040;
pub const INFO_COMPRESSION: u32 = 0x0000_0080;
pub const INFO_ENABLE_WINDOWS_KEY: u32 = 0x0000_0100;
pub const INFO_FORCE_ENCRYPTED_CS_PDU: u32 = 0x0000_4000;
pub const INFO_LOGON_ERRORS: u32 = 0x0001_0000;
pub const INFO_MOUSE_HAS_WHEEL: u32 = 0x0002_0000;

pub const PERF_DISABLE_WALLPAPER: u32 = 0x0000_0001;
pub const PERF_DISABLE_FULLWINDOWDRAG: u32 = 0x0000_0002;
pub const PERF_DISABLE_MENUANIMATIONS: u32 = 0x0000_0004;
pub const PERF_DISABLE_THEMING: u32 = 0x0000_0008;
pub const PERF_DISABLE_CURSOR_SHADOW: u32 = 0x0000_0020;
pub const PERF_DISABLE_CURSORSETTINGS: u32 = 0x0000_0040;

const CODE_PAGE_UNICODE: u32 = 0;
const CLIENT_RANDOM_LENGTH: usize = 32;
const COMPRESSION_LEVEL_SHIFT: u32 = 9;
const COMPRESSION_LEVEL_MASK: u32 = 0x0000_1E00;
const ADDRESS_FAMILY_INET: u16 = 0x0002;
const CLIENT_ADDRESS: &str = "0.0.0.0";
const CLIENT_DIR: &str = "C:\\Windows\\System32\\mstscax.dll";

pub struct ClientInfoSettings {
    pub domain: String,
    pub username: String,
    pub password: String,
    pub performance_flags: u32,
    pub redirection_password: Option<Vec<u8>>,
    pub auto_reconnect: Option<AutoReconnectCookie>,
}

pub fn build_client_info(settings: &ClientInfoSettings) -> RdpResult<Vec<u8>> {
    let mut cursor = WriteCursor::with_capacity(512);
    let flags = INFO_MOUSE
        | INFO_DISABLE_CTRL_ALT_DEL
        | INFO_AUTOLOGON
        | INFO_UNICODE
        | INFO_MAXIMIZE_SHELL
        | INFO_LOGON_NOTIFY
        | INFO_ENABLE_WINDOWS_KEY
        | INFO_LOGON_ERRORS
        | INFO_MOUSE_HAS_WHEEL
        | INFO_FORCE_ENCRYPTED_CS_PDU
        | INFO_COMPRESSION
        | ((u32::from(bulk::PACKET_COMPR_TYPE_RDP61) << COMPRESSION_LEVEL_SHIFT) & COMPRESSION_LEVEL_MASK);

    let password = match &settings.redirection_password {
        Some(password) => password.clone(),
        None => encode_utf16(&settings.password),
    };
    cursor.write_u32_le(CODE_PAGE_UNICODE);
    cursor.write_u32_le(flags);
    cursor.write_u16_le(utf16_byte_length(&settings.domain)?);
    cursor.write_u16_le(utf16_byte_length(&settings.username)?);
    cursor.write_u16_le(u16::try_from(password.len()).map_err(|_| RdpProtocolError::encoding(format!("client info password too long: {}", password.len())))?);
    cursor.write_u16_le(0);
    cursor.write_u16_le(0);
    write_utf16_field(&mut cursor, &settings.domain);
    write_utf16_field(&mut cursor, &settings.username);
    cursor.write_slice(&password);
    cursor.write_u16_le(0);
    write_utf16_field(&mut cursor, "");
    write_utf16_field(&mut cursor, "");

    cursor.write_u16_le(ADDRESS_FAMILY_INET);
    cursor.write_u16_le(utf16_byte_length(CLIENT_ADDRESS)? + 2);
    write_utf16_field(&mut cursor, CLIENT_ADDRESS);
    cursor.write_u16_le(utf16_byte_length(CLIENT_DIR)? + 2);
    write_utf16_field(&mut cursor, CLIENT_DIR);
    timezone::write_time_zone_information(&mut cursor)?;
    cursor.write_u32_le(0);
    cursor.write_u32_le(settings.performance_flags);
    write_auto_reconnect_cookie(&mut cursor, settings.auto_reconnect.as_ref())?;
    timezone::write_dynamic_time_zone(&mut cursor)?;
    Ok(cursor.into_vec())
}

fn write_auto_reconnect_cookie(cursor: &mut WriteCursor, cookie: Option<&AutoReconnectCookie>) -> RdpResult<()> {
    let Some(cookie) = cookie else {
        cursor.write_u16_le(0);
        return Ok(());
    };
    let verifier = hmac_md5(&cookie.random_bits, &[&[0u8; CLIENT_RANDOM_LENGTH]])?;
    cursor.write_u16_le(AUTO_RECONNECT_COOKIE_LENGTH as u16);
    cursor.write_u32_le(AUTO_RECONNECT_COOKIE_LENGTH);
    cursor.write_u32_le(cookie.version);
    cursor.write_u32_le(cookie.logon_id);
    cursor.write_slice(&verifier);
    Ok(())
}

pub fn default_performance_flags() -> u32 {
    PERF_DISABLE_WALLPAPER | PERF_DISABLE_FULLWINDOWDRAG | PERF_DISABLE_MENUANIMATIONS | PERF_DISABLE_THEMING | PERF_DISABLE_CURSOR_SHADOW | PERF_DISABLE_CURSORSETTINGS
}

fn utf16_byte_length(value: &str) -> RdpResult<u16> {
    let length = value.encode_utf16().count() * 2;
    u16::try_from(length).map_err(|_| RdpProtocolError::encoding(format!("client info field too long: {}", length)))
}

fn write_utf16_field(cursor: &mut WriteCursor, value: &str) {
    cursor.write_utf16_le(value);
    cursor.write_u16_le(0);
}

fn encode_utf16(value: &str) -> Vec<u8> {
    value.encode_utf16().flat_map(|unit| unit.to_le_bytes()).collect()
}
