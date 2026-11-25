use crate::core::stream::ReadCursor;
use crate::core::error::{RdpProtocolError, RdpResult};

const INFO_TYPE_LOGON: u32 = 0x0000_0000;
const INFO_TYPE_LOGON_LONG: u32 = 0x0000_0001;
const INFO_TYPE_LOGON_PLAIN_NOTIFY: u32 = 0x0000_0002;
const INFO_TYPE_LOGON_EXTENDED_INF: u32 = 0x0000_0003;

const LOGON_EX_AUTORECONNECTCOOKIE: u32 = 0x0000_0001;
const LOGON_EX_LOGONERRORS: u32 = 0x0000_0002;

const SAVE_SESSION_PDU_VERSION_ONE: u16 = 0x0001;

const LOGON_INFO_V1_DOMAIN_LENGTH: usize = 52;
const LOGON_INFO_V1_USERNAME_LENGTH: usize = 512;
const LOGON_INFO_V2_SIZE: u32 = 18;
const LOGON_INFO_V2_TOTAL_SIZE: u32 = 576;
const LOGON_INFO_V2_RESERVED: usize = 558;
const LOGON_PLAIN_NOTIFY_LENGTH: usize = 576;
const LOGON_EXTENDED_HEADER_LENGTH: u16 = 6;
const LOGON_EXTENDED_PAD_LENGTH: usize = 570;
const LOGON_ERROR_LENGTH: usize = 8;

pub const AUTO_RECONNECT_COOKIE_LENGTH: u32 = 28;
const AUTO_RECONNECT_RANDOM_LENGTH: usize = 16;

const MONITOR_LAYOUT_ENTRY_LENGTH: usize = 20;

#[derive(Clone, Copy)]
pub struct AutoReconnectCookie {
    pub version: u32,
    pub logon_id: u32,
    pub random_bits: [u8; AUTO_RECONNECT_RANDOM_LENGTH],
}

pub struct PlaySound {
    pub duration: u32,
    pub frequency: u32,
}

pub fn parse_save_session_info(cursor: &mut ReadCursor<'_>) -> RdpResult<Option<AutoReconnectCookie>> {
    let info_type = cursor.read_u32_le()?;
    match info_type {
        INFO_TYPE_LOGON => {
            read_logon_info_v1(cursor)?;
            Ok(None)
        }
        INFO_TYPE_LOGON_LONG => {
            read_logon_info_v2(cursor)?;
            Ok(None)
        }
        INFO_TYPE_LOGON_PLAIN_NOTIFY => {
            cursor.skip(LOGON_PLAIN_NOTIFY_LENGTH)?;
            Ok(None)
        }
        INFO_TYPE_LOGON_EXTENDED_INF => read_logon_info_extended(cursor),
        other => Err(RdpProtocolError::protocol(format!("unknown logon info type: 0x{:08x}", other))),
    }
}

pub fn parse_status_info(cursor: &mut ReadCursor<'_>) -> RdpResult<u32> {
    cursor.read_u32_le()
}

pub fn parse_play_sound(cursor: &mut ReadCursor<'_>) -> RdpResult<PlaySound> {
    let duration = cursor.read_u32_le()?;
    let frequency = cursor.read_u32_le()?;
    Ok(PlaySound { duration, frequency })
}

pub fn parse_monitor_layout(cursor: &mut ReadCursor<'_>) -> RdpResult<u32> {
    let count = cursor.read_u32_le()?;
    cursor.skip(count as usize * MONITOR_LAYOUT_ENTRY_LENGTH)?;
    Ok(count)
}

fn read_logon_info_v1(cursor: &mut ReadCursor<'_>) -> RdpResult<()> {
    cursor.read_u32_le()?;
    cursor.skip(LOGON_INFO_V1_DOMAIN_LENGTH)?;
    cursor.read_u32_le()?;
    cursor.skip(LOGON_INFO_V1_USERNAME_LENGTH)?;
    cursor.read_u32_le()?;
    Ok(())
}

fn read_logon_info_v2(cursor: &mut ReadCursor<'_>) -> RdpResult<()> {
    let version = cursor.read_u16_le()?;
    if version != SAVE_SESSION_PDU_VERSION_ONE {
        return Err(RdpProtocolError::protocol(format!("unknown logon info version: {}", version)));
    }
    let size = cursor.read_u32_le()?;
    if size != LOGON_INFO_V2_SIZE && size != LOGON_INFO_V2_TOTAL_SIZE {
        return Err(RdpProtocolError::protocol(format!("unexpected logon info length: {}", size)));
    }
    cursor.read_u32_le()?;
    let domain = cursor.read_u32_le()? as usize;
    let username = cursor.read_u32_le()? as usize;
    cursor.skip(LOGON_INFO_V2_RESERVED)?;
    cursor.skip(domain)?;
    cursor.skip(username)?;
    Ok(())
}

fn read_logon_info_extended(cursor: &mut ReadCursor<'_>) -> RdpResult<Option<AutoReconnectCookie>> {
    let length = cursor.read_u16_le()?;
    let fields = cursor.read_u32_le()?;
    if length < LOGON_EXTENDED_HEADER_LENGTH {
        return Err(RdpProtocolError::protocol(format!("unexpected extended logon info length: {}", length)));
    }
    let mut cookie = None;
    if fields & LOGON_EX_AUTORECONNECTCOOKIE != 0 {
        cursor.read_u32_le()?;
        cookie = Some(read_auto_reconnect_cookie(cursor)?);
    }
    if fields & LOGON_EX_LOGONERRORS != 0 {
        cursor.read_u32_le()?;
        cursor.skip(LOGON_ERROR_LENGTH)?;
    }
    cursor.skip(LOGON_EXTENDED_PAD_LENGTH)?;
    Ok(cookie)
}

fn read_auto_reconnect_cookie(cursor: &mut ReadCursor<'_>) -> RdpResult<AutoReconnectCookie> {
    let cb_len = cursor.read_u32_le()?;
    if cb_len != AUTO_RECONNECT_COOKIE_LENGTH {
        return Err(RdpProtocolError::protocol(format!("unexpected auto-reconnect cookie length: {}", cb_len)));
    }
    let version = cursor.read_u32_le()?;
    let logon_id = cursor.read_u32_le()?;
    let mut random_bits = [0u8; AUTO_RECONNECT_RANDOM_LENGTH];
    random_bits.copy_from_slice(cursor.read_slice(AUTO_RECONNECT_RANDOM_LENGTH)?);
    Ok(AutoReconnectCookie { version, logon_id, random_bits })
}
