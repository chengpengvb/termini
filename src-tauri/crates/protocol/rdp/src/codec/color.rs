use crate::core::error::{RdpProtocolError, RdpResult};

pub const PALETTE_MAX_ENTRIES: usize = 256;

const YCBCR_DIVISOR: i64 = 16;
const YCBCR_COEFFICIENT_SHIFT: i64 = 5;
const YCBCR_LUMA_OFFSET: i64 = 4096;
const YCBCR_CR_RED: i64 = 91916;
const YCBCR_CR_GREEN: i64 = 46819;
const YCBCR_CB_GREEN: i64 = 22527;
const YCBCR_CB_BLUE: i64 = 115992;

pub struct Palette {
    entries: [[u8; 4]; PALETTE_MAX_ENTRIES],
    pub(crate) loaded: bool,
}

impl Default for Palette {
    fn default() -> Self {
        Self { entries: [[0, 0, 0, 0xFF]; PALETTE_MAX_ENTRIES], loaded: false }
    }
}

impl Palette {
    pub fn load(&mut self, entries: &[[u8; 3]]) -> RdpResult<()> {
        if entries.len() > PALETTE_MAX_ENTRIES {
            return Err(RdpProtocolError::protocol(format!("too many palette entries: {}", entries.len())));
        }
        for (index, entry) in entries.iter().enumerate() {
            self.entries[index] = [entry[0], entry[1], entry[2], 0xFF];
        }
        self.loaded = true;
        Ok(())
    }

    pub(crate) fn lookup(&self, index: u8) -> [u8; 4] {
        self.entries[index as usize]
    }
}

pub fn bytes_per_pixel(bpp: u16) -> RdpResult<usize> {
    match bpp {
        8 => Ok(1),
        15 | 16 => Ok(2),
        24 => Ok(3),
        32 => Ok(4),
        _ => Err(RdpProtocolError::unsupported(format!("unsupported bitmap color depth: {}", bpp))),
    }
}

pub fn white_pixel(bpp: u16) -> RdpResult<u32> {
    match bpp {
        8 => Ok(0xFF),
        15 => Ok(0x7FFF),
        16 => Ok(0xFFFF),
        24 => Ok(0x00FF_FFFF),
        _ => Err(RdpProtocolError::unsupported(format!("unsupported RLE color depth: {}", bpp))),
    }
}

pub fn decode_order_color(value: u32, color_depth: u16, palette: &Palette) -> RdpResult<u32> {
    let pixel = match color_depth {
        8 => {
            if !palette.loaded {
                return Err(RdpProtocolError::protocol("received 8bpp drawing color but the server has not sent a palette yet".to_string()));
            }
            palette.lookup(value as u8)
        }
        15 => convert_rgb555(value as u16),
        16 => convert_rgb565(value as u16),
        24 | 32 => [value as u8, (value >> 8) as u8, (value >> 16) as u8, 0xFF],
        other => return Err(RdpProtocolError::unsupported(format!("unsupported drawing color depth: {}", other))),
    };
    Ok(u32::from_le_bytes(pixel))
}

pub fn to_rgba(source: &[u8], width: usize, height: usize, bpp: u16, palette: &Palette) -> RdpResult<Vec<u8>> {
    let pixel_size = bytes_per_pixel(bpp)?;
    let expected = width * height * pixel_size;
    if source.len() < expected {
        return Err(RdpProtocolError::underrun(format!("bitmap data underrun: need {} have {}", expected, source.len())));
    }
    if bpp == 8 && !palette.loaded {
        return Err(RdpProtocolError::protocol("received 8bpp bitmap but the server has not sent a palette yet".to_string()));
    }
    let mut output = vec![0u8; width * height * 4];
    for index in 0..width * height {
        let offset = index * pixel_size;
        let target = index * 4;
        let pixel = match bpp {
            8 => palette.lookup(source[offset]),
            15 => convert_rgb555(u16::from_le_bytes([source[offset], source[offset + 1]])),
            16 => convert_rgb565(u16::from_le_bytes([source[offset], source[offset + 1]])),
            24 => [source[offset + 2], source[offset + 1], source[offset], 0xFF],
            32 => [source[offset + 2], source[offset + 1], source[offset], 0xFF],
            _ => return Err(RdpProtocolError::unsupported(format!("unsupported bitmap color depth: {}", bpp))),
        };
        output[target..target + 4].copy_from_slice(&pixel);
    }
    Ok(output)
}

pub fn ycbcr_to_rgba(luma: i16, blue_chroma: i16, red_chroma: i16) -> [u8; 4] {
    let base = (i64::from(luma) + YCBCR_LUMA_OFFSET) << YCBCR_DIVISOR;
    let red = i64::from(red_chroma) * YCBCR_CR_RED;
    let green = i64::from(red_chroma) * YCBCR_CR_GREEN + i64::from(blue_chroma) * YCBCR_CB_GREEN;
    let blue = i64::from(blue_chroma) * YCBCR_CB_BLUE;
    [clamp_channel(base + red), clamp_channel(base - green), clamp_channel(base + blue), 0xFF]
}

pub fn rgb_to_yuv(red: u8, green: u8, blue: u8, bt709: bool) -> (u8, u8, u8) {
    let r = i32::from(red);
    let g = i32::from(green);
    let b = i32::from(blue);
    let (y, u, v) = if bt709 {
        (((47 * r + 157 * g + 16 * b + 128) >> 8) + 16, ((-26 * r - 86 * g + 112 * b + 128) >> 8) + 128, ((112 * r - 102 * g - 10 * b + 128) >> 8) + 128)
    } else {
        (((66 * r + 129 * g + 25 * b + 128) >> 8) + 16, ((-38 * r - 74 * g + 112 * b + 128) >> 8) + 128, ((112 * r - 94 * g - 18 * b + 128) >> 8) + 128)
    };
    (clamp_i32_u8(y), clamp_i32_u8(u), clamp_i32_u8(v))
}

fn clamp_channel(value: i64) -> u8 {
    ((value >> YCBCR_DIVISOR) >> YCBCR_COEFFICIENT_SHIFT).clamp(0, 0xFF) as u8
}

fn clamp_i32_u8(value: i32) -> u8 {
    value.clamp(0, 255) as u8
}

fn convert_rgb555(value: u16) -> [u8; 4] {
    let red = ((value >> 10) & 0x1F) as u8;
    let green = ((value >> 5) & 0x1F) as u8;
    let blue = (value & 0x1F) as u8;
    [expand5(red), expand5(green), expand5(blue), 0xFF]
}

fn convert_rgb565(value: u16) -> [u8; 4] {
    let red = ((value >> 11) & 0x1F) as u8;
    let green = ((value >> 5) & 0x3F) as u8;
    let blue = (value & 0x1F) as u8;
    [expand5(red), expand6(green), expand5(blue), 0xFF]
}

fn expand5(value: u8) -> u8 {
    (value << 3) | (value >> 2)
}

fn expand6(value: u8) -> u8 {
    (value << 2) | (value >> 4)
}
