use crate::core::error::{RdpProtocolError, RdpResult};

const ALPHA_MASK: u32 = 0xFF00_0000;
const COLOR_MASK: u32 = 0x00FF_FFFF;

pub const ROP3_PATCOPY: u8 = 0xF0;

const SOURCE_PAIRS: [(u8, u8); 4] = [(0x01, 0x04), (0x02, 0x08), (0x10, 0x40), (0x20, 0x80)];

pub fn uses_source(rop: u8) -> bool {
    SOURCE_PAIRS.iter().any(|(without, with)| (rop & without != 0) != (rop & with != 0))
}

pub fn rop3(rop: u8, pattern: u32, source: u32, dest: u32) -> u32 {
    let mut result = 0u32;
    if rop & 0x01 != 0 {
        result |= !pattern & !source & !dest;
    }
    if rop & 0x02 != 0 {
        result |= !pattern & !source & dest;
    }
    if rop & 0x04 != 0 {
        result |= !pattern & source & !dest;
    }
    if rop & 0x08 != 0 {
        result |= !pattern & source & dest;
    }
    if rop & 0x10 != 0 {
        result |= pattern & !source & !dest;
    }
    if rop & 0x20 != 0 {
        result |= pattern & !source & dest;
    }
    if rop & 0x40 != 0 {
        result |= pattern & source & !dest;
    }
    if rop & 0x80 != 0 {
        result |= pattern & source & dest;
    }
    opaque(result)
}

pub fn rop2(rop: u8, pen: u32, dest: u32) -> RdpResult<u32> {
    let result = match rop {
        1 => 0,
        2 => !(pen | dest),
        3 => !pen & dest,
        4 => !pen,
        5 => pen & !dest,
        6 => !dest,
        7 => pen ^ dest,
        8 => !(pen & dest),
        9 => pen & dest,
        10 => !(pen ^ dest),
        11 => dest,
        12 => !pen | dest,
        13 => pen,
        14 => pen | !dest,
        15 => pen | dest,
        16 => u32::MAX,
        other => return Err(RdpProtocolError::protocol(format!("unknown binary raster operation: {}", other))),
    };
    Ok(opaque(result))
}

fn opaque(value: u32) -> u32 {
    (value & COLOR_MASK) | ALPHA_MASK
}
