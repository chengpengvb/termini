use crate::core::stream::WriteCursor;
use crate::core::error::{RdpProtocolError, RdpResult};

pub const TPKT_VERSION: u8 = 3;
pub const TPKT_HEADER_LENGTH: usize = 4;

pub fn write_header(cursor: &mut WriteCursor, length: u16) {
    cursor.write_u8(TPKT_VERSION);
    cursor.write_u8(0);
    cursor.write_u16_be(length);
}

pub fn reserve_header(cursor: &mut WriteCursor) -> usize {
    let offset = cursor.len();
    write_header(cursor, 0);
    offset
}

pub fn finish_header(cursor: &mut WriteCursor, offset: usize) -> RdpResult<()> {
    let total = cursor.len() - offset;
    if total > u16::MAX as usize {
        return Err(RdpProtocolError::encoding(format!("tpkt payload too large: {}", total)));
    }
    cursor.patch_u16_be(offset + 2, total as u16)
}
