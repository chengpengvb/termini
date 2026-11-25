use crate::core::stream::ReadCursor;
use crate::core::error::{RdpProtocolError, RdpResult};

const SEGMENTED_SINGLE: u8 = 0xE0;
const SEGMENTED_MULTIPART: u8 = 0xE1;
const PACKET_COMPRESSED: u8 = 0x20;

const HISTORY_BUFFER_SIZE: usize = 2_500_000;
const OUTPUT_BUFFER_SIZE: usize = 65536;
const MATCH_MINIMUM_LENGTH: u32 = 3;
const UNENCODED_LENGTH_BITS: u32 = 15;

struct Token {
    prefix_length: u32,
    prefix_code: u32,
    value_bits: u32,
    token_type: u8,
    value_base: u32,
}

const TOKEN_TABLE: [Token; 40] = [
    Token { prefix_length: 1, prefix_code: 0, value_bits: 8, token_type: 0, value_base: 0 },
    Token { prefix_length: 5, prefix_code: 17, value_bits: 5, token_type: 1, value_base: 0 },
    Token { prefix_length: 5, prefix_code: 18, value_bits: 7, token_type: 1, value_base: 32 },
    Token { prefix_length: 5, prefix_code: 19, value_bits: 9, token_type: 1, value_base: 160 },
    Token { prefix_length: 5, prefix_code: 20, value_bits: 10, token_type: 1, value_base: 672 },
    Token { prefix_length: 5, prefix_code: 21, value_bits: 12, token_type: 1, value_base: 1696 },
    Token { prefix_length: 5, prefix_code: 24, value_bits: 0, token_type: 0, value_base: 0x00 },
    Token { prefix_length: 5, prefix_code: 25, value_bits: 0, token_type: 0, value_base: 0x01 },
    Token { prefix_length: 6, prefix_code: 44, value_bits: 14, token_type: 1, value_base: 5792 },
    Token { prefix_length: 6, prefix_code: 45, value_bits: 15, token_type: 1, value_base: 22176 },
    Token { prefix_length: 6, prefix_code: 52, value_bits: 0, token_type: 0, value_base: 0x02 },
    Token { prefix_length: 6, prefix_code: 53, value_bits: 0, token_type: 0, value_base: 0x03 },
    Token { prefix_length: 6, prefix_code: 54, value_bits: 0, token_type: 0, value_base: 0xFF },
    Token { prefix_length: 7, prefix_code: 92, value_bits: 18, token_type: 1, value_base: 54944 },
    Token { prefix_length: 7, prefix_code: 93, value_bits: 20, token_type: 1, value_base: 317088 },
    Token { prefix_length: 7, prefix_code: 110, value_bits: 0, token_type: 0, value_base: 0x04 },
    Token { prefix_length: 7, prefix_code: 111, value_bits: 0, token_type: 0, value_base: 0x05 },
    Token { prefix_length: 7, prefix_code: 112, value_bits: 0, token_type: 0, value_base: 0x06 },
    Token { prefix_length: 7, prefix_code: 113, value_bits: 0, token_type: 0, value_base: 0x07 },
    Token { prefix_length: 7, prefix_code: 114, value_bits: 0, token_type: 0, value_base: 0x08 },
    Token { prefix_length: 7, prefix_code: 115, value_bits: 0, token_type: 0, value_base: 0x09 },
    Token { prefix_length: 7, prefix_code: 116, value_bits: 0, token_type: 0, value_base: 0x0A },
    Token { prefix_length: 7, prefix_code: 117, value_bits: 0, token_type: 0, value_base: 0x0B },
    Token { prefix_length: 7, prefix_code: 118, value_bits: 0, token_type: 0, value_base: 0x3A },
    Token { prefix_length: 7, prefix_code: 119, value_bits: 0, token_type: 0, value_base: 0x3B },
    Token { prefix_length: 7, prefix_code: 120, value_bits: 0, token_type: 0, value_base: 0x3C },
    Token { prefix_length: 7, prefix_code: 121, value_bits: 0, token_type: 0, value_base: 0x3D },
    Token { prefix_length: 7, prefix_code: 122, value_bits: 0, token_type: 0, value_base: 0x3E },
    Token { prefix_length: 7, prefix_code: 123, value_bits: 0, token_type: 0, value_base: 0x3F },
    Token { prefix_length: 7, prefix_code: 124, value_bits: 0, token_type: 0, value_base: 0x40 },
    Token { prefix_length: 7, prefix_code: 125, value_bits: 0, token_type: 0, value_base: 0x80 },
    Token { prefix_length: 8, prefix_code: 188, value_bits: 20, token_type: 1, value_base: 1365664 },
    Token { prefix_length: 8, prefix_code: 189, value_bits: 21, token_type: 1, value_base: 2414240 },
    Token { prefix_length: 8, prefix_code: 252, value_bits: 0, token_type: 0, value_base: 0x0C },
    Token { prefix_length: 8, prefix_code: 253, value_bits: 0, token_type: 0, value_base: 0x38 },
    Token { prefix_length: 8, prefix_code: 254, value_bits: 0, token_type: 0, value_base: 0x39 },
    Token { prefix_length: 8, prefix_code: 255, value_bits: 0, token_type: 0, value_base: 0x66 },
    Token { prefix_length: 9, prefix_code: 380, value_bits: 22, token_type: 1, value_base: 4511392 },
    Token { prefix_length: 9, prefix_code: 381, value_bits: 23, token_type: 1, value_base: 8705696 },
    Token { prefix_length: 9, prefix_code: 382, value_bits: 24, token_type: 1, value_base: 17094304 },
];

