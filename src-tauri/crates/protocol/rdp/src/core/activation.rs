use crate::core::pdu;
use crate::core::stream::{ReadCursor, WriteCursor};
use crate::core::error::{RdpProtocolError, RdpResult};
use crate::core::capabilities::{self, ClientCapabilities, ServerCapabilities};

const SOURCE_DESCRIPTOR: &[u8] = b"MSTSC\0";
const ORIGINATOR_ID: u16 = 0x03EA;
const FONT_LIST_FLAGS_FIRST_LAST: u16 = 0x0003;
const FONT_LIST_ENTRY_SIZE: u16 = 0x0032;
const PERSIST_FIRST_PDU: u8 = 0x01;
const PERSIST_LAST_PDU: u8 = 0x02;

#[derive(Debug, Clone, Copy)]
pub struct DemandActive {
    pub share_id: u32,
    pub capabilities: ServerCapabilities,
}

pub fn parse_demand_active(cursor: &mut ReadCursor<'_>) -> RdpResult<DemandActive> {
    let share_id = cursor.read_u32_le()?;
    let length_source_descriptor = cursor.read_u16_le()? as usize;
    let length_combined_capabilities = cursor.read_u16_le()? as usize;
    cursor.skip(length_source_descriptor)?;
    if length_combined_capabilities < 4 {
        return Err(RdpProtocolError::protocol(format!("unexpected Demand Active capability set length: {}", length_combined_capabilities)));
    }
    let mut combined = cursor.sub_cursor(length_combined_capabilities)?;
    let count = combined.read_u16_le()?;
    combined.read_u16_le()?;
    let capabilities = capabilities::parse_server_capability_sets(&mut combined, count)?;
    Ok(DemandActive { share_id, capabilities })
}

pub fn build_confirm_active(share_id: u32, user_id: u16, settings: &ClientCapabilities) -> RdpResult<Vec<u8>> {
    let mut capability_sets = WriteCursor::with_capacity(1024);
    let count = capabilities::write_client_capability_sets(&mut capability_sets, settings)?;
    let capability_sets = capability_sets.into_vec();

    let mut cursor = WriteCursor::with_capacity(capability_sets.len() + 64);
    let control_offset = pdu::reserve_share_control_header(&mut cursor, pdu::PDU_TYPE_CONFIRM_ACTIVE, user_id);
    cursor.write_u32_le(share_id);
    cursor.write_u16_le(ORIGINATOR_ID);
    cursor.write_u16_le(SOURCE_DESCRIPTOR.len() as u16);
    cursor.write_u16_le(u16::try_from(capability_sets.len() + 4).map_err(|_| RdpProtocolError::encoding("total capability set length overflow".to_string()))?);
    cursor.write_slice(SOURCE_DESCRIPTOR);
    cursor.write_u16_le(count);
    cursor.write_u16_le(0);
    cursor.write_slice(&capability_sets);
    pdu::finish_share_control_header(&mut cursor, control_offset)?;
    Ok(cursor.into_vec())
}

pub fn build_synchronize(share_id: u32, user_id: u16, target_user: u16) -> RdpResult<Vec<u8>> {
    let mut body = WriteCursor::with_capacity(4);
    body.write_u16_le(pdu::SYNC_MSG_TYPE_SYNC);
    body.write_u16_le(target_user);
    pdu::build_data_pdu(share_id, user_id, pdu::PDU_TYPE2_SYNCHRONIZE, body.as_slice())
}

pub fn build_control(share_id: u32, user_id: u16, action: u16) -> RdpResult<Vec<u8>> {
    let mut body = WriteCursor::with_capacity(8);
    body.write_u16_le(action);
    body.write_u16_le(0);
    body.write_u32_le(0);
    pdu::build_data_pdu(share_id, user_id, pdu::PDU_TYPE2_CONTROL, body.as_slice())
}

pub fn build_font_list(share_id: u32, user_id: u16) -> RdpResult<Vec<u8>> {
    let mut body = WriteCursor::with_capacity(8);
    body.write_u16_le(0);
    body.write_u16_le(0);
    body.write_u16_le(FONT_LIST_FLAGS_FIRST_LAST);
    body.write_u16_le(FONT_LIST_ENTRY_SIZE);
    pdu::build_data_pdu(share_id, user_id, pdu::PDU_TYPE2_FONT_LIST, body.as_slice())
}

pub fn build_persistent_key_list(share_id: u32, user_id: u16, cells: &[Vec<u64>; 5]) -> RdpResult<Vec<u8>> {
    let mut body = WriteCursor::with_capacity(24 + cells.iter().map(Vec::len).sum::<usize>() * 8);
    for cell in cells {
        body.write_u16_le(u16::try_from(cell.len()).map_err(|_| RdpProtocolError::encoding(format!("too many persistent bitmap cache entries: {}", cell.len())))?);
    }
    for cell in cells {
        body.write_u16_le(u16::try_from(cell.len()).map_err(|_| RdpProtocolError::encoding(format!("persistent bitmap cache total too large: {}", cell.len())))?);
    }
    body.write_u8(PERSIST_FIRST_PDU | PERSIST_LAST_PDU);
    body.write_u8(0);
    body.write_u16_le(0);
    for cell in cells {
        for key in cell {
            body.write_u32_le(*key as u32);
            body.write_u32_le((*key >> 32) as u32);
        }
    }
    pdu::build_data_pdu(share_id, user_id, pdu::PDU_TYPE2_BITMAP_CACHE_PERSISTENT_LIST, body.as_slice())
}

pub fn build_refresh_rect(share_id: u32, user_id: u16, width: u16, height: u16) -> RdpResult<Vec<u8>> {
    let mut body = WriteCursor::with_capacity(12);
    body.write_u8(1);
    body.write_zeros(3);
    body.write_u16_le(0);
    body.write_u16_le(0);
    body.write_u16_le(width.saturating_sub(1));
    body.write_u16_le(height.saturating_sub(1));
    pdu::build_data_pdu(share_id, user_id, pdu::PDU_TYPE2_REFRESH_RECT, body.as_slice())
}

pub fn build_suppress_output(share_id: u32, user_id: u16, allow: bool, width: u16, height: u16) -> RdpResult<Vec<u8>> {
    let mut body = WriteCursor::with_capacity(12);
    body.write_u8(u8::from(allow));
    body.write_zeros(3);
    if allow {
        body.write_u16_le(0);
        body.write_u16_le(0);
        body.write_u16_le(width);
        body.write_u16_le(height);
    }
    pdu::build_data_pdu(share_id, user_id, pdu::PDU_TYPE2_SUPPRESS_OUTPUT, body.as_slice())
}

pub fn build_frame_acknowledge(share_id: u32, user_id: u16, frame_id: u32) -> RdpResult<Vec<u8>> {
    let mut body = WriteCursor::with_capacity(4);
    body.write_u32_le(frame_id);
    pdu::build_data_pdu(share_id, user_id, pdu::PDU_TYPE2_FRAME_ACKNOWLEDGE, body.as_slice())
}

pub fn build_shutdown_request(share_id: u32, user_id: u16) -> RdpResult<Vec<u8>> {
    pdu::build_data_pdu(share_id, user_id, pdu::PDU_TYPE2_SHUTDOWN_REQUEST, &[])
}
