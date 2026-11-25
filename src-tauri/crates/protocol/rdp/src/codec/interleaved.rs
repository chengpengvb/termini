use crate::codec::color;
use crate::core::error::{RdpProtocolError, RdpResult};

const REGULAR_BG_RUN: u32 = 0x00;
const REGULAR_FG_RUN: u32 = 0x01;
const REGULAR_FGBG_IMAGE: u32 = 0x02;
const REGULAR_COLOR_RUN: u32 = 0x03;
const REGULAR_COLOR_IMAGE: u32 = 0x04;
const LITE_SET_FG_FG_RUN: u32 = 0x0C;
const LITE_SET_FG_FGBG_IMAGE: u32 = 0x0D;
const LITE_DITHERED_RUN: u32 = 0x0E;
const MEGA_MEGA_BG_RUN: u32 = 0xF0;
const MEGA_MEGA_FG_RUN: u32 = 0xF1;
const MEGA_MEGA_FGBG_IMAGE: u32 = 0xF2;
const MEGA_MEGA_COLOR_RUN: u32 = 0xF3;
const MEGA_MEGA_COLOR_IMAGE: u32 = 0xF4;
const MEGA_MEGA_SET_FG_RUN: u32 = 0xF6;
const MEGA_MEGA_SET_FGBG_IMAGE: u32 = 0xF7;
const MEGA_MEGA_DITHERED_RUN: u32 = 0xF8;
const SPECIAL_FGBG_1: u32 = 0xF9;
const SPECIAL_FGBG_2: u32 = 0xFA;
const SPECIAL_WHITE: u32 = 0xFD;
const SPECIAL_BLACK: u32 = 0xFE;

const MASK_REGULAR_RUN_LENGTH: u8 = 0x1F;
const MASK_LITE_RUN_LENGTH: u8 = 0x0F;
const MASK_SPECIAL_FGBG_1: u8 = 0x03;
const MASK_SPECIAL_FGBG_2: u8 = 0x05;

