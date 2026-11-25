use std::borrow::Cow;
use crate::codec::bulk;
use crate::codec::bitstream::BitStream;
use crate::core::error::{RdpProtocolError, RdpResult};

const HISTORY_BUFFER_SIZE: usize = 65536;
const HISTORY_BUFFER_SIZE_8K: usize = 8192;
const HISTORY_MASK_64K: usize = 0xFFFF;
const HISTORY_MASK_8K: usize = 0x1FFF;

pub const LEVEL_8K: u32 = 0;
pub const LEVEL_64K: u32 = 1;

pub struct MppcContext {
    history: Vec<u8>,
    offset: usize,
    size: usize,
    level: u32,
}

impl Default for MppcContext {
    fn default() -> Self {
        Self { history: vec![0u8; HISTORY_BUFFER_SIZE], offset: 0, size: HISTORY_BUFFER_SIZE, level: LEVEL_64K }
    }
}

impl MppcContext {
    fn set_level(&mut self, level: u32) {
        match level {
            LEVEL_8K => {
                self.level = LEVEL_8K;
                self.size = HISTORY_BUFFER_SIZE_8K;
            }
            _ => {
                self.level = LEVEL_64K;
                self.size = HISTORY_BUFFER_SIZE;
            }
        }
    }

    fn mask(&self) -> usize {
        match self.level {
            LEVEL_8K => HISTORY_MASK_8K,
            _ => HISTORY_MASK_64K,
        }
    }
}

pub fn decompress<'a>(context: &mut MppcContext, source: &'a [u8], flags: u8, level: u32) -> RdpResult<Cow<'a, [u8]>> {
    context.set_level(level);

    if flags & bulk::PACKET_AT_FRONT != 0 {
        context.offset = 0;
    }

    if flags & bulk::PACKET_FLUSHED != 0 {
        context.offset = 0;
        context.history.fill(0);
    }

    if flags & bulk::PACKET_COMPRESSED == 0 {
        return Ok(Cow::Borrowed(source));
    }

    let start = context.offset;
    let mut stream = BitStream::new(source);
    while stream.remaining() >= 8 {
        if context.offset >= context.size {
            return Err(RdpProtocolError::protocol(format!("mppc history buffer out of bounds: {} >= {}", context.offset, context.size)));
        }

        let accumulator = stream.accumulator();
        if accumulator & 0x8000_0000 == 0 {
            context.history[context.offset] = ((accumulator & 0x7F00_0000) >> 24) as u8;
            context.offset += 1;
            stream.shift(8);
            continue;
        }
        if accumulator & 0xC000_0000 == 0x8000_0000 {
            context.history[context.offset] = (((accumulator & 0x3F80_0000) >> 23) + 0x80) as u8;
            context.offset += 1;
            stream.shift(9);
            continue;
        }

        let copy_offset = read_copy_offset(&mut stream, accumulator, context.level)?;
        let length = read_length_of_match(&mut stream, context.level)?;

        if context.offset + length > context.size {
            return Err(RdpProtocolError::protocol(format!("mppc match write out of bounds: {}+{} > {}", context.offset, length, context.size)));
        }

        let origin = context.offset.wrapping_sub(copy_offset) & context.mask();
        if origin + length > context.size - 1 {
            return Err(RdpProtocolError::protocol(format!("mppc match read out of bounds: distance {} length {}", copy_offset, length)));
        }

        for step in 0..length {
            let value = context.history[origin + step];
            context.history[context.offset + step] = value;
        }
        context.offset += length;
    }

    Ok(Cow::Owned(context.history[start..context.offset].to_vec()))
}