pub struct ZgfxContext {
    history: Vec<u8>,
    history_index: usize,
}

impl Default for ZgfxContext {
    fn default() -> Self {
        Self { history: vec![0; HISTORY_BUFFER_SIZE], history_index: 0 }
    }
}

impl ZgfxContext {
    fn history_write(&mut self, source: &[u8]) {
        let mut source = source;
        if source.len() > HISTORY_BUFFER_SIZE {
            let residue = source.len() - HISTORY_BUFFER_SIZE;
            source = &source[residue..];
            self.history_index = (self.history_index + residue) % HISTORY_BUFFER_SIZE;
        }
        if self.history_index + source.len() <= HISTORY_BUFFER_SIZE {
            self.history[self.history_index..self.history_index + source.len()].copy_from_slice(source);
            self.history_index += source.len();
            if self.history_index == HISTORY_BUFFER_SIZE {
                self.history_index = 0;
            }
            return;
        }
        let front = HISTORY_BUFFER_SIZE - self.history_index;
        self.history[self.history_index..].copy_from_slice(&source[..front]);
        self.history[..source.len() - front].copy_from_slice(&source[front..]);
        self.history_index = source.len() - front;
    }

    fn history_push(&mut self, value: u8) {
        self.history[self.history_index] = value;
        self.history_index += 1;
        if self.history_index == HISTORY_BUFFER_SIZE {
            self.history_index = 0;
        }
    }

    fn history_read(&self, distance: usize, destination: &mut [u8]) -> RdpResult<()> {
        if distance > HISTORY_BUFFER_SIZE {
            return Err(RdpProtocolError::protocol(format!("zgfx match distance exceeds history buffer: {}", distance)));
        }
        let index = (self.history_index + HISTORY_BUFFER_SIZE - distance) % HISTORY_BUFFER_SIZE;
        let first = destination.len().min(distance);
        if index + first <= HISTORY_BUFFER_SIZE {
            destination[..first].copy_from_slice(&self.history[index..index + first]);
        } else {
            let front = HISTORY_BUFFER_SIZE - index;
            destination[..front].copy_from_slice(&self.history[index..]);
            destination[front..first].copy_from_slice(&self.history[..first - front]);
        }
        let mut written = first;
        let mut valid = first;
        while written < destination.len() {
            let length = valid.min(destination.len() - written);
            destination.copy_within(0..length, written);
            written += length;
            valid <<= 1;
        }
        Ok(())
    }

    fn decompress_segment(&mut self, segment: &[u8]) -> RdpResult<Vec<u8>> {
        if segment.len() < 2 {
            return Err(RdpProtocolError::protocol(format!("zgfx segment too short: {}", segment.len())));
        }
        let flags = segment[0];
        let body = &segment[1..];
        if flags & PACKET_COMPRESSED == 0 {
            if body.len() > OUTPUT_BUFFER_SIZE {
                return Err(RdpProtocolError::protocol(format!("zgfx uncompressed segment too large: {}", body.len())));
            }
            self.history_write(body);
            return Ok(body.to_vec());
        }

        let (input, padding) = body.split_at(body.len() - 1);
        let bits = 8 * input.len() as u32;
        let padding = u32::from(padding[0]);
        if bits < padding {
            return Err(RdpProtocolError::protocol(format!("zgfx invalid padding bit count: {} > {}", padding, bits)));
        }
        let mut reader = BitReader { input, position: 0, bits: 0, bits_remaining: bits - padding, bits_current: 0, count_current: 0 };
        let mut output = Vec::new();
        while reader.bits_remaining != 0 {
            let token = reader.read_token()?;
            if token.token_type == 0 {
                reader.read_bits(token.value_bits)?;
                let literal = (token.value_base + reader.bits) as u8;
                self.history_push(literal);
                if output.len() >= OUTPUT_BUFFER_SIZE {
                    return Err(RdpProtocolError::protocol("zgfx decompressed output exceeds segment limit".to_string()));
                }
                output.push(literal);
                continue;
            }
            reader.read_bits(token.value_bits)?;
            let distance = (token.value_base + reader.bits) as usize;
            if distance != 0 {
                let count = reader.read_match_length()? as usize;
                if count > OUTPUT_BUFFER_SIZE - output.len() {
                    return Err(RdpProtocolError::protocol("zgfx match length exceeds segment limit".to_string()));
                }
                let start = output.len();
                output.resize(start + count, 0);
                self.history_read(distance, &mut output[start..])?;
                self.history_write(&output[start..]);
                continue;
            }
            let count = reader.read_unencoded_length()? as usize;
            if count > OUTPUT_BUFFER_SIZE - output.len() {
                return Err(RdpProtocolError::protocol("zgfx literal data exceeds segment limit".to_string()));
            }
            if count > (reader.bits_remaining / 8) as usize {
                return Err(RdpProtocolError::protocol("zgfx literal data exceeds remaining bits".to_string()));
            }
            let raw = reader.read_raw(count)?;
            output.extend_from_slice(raw);
            self.history_write(raw);
        }
        Ok(output)
    }
}

