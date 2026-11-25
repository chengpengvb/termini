use crate::core::ber;
use crate::core::per;
use crate::core::tpdu;
use crate::core::tpkt;
use crate::core::stream::{ReadCursor, WriteCursor};
use crate::core::error::{RdpProtocolError, RdpResult};

pub const MCS_BASE_CHANNEL_ID: u16 = 1001;

const DOMAIN_MCS_ERECT_DOMAIN_REQUEST: u8 = 1;
const DOMAIN_MCS_DISCONNECT_PROVIDER_ULTIMATUM: u8 = 8;
const DOMAIN_MCS_ATTACH_USER_REQUEST: u8 = 10;
const DOMAIN_MCS_ATTACH_USER_CONFIRM: u8 = 11;
const DOMAIN_MCS_CHANNEL_JOIN_REQUEST: u8 = 14;
const DOMAIN_MCS_CHANNEL_JOIN_CONFIRM: u8 = 15;
const DOMAIN_MCS_SEND_DATA_REQUEST: u8 = 25;
const DOMAIN_MCS_SEND_DATA_INDICATION: u8 = 26;

const MCS_RESULT_ENUM_LENGTH: u8 = 16;
const CONNECT_INITIAL_TAG: u8 = 101;
const CONNECT_RESPONSE_TAG: u8 = 102;

const SEND_DATA_PRIORITY: u8 = 0x70;

pub struct SendDataIndication {
    pub channel_id: u16,
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone, Copy)]
struct DomainParameters {
    max_channel_ids: u32,
    max_user_ids: u32,
    max_token_ids: u32,
    num_priorities: u32,
    min_throughput: u32,
    max_height: u32,
    max_mcs_pdu_size: u32,
    protocol_version: u32,
}

impl DomainParameters {
    fn target() -> Self {
        Self { max_channel_ids: 34, max_user_ids: 2, max_token_ids: 0, num_priorities: 1, min_throughput: 0, max_height: 1, max_mcs_pdu_size: 0xFFFF, protocol_version: 2 }
    }

    fn minimum() -> Self {
        Self { max_channel_ids: 1, max_user_ids: 1, max_token_ids: 1, num_priorities: 1, min_throughput: 0, max_height: 1, max_mcs_pdu_size: 0x420, protocol_version: 2 }
    }

    fn maximum() -> Self {
        Self { max_channel_ids: 0xFFFF, max_user_ids: 0xFC17, max_token_ids: 0xFFFF, num_priorities: 1, min_throughput: 0, max_height: 1, max_mcs_pdu_size: 0xFFFF, protocol_version: 2 }
    }

    fn encode(&self) -> Vec<u8> {
        let mut content = Vec::with_capacity(48);
        for value in [self.max_channel_ids, self.max_user_ids, self.max_token_ids, self.num_priorities, self.min_throughput, self.max_height, self.max_mcs_pdu_size, self.protocol_version] {
            content.extend_from_slice(&ber::encode_integer(value));
        }
        ber::encode_sequence(&content)
    }
}

pub fn build_connect_initial(user_data: &[u8]) -> RdpResult<Vec<u8>> {
    let mut content = Vec::with_capacity(user_data.len() + 128);
    content.extend_from_slice(&ber::encode_octet_string(&[]));
    content.extend_from_slice(&ber::encode_octet_string(&[]));
    content.extend_from_slice(&ber::encode_boolean(true));
    content.extend_from_slice(&DomainParameters::target().encode());
    content.extend_from_slice(&DomainParameters::minimum().encode());
    content.extend_from_slice(&DomainParameters::maximum().encode());
    content.extend_from_slice(&ber::encode_octet_string(user_data));
    let body = ber::encode_application(CONNECT_INITIAL_TAG, &content);
    wrap_x224_data(&body)
}

pub fn parse_connect_response(payload: &[u8]) -> RdpResult<Vec<u8>> {
    let mut cursor = ReadCursor::new(payload);
    tpdu::read_data_header(&mut cursor)?;
    let mut reader = ber::BerReader::new(cursor.rest());
    let content = reader.read_application(CONNECT_RESPONSE_TAG)?;
    let mut fields = ber::BerReader::new(content);
    let result = fields.read_enumerated()?;
    if result != 0 {
        return Err(RdpProtocolError::rejected(format!("MCS connect rejected: result={}", result)));
    }
    fields.read_integer()?;
    fields.read_sequence()?;
    Ok(fields.read_octet_string()?.to_vec())
}

pub fn build_erect_domain_request() -> RdpResult<Vec<u8>> {
    let mut cursor = WriteCursor::with_capacity(8);
    per::write_choice(&mut cursor, DOMAIN_MCS_ERECT_DOMAIN_REQUEST << 2);
    per::write_integer(&mut cursor, 0);
    per::write_integer(&mut cursor, 0);
    wrap_x224_data(cursor.as_slice())
}

