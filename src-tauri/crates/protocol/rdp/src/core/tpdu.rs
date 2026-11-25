use crate::core::stream::{ReadCursor, WriteCursor};
use crate::core::error::{RdpProtocolError, RdpResult};

pub const X224_TPDU_CONNECTION_REQUEST: u8 = 0xE0;
pub const X224_TPDU_CONNECTION_CONFIRM: u8 = 0xD0;
pub const X224_TPDU_DISCONNECT_REQUEST: u8 = 0x80;
pub const X224_TPDU_DATA: u8 = 0xF0;

const X224_EOT: u8 = 0x80;
const X224_CONNECT_HEADER_LENGTH: u8 = 6;
const X224_DATA_HEADER_LENGTH: u8 = 2;

pub fn reserve_connect_header(cursor: &mut WriteCursor, code: u8) -> usize {
    let offset = cursor.len();
    cursor.write_u8(X224_CONNECT_HEADER_LENGTH);
    cursor.write_u8(code);
    cursor.write_u16_be(0);
    cursor.write_u16_be(0);
    cursor.write_u8(0);
    offset
}

pub fn finish_connect_header(cursor: &mut WriteCursor, offset: usize) -> RdpResult<()> {
    let length_indicator = cursor.len() - offset - 1;
    if length_indicator > u8::MAX as usize {
        return Err(RdpProtocolError::encoding(format!("x224 header too large: {}", length_indicator)));
    }
    cursor.patch_u8(offset, length_indicator as u8)
}

pub fn write_data_header(cursor: &mut WriteCursor) {
    cursor.write_u8(X224_DATA_HEADER_LENGTH);
    cursor.write_u8(X224_TPDU_DATA);
    cursor.write_u8(X224_EOT);
}

pub fn read_data_header(cursor: &mut ReadCursor<'_>) -> RdpResult<()> {
    let length_indicator = cursor.read_u8()?;
    if length_indicator != X224_DATA_HEADER_LENGTH {
        return Err(RdpProtocolError::protocol(format!("unexpected x224 data length indicator: {}", length_indicator)));
    }
    let code = cursor.read_u8()?;
    if code != X224_TPDU_DATA {
        return Err(RdpProtocolError::protocol(format!("unexpected x224 code: 0x{:02x}", code)));
    }
    cursor.read_u8()?;
    Ok(())
}

pub fn read_connect_header(cursor: &mut ReadCursor<'_>, expected: u8) -> RdpResult<usize> {
    let length_indicator = cursor.read_u8()? as usize;
    if length_indicator < 1 {
        return Err(RdpProtocolError::protocol("x224 length indicator is zero".to_string()));
    }
    let code = cursor.read_u8()?;
    if code != expected {
        if code == X224_TPDU_DISCONNECT_REQUEST {
            return Err(RdpProtocolError::rejected("server sent x224 disconnect request".to_string()));
        }
        return Err(RdpProtocolError::protocol(format!("unexpected x224 code: 0x{:02x}", code)));
    }
    cursor.read_u16_be()?;
    cursor.read_u16_be()?;
    cursor.read_u8()?;
    Ok(length_indicator - (X224_CONNECT_HEADER_LENGTH as usize))
}
