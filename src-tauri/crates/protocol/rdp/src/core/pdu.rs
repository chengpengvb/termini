use std::borrow::Cow;
use crate::codec::bulk;
use crate::codec::bulk::BulkContext;
use crate::core::stream::{ReadCursor, WriteCursor};
use crate::core::error::{RdpProtocolError, RdpResult};

pub const PDU_TYPE_DEMAND_ACTIVE: u16 = 0x1;
pub const PDU_TYPE_CONFIRM_ACTIVE: u16 = 0x3;
pub const PDU_TYPE_DEACTIVATE_ALL: u16 = 0x6;
pub const PDU_TYPE_DATA: u16 = 0x7;
pub const PDU_TYPE_SERVER_REDIRECTION: u16 = 0xA;
pub const PDU_TYPE_FLOW_RESPONSE: u16 = 0x41;
pub const PDU_TYPE_FLOW_STOP: u16 = 0x42;
pub const PDU_TYPE_FLOW_TEST: u16 = 0x43;
pub const PDU_PROTOCOL_VERSION: u16 = 0x10;

pub const PDU_TYPE2_UPDATE: u8 = 2;
pub const PDU_TYPE2_CONTROL: u8 = 20;
pub const PDU_TYPE2_POINTER: u8 = 27;
pub const PDU_TYPE2_INPUT: u8 = 28;
pub const PDU_TYPE2_PLAY_SOUND: u8 = 34;
pub const PDU_TYPE2_SAVE_SESSION_INFO: u8 = 38;
pub const PDU_TYPE2_STATUS_INFO: u8 = 54;
pub const PDU_TYPE2_MONITOR_LAYOUT: u8 = 55;
pub const PDU_TYPE2_SYNCHRONIZE: u8 = 31;
pub const PDU_TYPE2_REFRESH_RECT: u8 = 33;
pub const PDU_TYPE2_SUPPRESS_OUTPUT: u8 = 35;
pub const PDU_TYPE2_SHUTDOWN_REQUEST: u8 = 36;
pub const PDU_TYPE2_FONT_LIST: u8 = 39;
pub const PDU_TYPE2_FONT_MAP: u8 = 40;
pub const PDU_TYPE2_BITMAP_CACHE_PERSISTENT_LIST: u8 = 43;
pub const PDU_TYPE2_SET_ERROR_INFO: u8 = 47;
pub const PDU_TYPE2_FRAME_ACKNOWLEDGE: u8 = 56;

pub const STREAM_LOW: u8 = 1;

pub const CTRL_ACTION_REQUEST_CONTROL: u16 = 1;
pub const CTRL_ACTION_COOPERATE: u16 = 4;

pub const SYNC_MSG_TYPE_SYNC: u16 = 1;

const SHARE_CONTROL_HEADER_LENGTH: usize = 6;
const SHARE_DATA_HEADER_LENGTH: usize = 12;
const SHARE_CONTROL_MIN_LENGTH: u16 = 4;
const SHARE_CONTROL_SOURCE_LENGTH: u16 = 5;
const FLOW_CONTROL_MARKER: u16 = 0x8000;
const DATA_PDU_HEADER_LENGTH: u16 = (SHARE_CONTROL_HEADER_LENGTH + SHARE_DATA_HEADER_LENGTH) as u16;

#[derive(Debug, Clone, Copy)]
pub struct ShareControlHeader {
    pub pdu_type: u16,
}

#[derive(Debug, Clone, Copy)]
pub struct ShareDataHeader {
    pub pdu_type2: u8,
    pub compressed_type: u8,
    pub compressed_length: u16,
}

pub fn reserve_share_control_header(cursor: &mut WriteCursor, pdu_type: u16, pdu_source: u16) -> usize {
    let offset = cursor.len();
    cursor.write_u16_le(0);
    cursor.write_u16_le(pdu_type | PDU_PROTOCOL_VERSION);
    cursor.write_u16_le(pdu_source);
    offset
}