fn read_copy_offset(stream: &mut BitStream<'_>, accumulator: u32, level: u32) -> RdpResult<usize> {
    if level != LEVEL_8K {
        if accumulator & 0xF800_0000 == 0xF800_0000 {
            stream.shift(11);
            return Ok(((accumulator >> 21) & 0x3F) as usize);
        }
        if accumulator & 0xF800_0000 == 0xF000_0000 {
            stream.shift(13);
            return Ok((((accumulator >> 19) & 0xFF) + 64) as usize);
        }
        if accumulator & 0xF000_0000 == 0xE000_0000 {
            stream.shift(15);
            return Ok((((accumulator >> 17) & 0x7FF) + 320) as usize);
        }
        if accumulator & 0xE000_0000 == 0xC000_0000 {
            stream.shift(19);
            return Ok((((accumulator >> 13) & 0xFFFF) + 2368) as usize);
        }
        return Err(RdpProtocolError::protocol("mppc invalid CopyOffset encoding (RDP5)".to_string()));
    }

    if accumulator & 0xF000_0000 == 0xF000_0000 {
        stream.shift(10);
        return Ok(((accumulator >> 22) & 0x3F) as usize);
    }
    if accumulator & 0xF000_0000 == 0xE000_0000 {
        stream.shift(12);
        return Ok((((accumulator >> 20) & 0xFF) + 64) as usize);
    }
    if accumulator & 0xE000_0000 == 0xC000_0000 {
        stream.shift(16);
        return Ok((((accumulator >> 16) & 0x1FFF) + 320) as usize);
    }
    Err(RdpProtocolError::protocol("mppc invalid CopyOffset encoding (RDP4)".to_string()))
}

fn read_length_of_match(stream: &mut BitStream<'_>, level: u32) -> RdpResult<usize> {
    let accumulator = stream.accumulator();
    if accumulator & 0x8000_0000 == 0 {
        stream.shift(1);
        return Ok(3);
    }
    if accumulator & 0xC000_0000 == 0x8000_0000 {
        stream.shift(4);
        return Ok((((accumulator >> 28) & 0x0003) + 0x0004) as usize);
    }
    if accumulator & 0xE000_0000 == 0xC000_0000 {
        stream.shift(6);
        return Ok((((accumulator >> 26) & 0x0007) + 0x0008) as usize);
    }
    if accumulator & 0xF000_0000 == 0xE000_0000 {
        stream.shift(8);
        return Ok((((accumulator >> 24) & 0x000F) + 0x0010) as usize);
    }
    if accumulator & 0xF800_0000 == 0xF000_0000 {
        stream.shift(10);
        return Ok((((accumulator >> 22) & 0x001F) + 0x0020) as usize);
    }
    if accumulator & 0xFC00_0000 == 0xF800_0000 {
        stream.shift(12);
        return Ok((((accumulator >> 20) & 0x003F) + 0x0040) as usize);
    }
    if accumulator & 0xFE00_0000 == 0xFC00_0000 {
        stream.shift(14);
        return Ok((((accumulator >> 18) & 0x007F) + 0x0080) as usize);
    }
    if accumulator & 0xFF00_0000 == 0xFE00_0000 {
        stream.shift(16);
        return Ok((((accumulator >> 16) & 0x00FF) + 0x0100) as usize);
    }
    if accumulator & 0xFF80_0000 == 0xFF00_0000 {
        stream.shift(18);
        return Ok((((accumulator >> 14) & 0x01FF) + 0x0200) as usize);
    }
    if accumulator & 0xFFC0_0000 == 0xFF80_0000 {
        stream.shift(20);
        return Ok((((accumulator >> 12) & 0x03FF) + 0x0400) as usize);
    }
    if accumulator & 0xFFE0_0000 == 0xFFC0_0000 {
        stream.shift(22);
        return Ok((((accumulator >> 10) & 0x07FF) + 0x0800) as usize);
    }
    if accumulator & 0xFFF0_0000 == 0xFFE0_0000 {
        stream.shift(24);
        return Ok((((accumulator >> 8) & 0x0FFF) + 0x1000) as usize);
    }
    if level != LEVEL_8K {
        if accumulator & 0xFFF8_0000 == 0xFFF0_0000 {
            stream.shift(26);
            return Ok((((accumulator >> 6) & 0x1FFF) + 0x2000) as usize);
        }
        if accumulator & 0xFFFC_0000 == 0xFFF8_0000 {
            stream.shift(28);
            return Ok((((accumulator >> 4) & 0x3FFF) + 0x4000) as usize);
        }
        if accumulator & 0xFFFE_0000 == 0xFFFC_0000 {
            stream.shift(30);
            return Ok((((accumulator >> 2) & 0x7FFF) + 0x8000) as usize);
        }
    }
    Err(RdpProtocolError::protocol("mppc invalid LengthOfMatch encoding".to_string()))
}