pub fn build_attach_user_request() -> RdpResult<Vec<u8>> {
    let mut cursor = WriteCursor::with_capacity(4);
    per::write_choice(&mut cursor, DOMAIN_MCS_ATTACH_USER_REQUEST << 2);
    wrap_x224_data(cursor.as_slice())
}

pub fn parse_attach_user_confirm(payload: &[u8]) -> RdpResult<u16> {
    let mut cursor = ReadCursor::new(payload);
    tpdu::read_data_header(&mut cursor)?;
    let choice = per::read_choice(&mut cursor)?;
    if choice >> 2 != DOMAIN_MCS_ATTACH_USER_CONFIRM {
        return Err(RdpProtocolError::protocol(format!("MCS expected attach user confirm, got {}", choice >> 2)));
    }
    let result = per::read_enumerated(&mut cursor, MCS_RESULT_ENUM_LENGTH)?;
    if result != 0 {
        return Err(RdpProtocolError::rejected(format!("MCS attach user rejected: result={}", result)));
    }
    per::read_integer16(&mut cursor, MCS_BASE_CHANNEL_ID)
}

pub fn build_channel_join_request(user_id: u16, channel_id: u16) -> RdpResult<Vec<u8>> {
    let mut cursor = WriteCursor::with_capacity(8);
    per::write_choice(&mut cursor, DOMAIN_MCS_CHANNEL_JOIN_REQUEST << 2);
    per::write_integer16(&mut cursor, user_id, MCS_BASE_CHANNEL_ID)?;
    per::write_integer16(&mut cursor, channel_id, 0)?;
    wrap_x224_data(cursor.as_slice())
}

pub fn parse_channel_join_confirm(payload: &[u8]) -> RdpResult<u16> {
    let mut cursor = ReadCursor::new(payload);
    tpdu::read_data_header(&mut cursor)?;
    let choice = per::read_choice(&mut cursor)?;
    if choice >> 2 != DOMAIN_MCS_CHANNEL_JOIN_CONFIRM {
        return Err(RdpProtocolError::protocol(format!("MCS expected channel join confirm, got {}", choice >> 2)));
    }
    let result = per::read_enumerated(&mut cursor, MCS_RESULT_ENUM_LENGTH)?;
    if result != 0 {
        return Err(RdpProtocolError::rejected(format!("MCS channel join rejected: result={}", result)));
    }
    per::read_integer16(&mut cursor, MCS_BASE_CHANNEL_ID)?;
    let requested = per::read_integer16(&mut cursor, 0)?;
    Ok(requested)
}

pub fn build_send_data_request(user_id: u16, channel_id: u16, data: &[u8]) -> RdpResult<Vec<u8>> {
    let mut cursor = WriteCursor::with_capacity(data.len() + 16);
    let tpkt_offset = tpkt::reserve_header(&mut cursor);
    tpdu::write_data_header(&mut cursor);
    per::write_choice(&mut cursor, DOMAIN_MCS_SEND_DATA_REQUEST << 2);
    per::write_integer16(&mut cursor, user_id, MCS_BASE_CHANNEL_ID)?;
    per::write_integer16(&mut cursor, channel_id, 0)?;
    cursor.write_u8(SEND_DATA_PRIORITY);
    per::write_length(&mut cursor, u16::try_from(data.len()).map_err(|_| RdpProtocolError::encoding("MCS data too long".to_string()))?)?;
    cursor.write_slice(data);
    tpkt::finish_header(&mut cursor, tpkt_offset)?;
    Ok(cursor.into_vec())
}

pub fn parse_send_data_indication(payload: &[u8]) -> RdpResult<SendDataIndication> {
    let mut cursor = ReadCursor::new(payload);
    tpdu::read_data_header(&mut cursor)?;
    let choice = per::read_choice(&mut cursor)?;
    let kind = choice >> 2;
    if kind == DOMAIN_MCS_DISCONNECT_PROVIDER_ULTIMATUM {
        let b2 = cursor.read_u8()?;
        let reason = ((choice & 0x01) << 1) | (b2 >> 7);
        return Err(RdpProtocolError::rejected(format!("server disconnected MCS connection: reason={}", reason)));
    }
    if kind != DOMAIN_MCS_SEND_DATA_INDICATION {
        return Err(RdpProtocolError::protocol(format!("MCS expected send data indication, got {}", kind)));
    }
    per::read_integer16(&mut cursor, MCS_BASE_CHANNEL_ID)?;
    let channel_id = per::read_integer16(&mut cursor, 0)?;
    cursor.read_u8()?;
    let length = per::read_length(&mut cursor)? as usize;
    Ok(SendDataIndication { channel_id, payload: cursor.read_vec(length)? })
}

fn wrap_x224_data(body: &[u8]) -> RdpResult<Vec<u8>> {
    let mut cursor = WriteCursor::with_capacity(body.len() + 8);
    let tpkt_offset = tpkt::reserve_header(&mut cursor);
    tpdu::write_data_header(&mut cursor);
    cursor.write_slice(body);
    tpkt::finish_header(&mut cursor, tpkt_offset)?;
    Ok(cursor.into_vec())
}
