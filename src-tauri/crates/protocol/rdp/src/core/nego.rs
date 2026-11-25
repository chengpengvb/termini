use crate::core::tpdu;
use crate::core::tpkt;
use crate::core::stream::{ReadCursor, WriteCursor};
use crate::core::error::{RdpProtocolError, RdpResult};

pub const PROTOCOL_RDP: u32 = 0x0000_0000;
pub const PROTOCOL_SSL: u32 = 0x0000_0001;
pub const PROTOCOL_HYBRID: u32 = 0x0000_0002;
pub const PROTOCOL_RDSTLS: u32 = 0x0000_0004;
pub const PROTOCOL_HYBRID_EX: u32 = 0x0000_0008;
pub const PROTOCOL_RDSAAD: u32 = 0x0000_0010;

const TYPE_RDP_NEG_REQ: u8 = 0x01;
const TYPE_RDP_NEG_RSP: u8 = 0x02;
const TYPE_RDP_NEG_FAILURE: u8 = 0x03;
const TYPE_RDP_CORRELATION_INFO: u8 = 0x06;

const NEG_LENGTH: u16 = 8;
const CORRELATION_INFO_LENGTH: u16 = 36;

const EXTENDED_CLIENT_DATA_SUPPORTED: u8 = 0x01;

const ROUTING_TOKEN_TERMINATOR: &[u8] = b"\r\n";

const FAILURE_SSL_REQUIRED_BY_SERVER: u32 = 0x0000_0001;
const FAILURE_SSL_NOT_ALLOWED_BY_SERVER: u32 = 0x0000_0002;
const FAILURE_SSL_CERT_NOT_ON_SERVER: u32 = 0x0000_0003;
const FAILURE_INCONSISTENT_FLAGS: u32 = 0x0000_0004;
const FAILURE_HYBRID_REQUIRED_BY_SERVER: u32 = 0x0000_0005;
const FAILURE_SSL_WITH_USER_AUTH_REQUIRED_BY_SERVER: u32 = 0x0000_0006;

#[derive(Debug, Clone, Copy)]
pub struct NegotiationResponse {
    pub selected_protocol: u32,
    pub extended_client_data: bool,
}

pub fn build_connection_request(routing_token: Option<&[u8]>, cookie_user: Option<&str>, requested_protocols: u32) -> RdpResult<Vec<u8>> {
    let mut cursor = WriteCursor::with_capacity(64);
    let tpkt_offset = tpkt::reserve_header(&mut cursor);
    let tpdu_offset = tpdu::reserve_connect_header(&mut cursor, tpdu::X224_TPDU_CONNECTION_REQUEST);

    match routing_token {
        Some(token) => {
            cursor.write_slice(token);
            if !token.ends_with(ROUTING_TOKEN_TERMINATOR) {
                cursor.write_slice(ROUTING_TOKEN_TERMINATOR);
            }
        }
        None => {
            if let Some(user) = cookie_user {
                let cookie = format!("Cookie: mstshash={}\r\n", sanitize_cookie_user(user));
                cursor.write_slice(cookie.as_bytes());
            }
        }
    }

    cursor.write_u8(TYPE_RDP_NEG_REQ);
    cursor.write_u8(0);
    cursor.write_u16_le(NEG_LENGTH);
    cursor.write_u32_le(requested_protocols);

    tpdu::finish_connect_header(&mut cursor, tpdu_offset)?;
    tpkt::finish_header(&mut cursor, tpkt_offset)?;
    Ok(cursor.into_vec())
}

pub fn parse_connection_confirm(payload: &[u8]) -> RdpResult<NegotiationResponse> {
    let mut cursor = ReadCursor::new(payload);
    let variable_length = tpdu::read_connect_header(&mut cursor, tpdu::X224_TPDU_CONNECTION_CONFIRM)?;
    if variable_length == 0 {
        return Ok(NegotiationResponse { selected_protocol: PROTOCOL_RDP, extended_client_data: false });
    }
    parse_negotiation_body(&mut cursor)
}

fn parse_negotiation_body(cursor: &mut ReadCursor<'_>) -> RdpResult<NegotiationResponse> {
    let kind = cursor.read_u8()?;
    let flags = cursor.read_u8()?;
    let length = cursor.read_u16_le()?;
    match kind {
        TYPE_RDP_NEG_RSP => {
            if length != NEG_LENGTH {
                return Err(RdpProtocolError::protocol(format!("unexpected negotiation response length: {}", length)));
            }
            let selected_protocol = cursor.read_u32_le()?;
            Ok(NegotiationResponse { selected_protocol, extended_client_data: flags & EXTENDED_CLIENT_DATA_SUPPORTED != 0 })
        }
        TYPE_RDP_NEG_FAILURE => {
            if length != NEG_LENGTH {
                return Err(RdpProtocolError::protocol(format!("unexpected negotiation failure length: {}", length)));
            }
            let code = cursor.read_u32_le()?;
            Err(RdpProtocolError::rejected(describe_failure(code)))
        }
        TYPE_RDP_CORRELATION_INFO => {
            if length != CORRELATION_INFO_LENGTH {
                return Err(RdpProtocolError::protocol(format!("unexpected correlation info length: {}", length)));
            }
            cursor.skip(CORRELATION_INFO_LENGTH as usize - 4)?;
            parse_negotiation_body(cursor)
        }
        _ => Err(RdpProtocolError::protocol(format!("unexpected negotiation type: 0x{:02x}", kind))),
    }
}

pub fn describe_failure(code: u32) -> String {
    let reason = match code {
        FAILURE_SSL_REQUIRED_BY_SERVER => "server requires TLS",
        FAILURE_SSL_NOT_ALLOWED_BY_SERVER => "server does not allow TLS",
        FAILURE_SSL_CERT_NOT_ON_SERVER => "server has no usable certificate",
        FAILURE_INCONSISTENT_FLAGS => "inconsistent negotiation flags",
        FAILURE_HYBRID_REQUIRED_BY_SERVER => "server requires NLA",
        FAILURE_SSL_WITH_USER_AUTH_REQUIRED_BY_SERVER => "server requires TLS with user authentication",
        _ => "unknown negotiation failure",
    };
    format!("{} (code=0x{:08x})", reason, code)
}

pub fn describe_protocol(protocol: u32) -> &'static str {
    match protocol {
        PROTOCOL_RDP => "RDP",
        PROTOCOL_SSL => "TLS",
        PROTOCOL_HYBRID => "NLA",
        PROTOCOL_RDSTLS => "RDSTLS",
        PROTOCOL_HYBRID_EX => "NLA-EX",
        PROTOCOL_RDSAAD => "RDSAAD",
        _ => "UNKNOWN",
    }
}

fn sanitize_cookie_user(user: &str) -> String {
    let trimmed: String = user.chars().filter(|c| !c.is_control() && *c != ';' && *c != ',').take(9).collect();
    trimmed
}