struct BitReader<'a> {
    input: &'a [u8],
    position: usize,
    bits: u32,
    bits_remaining: u32,
    bits_current: u32,
    count_current: u32,
}

impl<'a> BitReader<'a> {
    fn read_bits(&mut self, count: u32) -> RdpResult<()> {
        while self.count_current < count {
            self.bits_current <<= 8;
            if self.position < self.input.len() {
                self.bits_current += u32::from(self.input[self.position]);
                self.position += 1;
            }
            self.count_current += 8;
        }
        self.bits_remaining = self.bits_remaining.checked_sub(count).ok_or_else(|| RdpProtocolError::protocol("zgfx bit stream exhausted".to_string()))?;
        self.count_current -= count;
        self.bits = self.bits_current >> self.count_current;
        self.bits_current &= (1u32 << self.count_current) - 1;
        Ok(())
    }

    fn read_token(&mut self) -> RdpResult<&'static Token> {
        let mut have_bits = 0u32;
        let mut prefix = 0u32;
        for token in TOKEN_TABLE.iter() {
            while have_bits < token.prefix_length {
                self.read_bits(1)?;
                prefix = (prefix << 1) + self.bits;
                have_bits += 1;
            }
            if prefix == token.prefix_code {
                return Ok(token);
            }
        }
        Err(RdpProtocolError::protocol(format!("zgfx unknown prefix code: {}", prefix)))
    }

    fn read_match_length(&mut self) -> RdpResult<u32> {
        self.read_bits(1)?;
        if self.bits == 0 {
            return Ok(MATCH_MINIMUM_LENGTH);
        }
        let mut count = MATCH_MINIMUM_LENGTH + 1;
        let mut extra = 2u32;
        self.read_bits(1)?;
        while self.bits == 1 {
            count *= 2;
            extra += 1;
            if count as usize > OUTPUT_BUFFER_SIZE {
                return Err(RdpProtocolError::protocol(format!("zgfx match length out of range: {}", count)));
            }
            self.read_bits(1)?;
        }
        self.read_bits(extra)?;
        Ok(count + self.bits)
    }

    fn read_unencoded_length(&mut self) -> RdpResult<u32> {
        self.read_bits(UNENCODED_LENGTH_BITS)?;
        let count = self.bits;
        self.bits_remaining = self.bits_remaining.checked_sub(self.count_current).ok_or_else(|| RdpProtocolError::protocol("zgfx bit stream exhausted".to_string()))?;
        self.count_current = 0;
        self.bits_current = 0;
        Ok(count)
    }

    fn read_raw(&mut self, count: usize) -> RdpResult<&'a [u8]> {
        if self.position + count > self.input.len() {
            return Err(RdpProtocolError::protocol(format!("zgfx literal data out of bounds: {}+{} > {}", self.position, count, self.input.len())));
        }
        let raw = &self.input[self.position..self.position + count];
        self.position += count;
        self.bits_remaining = self.bits_remaining.checked_sub(8 * count as u32).ok_or_else(|| RdpProtocolError::protocol("zgfx bit stream exhausted".to_string()))?;
        Ok(raw)
    }
}

pub fn decompress(context: &mut ZgfxContext, source: &[u8]) -> RdpResult<Vec<u8>> {
    let mut cursor = ReadCursor::new(source);
    let descriptor = cursor.read_u8()?;
    match descriptor {
        SEGMENTED_SINGLE => context.decompress_segment(cursor.rest()),
        SEGMENTED_MULTIPART => {
            let segment_count = cursor.read_u16_le()?;
            let uncompressed_size = cursor.read_u32_le()? as usize;
            let mut output = Vec::with_capacity(uncompressed_size);
            for _ in 0..segment_count {
                let length = cursor.read_u32_le()? as usize;
                let segment = cursor.read_slice(length)?;
                output.extend_from_slice(&context.decompress_segment(segment)?);
                if output.len() > uncompressed_size {
                    return Err(RdpProtocolError::protocol(format!("zgfx total segment length exceeds declared size: {} > {}", output.len(), uncompressed_size)));
                }
            }
            if output.len() != uncompressed_size {
                return Err(RdpProtocolError::protocol(format!("zgfx total segment length mismatch: {} != {}", output.len(), uncompressed_size)));
            }
            Ok(output)
        }
        other => Err(RdpProtocolError::protocol(format!("unknown zgfx descriptor: 0x{:02x}", other))),
    }
}