pub fn finish_share_control_header(cursor: &mut WriteCursor, offset: usize) -> RdpResult<()> {
    let length = cursor.len() - offset;
    cursor.patch_u16_le(offset, u16::try_from(length).map_err(|_| RdpProtocolError::encoding("share control pdu too long".to_string()))?)
}

pub fn read_share_control_header(cursor: &mut ReadCursor<'_>) -> RdpResult<ShareControlHeader> {
    let total_length = cursor.read_u16_le()?;
    if total_length == FLOW_CONTROL_MARKER {
        let pdu_type_flow = cursor.read_u8()? as u16;
        cursor.skip(5)?;
        return Ok(ShareControlHeader { pdu_type: pdu_type_flow });
    }
    if total_length < SHARE_CONTROL_MIN_LENGTH {
        return Err(RdpProtocolError::protocol(format!("unexpected share control length: {}", total_length)));
    }
    let pdu_type = cursor.read_u16_le()?;
    if total_length > SHARE_CONTROL_SOURCE_LENGTH {
        cursor.read_u16_le()?;
    }
    Ok(ShareControlHeader { pdu_type: pdu_type & 0x000F })
}

pub fn reserve_share_data_header(cursor: &mut WriteCursor, share_id: u32, pdu_type2: u8) -> usize {
    let offset = cursor.len();
    cursor.write_u32_le(share_id);
    cursor.write_u8(0);
    cursor.write_u8(STREAM_LOW);
    cursor.write_u16_le(0);
    cursor.write_u8(pdu_type2);
    cursor.write_u8(0);
    cursor.write_u16_le(0);
    offset
}

pub fn finish_share_data_header(cursor: &mut WriteCursor, offset: usize) -> RdpResult<()> {
    let length = cursor.len() - offset + SHARE_CONTROL_HEADER_LENGTH;
    cursor.patch_u16_le(offset + 6, u16::try_from(length).map_err(|_| RdpProtocolError::encoding("share data pdu too long".to_string()))?)
}

pub fn read_share_data_header(cursor: &mut ReadCursor<'_>) -> RdpResult<ShareDataHeader> {
    cursor.read_u32_le()?;
    cursor.read_u8()?;
    cursor.read_u8()?;
    cursor.read_u16_le()?;
    let pdu_type2 = cursor.read_u8()?;
    let compressed_type = cursor.read_u8()?;
    let compressed_length = cursor.read_u16_le()?;
    Ok(ShareDataHeader { pdu_type2, compressed_type, compressed_length })
}

pub fn read_data_pdu_body<'a>(cursor: &mut ReadCursor<'a>, header: &ShareDataHeader, bulk: &mut BulkContext) -> RdpResult<Cow<'a, [u8]>> {
    if header.compressed_type & bulk::PACKET_COMPRESSED == 0 {
        return Ok(Cow::Borrowed(cursor.rest()));
    }
    if header.compressed_length < DATA_PDU_HEADER_LENGTH {
        return Err(RdpProtocolError::protocol(format!("compressed data pdu too short: {}", header.compressed_length)));
    }
    let source = cursor.read_slice((header.compressed_length - DATA_PDU_HEADER_LENGTH) as usize)?;
    bulk::decompress(bulk, source, header.compressed_type)
}

pub fn build_data_pdu(share_id: u32, pdu_source: u16, pdu_type2: u8, body: &[u8]) -> RdpResult<Vec<u8>> {
    let mut cursor = WriteCursor::with_capacity(body.len() + SHARE_CONTROL_HEADER_LENGTH + SHARE_DATA_HEADER_LENGTH);
    let control_offset = reserve_share_control_header(&mut cursor, PDU_TYPE_DATA, pdu_source);
    let data_offset = reserve_share_data_header(&mut cursor, share_id, pdu_type2);
    cursor.write_slice(body);
    finish_share_data_header(&mut cursor, data_offset)?;
    finish_share_control_header(&mut cursor, control_offset)?;
    Ok(cursor.into_vec())
}