pub fn decompress(source: &[u8], width: usize, height: usize, bpp: u16) -> RdpResult<Vec<u8>> {
    let pixel_size = color::bytes_per_pixel(bpp)?;
    let white = color::white_pixel(bpp)?;
    let row_delta = width * pixel_size;
    if row_delta == 0 || height == 0 {
        return Err(RdpProtocolError::protocol("RLE bitmap size is zero".to_string()));
    }

    let mut context = RleContext { output: vec![0u8; row_delta * height], position: 0, pixel_size, row_delta };
    let mut cursor = SourceCursor { data: source, position: 0 };
    let mut foreground = white;
    let mut insert_foreground = false;
    let mut first_line = true;

    while cursor.remaining() > 0 {
        if first_line && context.position >= row_delta {
            first_line = false;
            insert_foreground = false;
        }

        let code = extract_code_id(cursor.peek()?);

        if code == REGULAR_BG_RUN || code == MEGA_MEGA_BG_RUN {
            let mut run_length = extract_run_length(code, &mut cursor)?;
            match first_line {
                true => {
                    if insert_foreground {
                        context.write_pixel(foreground)?;
                        run_length = run_length.saturating_sub(1);
                    }
                    for _ in 0..run_length {
                        context.write_pixel(0)?;
                    }
                }
                false => {
                    if insert_foreground {
                        let above = context.read_above()?;
                        context.write_pixel(above ^ foreground)?;
                        run_length = run_length.saturating_sub(1);
                    }
                    for _ in 0..run_length {
                        let above = context.read_above()?;
                        context.write_pixel(above)?;
                    }
                }
            }
            insert_foreground = true;
            continue;
        }

        insert_foreground = false;

        match code {
            REGULAR_FG_RUN | MEGA_MEGA_FG_RUN | LITE_SET_FG_FG_RUN | MEGA_MEGA_SET_FG_RUN => {
                let run_length = extract_run_length(code, &mut cursor)?;
                if code == LITE_SET_FG_FG_RUN || code == MEGA_MEGA_SET_FG_RUN {
                    foreground = cursor.read_pixel(pixel_size)?;
                }
                for _ in 0..run_length {
                    match first_line {
                        true => context.write_pixel(foreground)?,
                        false => {
                            let above = context.read_above()?;
                            context.write_pixel(above ^ foreground)?;
                        }
                    }
                }
            }
            LITE_DITHERED_RUN | MEGA_MEGA_DITHERED_RUN => {
                let run_length = extract_run_length(code, &mut cursor)?;
                let pixel_a = cursor.read_pixel(pixel_size)?;
                let pixel_b = cursor.read_pixel(pixel_size)?;
                for _ in 0..run_length {
                    context.write_pixel(pixel_a)?;
                    context.write_pixel(pixel_b)?;
                }
            }
            REGULAR_COLOR_RUN | MEGA_MEGA_COLOR_RUN => {
                let run_length = extract_run_length(code, &mut cursor)?;
                let pixel = cursor.read_pixel(pixel_size)?;
                for _ in 0..run_length {
                    context.write_pixel(pixel)?;
                }
            }
            REGULAR_FGBG_IMAGE | MEGA_MEGA_FGBG_IMAGE | LITE_SET_FG_FGBG_IMAGE | MEGA_MEGA_SET_FGBG_IMAGE => {
                let mut run_length = extract_run_length(code, &mut cursor)?;
                if code == LITE_SET_FG_FGBG_IMAGE || code == MEGA_MEGA_SET_FGBG_IMAGE {
                    foreground = cursor.read_pixel(pixel_size)?;
                }
                while run_length > 8 {
                    let bitmask = cursor.read_u8()?;
                    context.write_fgbg(bitmask, foreground, 8, first_line)?;
                    run_length -= 8;
                }
                if run_length > 0 {
                    let bitmask = cursor.read_u8()?;
                    context.write_fgbg(bitmask, foreground, run_length, first_line)?;
                }
            }
            REGULAR_COLOR_IMAGE | MEGA_MEGA_COLOR_IMAGE => {
                let run_length = extract_run_length(code, &mut cursor)?;
                for _ in 0..run_length {
                    let pixel = cursor.read_pixel(pixel_size)?;
                    context.write_pixel(pixel)?;
                }
            }
            SPECIAL_FGBG_1 => {
                cursor.skip(1)?;
                context.write_fgbg(MASK_SPECIAL_FGBG_1, foreground, 8, first_line)?;
            }
            SPECIAL_FGBG_2 => {
                cursor.skip(1)?;
                context.write_fgbg(MASK_SPECIAL_FGBG_2, foreground, 8, first_line)?;
            }
            SPECIAL_WHITE => {
                cursor.skip(1)?;
                context.write_pixel(white)?;
            }
            SPECIAL_BLACK => {
                cursor.skip(1)?;
                context.write_pixel(0)?;
            }
            _ => return Err(RdpProtocolError::protocol(format!("invalid RLE code: 0x{:02x}", code))),
        }
    }

    Ok(context.output)
}

struct RleContext {
    output: Vec<u8>,
    position: usize,
    pixel_size: usize,
    row_delta: usize,
}

impl RleContext {
    fn write_pixel(&mut self, value: u32) -> RdpResult<()> {
        if self.position + self.pixel_size > self.output.len() {
            return Err(RdpProtocolError::protocol(format!("RLE output out of bounds: {} bytes", self.output.len())));
        }
        let bytes = value.to_le_bytes();
        self.output[self.position..self.position + self.pixel_size].copy_from_slice(&bytes[..self.pixel_size]);
        self.position += self.pixel_size;
        Ok(())
    }

    fn read_above(&self) -> RdpResult<u32> {
        if self.position < self.row_delta {
            return Err(RdpProtocolError::protocol("RLE references a previous scanline that does not exist".to_string()));
        }
        let offset = self.position - self.row_delta;
        let mut bytes = [0u8; 4];
        bytes[..self.pixel_size].copy_from_slice(&self.output[offset..offset + self.pixel_size]);
        Ok(u32::from_le_bytes(bytes))
    }

