use std::borrow::Cow;
use crate::codec::bulk;
use crate::codec::mppc;
use crate::codec::mppc::MppcContext;
use crate::core::error::{RdpProtocolError, RdpResult};

const HISTORY_BUFFER_SIZE: usize = 2_000_000;

const L1_COMPRESSED: u8 = 0x01;
const L1_NO_COMPRESSION: u8 = 0x02;
const L1_PACKET_AT_FRONT: u8 = 0x04;

const LEVEL_FLAGS_LENGTH: usize = 2;
const MATCH_DETAILS_LENGTH: usize = 8;

pub struct XcrushContext {
    history: Vec<u8>,
    offset: usize,
    mppc: MppcContext,
}

impl Default for XcrushContext {
    fn default() -> Self {
        Self { history: vec![0u8; HISTORY_BUFFER_SIZE], offset: 0, mppc: MppcContext::default() }
    }
}

pub fn decompress(context: &mut XcrushContext, source: &[u8], flags: u8) -> RdpResult<Vec<u8>> {
    if source.len() < LEVEL_FLAGS_LENGTH {
        return Err(RdpProtocolError::underrun(format!("xcrush packet too short: {}", source.len())));
    }
    let level1 = source[0];
    let level2 = source[1];
    let body = &source[LEVEL_FLAGS_LENGTH..];

    if flags & bulk::PACKET_FLUSHED != 0 {
        context.history.fill(0);
        context.offset = 0;
    }

    let inner = match level2 & bulk::PACKET_COMPRESSED != 0 {
        true => mppc::decompress(&mut context.mppc, body, level2, mppc::LEVEL_64K)?,
        false => Cow::Borrowed(body),
    };
    decompress_level1(context, &inner, level1)
}

fn decompress_level1(context: &mut XcrushContext, source: &[u8], flags: u8) -> RdpResult<Vec<u8>> {
    if source.is_empty() {
        return Err(RdpProtocolError::underrun("xcrush level-1 data is empty".to_string()));
    }
    if flags & L1_PACKET_AT_FRONT != 0 {
        context.offset = 0;
    }
    let start = context.offset;
    let mut literals = source;

    if flags & L1_NO_COMPRESSION == 0 {
        if flags & L1_COMPRESSED == 0 {
            return Err(RdpProtocolError::protocol(format!("xcrush unexpected level-1 compression flags: 0x{:02x}", flags)));
        }
        if source.len() < 2 {
            return Err(RdpProtocolError::underrun(format!("xcrush match count field underrun: {}", source.len())));
        }
        let count = usize::from(u16::from_le_bytes([source[0], source[1]]));
        let details_end = 2 + count * MATCH_DETAILS_LENGTH;
        if details_end > source.len() {
            return Err(RdpProtocolError::underrun(format!("xcrush match details underrun: need {} have {}", details_end, source.len())));
        }
        literals = &source[details_end..];
        let mut literal_offset = 0usize;
        let mut output_offset = 0usize;

        for index in 0..count {
            let entry = &source[2 + index * MATCH_DETAILS_LENGTH..2 + (index + 1) * MATCH_DETAILS_LENGTH];
            let match_length = usize::from(u16::from_le_bytes([entry[0], entry[1]]));
            let match_output_offset = usize::from(u16::from_le_bytes([entry[2], entry[3]]));
            let match_history_offset = u32::from_le_bytes([entry[4], entry[5], entry[6], entry[7]]) as usize;
            if match_output_offset < output_offset {
                return Err(RdpProtocolError::protocol(format!("xcrush match output offset moved backwards: {} < {}", match_output_offset, output_offset)));
            }
            if match_length > HISTORY_BUFFER_SIZE || match_history_offset > HISTORY_BUFFER_SIZE {
                return Err(RdpProtocolError::protocol(format!("xcrush match out of bounds: length={} history={}", match_length, match_history_offset)));
            }
            let output_length = match_output_offset - output_offset;
            if output_length > 0 {
                if context.offset + output_length >= HISTORY_BUFFER_SIZE || literal_offset + output_length > literals.len() {
                    return Err(RdpProtocolError::protocol(format!("xcrush literal out of bounds: offset={} length={}", context.offset, output_length)));
                }
                context.history[context.offset..context.offset + output_length].copy_from_slice(&literals[literal_offset..literal_offset + output_length]);
                context.offset += output_length;
                literal_offset += output_length;
                output_offset += output_length;
            }
            if context.offset + match_length >= HISTORY_BUFFER_SIZE || match_history_offset + match_length >= HISTORY_BUFFER_SIZE {
                return Err(RdpProtocolError::protocol(format!("xcrush match copy out of bounds: offset={} history={} length={}", context.offset, match_history_offset, match_length)));
            }
            for step in 0..match_length {
                context.history[context.offset + step] = context.history[match_history_offset + step];
            }
            context.offset += match_length;
            output_offset += match_length;
        }
        literals = &literals[literal_offset..];
    }

    if !literals.is_empty() {
        if context.offset + literals.len() >= HISTORY_BUFFER_SIZE {
            return Err(RdpProtocolError::protocol(format!("xcrush trailing literal out of bounds: offset={} length={}", context.offset, literals.len())));
        }
        context.history[context.offset..context.offset + literals.len()].copy_from_slice(literals);
        context.offset += literals.len();
    }
    Ok(context.history[start..context.offset].to_vec())
}
