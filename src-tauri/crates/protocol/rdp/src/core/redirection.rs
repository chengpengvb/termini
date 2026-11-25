use crate::core::stream::ReadCursor;
use crate::core::error::{RdpProtocolError, RdpResult};

const SEC_REDIRECTION_PKT: u16 = 0x0400;

const LB_TARGET_NET_ADDRESS: u32 = 0x0000_0001;
const LB_LOAD_BALANCE_INFO: u32 = 0x0000_0002;
const LB_USERNAME: u32 = 0x0000_0004;
const LB_DOMAIN: u32 = 0x0000_0008;
const LB_PASSWORD: u32 = 0x0000_0010;
const LB_NOREDIRECT: u32 = 0x0000_0080;
const LB_TARGET_FQDN: u32 = 0x0000_0100;
const LB_TARGET_NETBIOS_NAME: u32 = 0x0000_0200;
const LB_TARGET_NET_ADDRESSES: u32 = 0x0000_0800;
const LB_CLIENT_TSV_URL: u32 = 0x0000_1000;
const LB_PASSWORD_IS_PK_ENCRYPTED: u32 = 0x0000_4000;
const LB_REDIRECTION_GUID: u32 = 0x0000_8000;
const LB_TARGET_CERTIFICATE: u32 = 0x0001_0000;

const PASSWORD_MAX_LENGTH: usize = 512;
const TRAILING_PAD_LENGTH: usize = 8;

#[derive(Default, Clone)]
pub struct Redirection {
    pub session_id: u32,
    pub username: Option<String>,
    pub domain: Option<String>,
    pub password: Option<Vec<u8>>,
    pub target_fqdn: Option<String>,
    pub target_net_address: Option<String>,
    pub target_netbios_name: Option<String>,
    pub target_net_addresses: Vec<String>,
    pub load_balance_info: Option<Vec<u8>>,
    pub redirection_guid: Option<Vec<u8>>,
    pub password_is_encrypted: bool,
}

impl Redirection {
    pub fn target_host(&self) -> Option<&str> {
        if self.load_balance_info.is_some() {
            return None;
        }
        self.target_fqdn.as_deref().or(self.target_net_address.as_deref()).or(self.target_netbios_name.as_deref()).or_else(|| self.target_net_addresses.first().map(String::as_str))
    }
}

pub fn parse(cursor: &mut ReadCursor<'_>) -> RdpResult<Option<Redirection>> {
    cursor.skip(2)?;
    let flags = cursor.read_u16_le()?;
    if flags != SEC_REDIRECTION_PKT {
        return Err(RdpProtocolError::protocol(format!("unexpected redirection packet security flags: 0x{:04x}", flags)));
    }
    cursor.read_u16_le()?;
    let mut redirection = Redirection { session_id: cursor.read_u32_le()?, ..Default::default() };
    let redirection_flags = cursor.read_u32_le()?;
    redirection.password_is_encrypted = redirection_flags & LB_PASSWORD_IS_PK_ENCRYPTED != 0;

    if redirection_flags & LB_TARGET_NET_ADDRESS != 0 {
        redirection.target_net_address = Some(read_unicode(cursor)?);
    }
    if redirection_flags & LB_LOAD_BALANCE_INFO != 0 {
        redirection.load_balance_info = Some(read_binary(cursor)?);
    }
    if redirection_flags & LB_USERNAME != 0 {
        redirection.username = Some(read_unicode(cursor)?);
    }
    if redirection_flags & LB_DOMAIN != 0 {
        redirection.domain = Some(read_unicode(cursor)?);
    }
    if redirection_flags & LB_PASSWORD != 0 {
        let password = read_binary(cursor)?;
        if !redirection.password_is_encrypted && password.len() > PASSWORD_MAX_LENGTH {
            return Err(RdpProtocolError::protocol(format!("redirection password length out of range: {}", password.len())));
        }
        redirection.password = Some(password);
    }
    if redirection_flags & LB_TARGET_FQDN != 0 {
        redirection.target_fqdn = Some(read_unicode(cursor)?);
    }
    if redirection_flags & LB_TARGET_NETBIOS_NAME != 0 {
        redirection.target_netbios_name = Some(read_unicode(cursor)?);
    }
    if redirection_flags & LB_CLIENT_TSV_URL != 0 {
        read_binary(cursor)?;
    }
    if redirection_flags & LB_REDIRECTION_GUID != 0 {
        redirection.redirection_guid = Some(read_binary(cursor)?);
    }
    if redirection_flags & LB_TARGET_CERTIFICATE != 0 {
        read_binary(cursor)?;
    }
    if redirection_flags & LB_TARGET_NET_ADDRESSES != 0 {
        cursor.read_u32_le()?;
        let count = cursor.read_u32_le()?;
        if count as usize * 2 > cursor.remaining() {
            return Err(RdpProtocolError::protocol(format!("redirection address count out of range: {}", count)));
        }
        for _ in 0..count {
            redirection.target_net_addresses.push(read_unicode(cursor)?);
        }
    }
    if cursor.remaining() >= TRAILING_PAD_LENGTH {
        cursor.skip(TRAILING_PAD_LENGTH)?;
    }
    match redirection_flags & LB_NOREDIRECT != 0 {
        true => Ok(None),
        false => Ok(Some(redirection)),
    }
}

fn read_binary(cursor: &mut ReadCursor<'_>) -> RdpResult<Vec<u8>> {
    let length = cursor.read_u32_le()? as usize;
    cursor.read_vec(length)
}

fn read_unicode(cursor: &mut ReadCursor<'_>) -> RdpResult<String> {
    let length = cursor.read_u32_le()? as usize;
    if length % 2 != 0 {
        return Err(RdpProtocolError::protocol(format!("invalid redirection string length: {}", length)));
    }
    let raw = cursor.read_slice(length)?;
    let units = raw.chunks_exact(2).map(|pair| u16::from_le_bytes([pair[0], pair[1]])).take_while(|unit| *unit != 0).collect::<Vec<u16>>();
    String::from_utf16(&units).map_err(|error| RdpProtocolError::encoding(format!("invalid redirection string: {}", error)))
}