    fn write_fgbg(&mut self, bitmask: u8, foreground: u32, count: u32, first_line: bool) -> RdpResult<()> {
        if count > 8 {
            return Err(RdpProtocolError::protocol(format!("RLE fgbg bit count mismatch: {}", count)));
        }
        let mut mask = 0x01u8;
        for _ in 0..count {
            let value = match first_line {
                true => match bitmask & mask != 0 {
                    true => foreground,
                    false => 0,
                },
                false => {
                    let above = self.read_above()?;
                    match bitmask & mask != 0 {
                        true => above ^ foreground,
                        false => above,
                    }
                }
            };
            self.write_pixel(value)?;
            mask = mask.wrapping_shl(1);
        }
        Ok(())
    }
}

struct SourceCursor<'a> {
    data: &'a [u8],
    position: usize,
}

impl<'a> SourceCursor<'a> {
    fn remaining(&self) -> usize {
        self.data.len() - self.position
    }

    fn peek(&self) -> RdpResult<u8> {
        self.data.get(self.position).copied().ok_or_else(|| RdpProtocolError::underrun("RLE source data exhausted".to_string()))
    }

    fn peek_at(&self, offset: usize) -> RdpResult<u8> {
        self.data.get(self.position + offset).copied().ok_or_else(|| RdpProtocolError::underrun("RLE source data exhausted".to_string()))
    }

    fn read_u8(&mut self) -> RdpResult<u8> {
        let value = self.peek()?;
        self.position += 1;
        Ok(value)
    }

    fn skip(&mut self, count: usize) -> RdpResult<()> {
        if self.remaining() < count {
            return Err(RdpProtocolError::underrun("RLE source data exhausted".to_string()));
        }
        self.position += count;
        Ok(())
    }

    fn read_pixel(&mut self, pixel_size: usize) -> RdpResult<u32> {
        if self.remaining() < pixel_size {
            return Err(RdpProtocolError::underrun("RLE source data exhausted".to_string()));
        }
        let mut bytes = [0u8; 4];
        bytes[..pixel_size].copy_from_slice(&self.data[self.position..self.position + pixel_size]);
        self.position += pixel_size;
        Ok(u32::from_le_bytes(bytes))
    }
}

fn extract_code_id(header: u8) -> u32 {
    if header & 0xC0 != 0xC0 {
        return (header >> 5) as u32;
    }
    if header & 0xF0 == 0xF0 {
        return header as u32;
    }
    (header >> 4) as u32
}

fn extract_run_length(code: u32, cursor: &mut SourceCursor<'_>) -> RdpResult<u32> {
    let header = cursor.peek()?;
    let (run_length, advance) = match code {
        REGULAR_FGBG_IMAGE => match header & MASK_REGULAR_RUN_LENGTH {
            0 => (cursor.peek_at(1)? as u32 + 1, 2),
            value => (value as u32 * 8, 1),
        },
        LITE_SET_FG_FGBG_IMAGE => match header & MASK_LITE_RUN_LENGTH {
            0 => (cursor.peek_at(1)? as u32 + 1, 2),
            value => (value as u32 * 8, 1),
        },
        REGULAR_BG_RUN | REGULAR_FG_RUN | REGULAR_COLOR_RUN | REGULAR_COLOR_IMAGE => match header & MASK_REGULAR_RUN_LENGTH {
            0 => (cursor.peek_at(1)? as u32 + 32, 2),
            value => (value as u32, 1),
        },
        LITE_SET_FG_FG_RUN | LITE_DITHERED_RUN => match header & MASK_LITE_RUN_LENGTH {
            0 => (cursor.peek_at(1)? as u32 + 16, 2),
            value => (value as u32, 1),
        },
        MEGA_MEGA_BG_RUN | MEGA_MEGA_FG_RUN | MEGA_MEGA_SET_FG_RUN | MEGA_MEGA_DITHERED_RUN | MEGA_MEGA_COLOR_RUN | MEGA_MEGA_FGBG_IMAGE | MEGA_MEGA_SET_FGBG_IMAGE | MEGA_MEGA_COLOR_IMAGE => {
            let low = cursor.peek_at(1)? as u32;
            let high = cursor.peek_at(2)? as u32;
            (low | (high << 8), 3)
        }
        _ => return Err(RdpProtocolError::protocol(format!("invalid RLE code: 0x{:02x}", code))),
    };
    cursor.skip(advance)?;
    Ok(run_length)
}
