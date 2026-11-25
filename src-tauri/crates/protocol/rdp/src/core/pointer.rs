use bytes::Bytes;
use std::collections::HashMap;
use crate::codec::color::Palette;
use crate::core::stream::{ReadCursor, WriteCursor};
use crate::core::error::{RdpProtocolError, RdpResult};

pub const CURSOR_KIND_HIDDEN: u8 = 0;
pub const CURSOR_KIND_DEFAULT: u8 = 1;
pub const CURSOR_KIND_IMAGE: u8 = 2;

const MAX_CURSOR_EDGE: u16 = 384;
const POINTER_CACHE_SIZE: u16 = 128;

pub struct CursorImage {
    pub hotspot_x: u16,
    pub hotspot_y: u16,
    pub width: u16,
    pub height: u16,
    pub rgba: Vec<u8>,
}

#[derive(Default)]
pub struct PointerCache {
    entries: HashMap<u16, Bytes>,
}

impl PointerCache {
    pub fn store(&mut self, index: u16, payload: Bytes) -> RdpResult<()> {
        if index >= POINTER_CACHE_SIZE {
            return Err(RdpProtocolError::protocol(format!("pointer cache index out of range: index={} limit={}", index, POINTER_CACHE_SIZE)));
        }
        self.entries.insert(index, payload);
        Ok(())
    }

    pub fn get(&self, index: u16) -> RdpResult<Bytes> {
        self.entries.get(&index).cloned().ok_or_else(|| RdpProtocolError::protocol(format!("pointer cache miss: index={}", index)))
    }
}

pub fn encode_hidden() -> Bytes {
    Bytes::from(vec![CURSOR_KIND_HIDDEN])
}

pub fn encode_default() -> Bytes {
    Bytes::from(vec![CURSOR_KIND_DEFAULT])
}

pub fn encode_image(image: &CursorImage) -> Bytes {
    let mut cursor = WriteCursor::with_capacity(image.rgba.len() + 9);
    cursor.write_u8(CURSOR_KIND_IMAGE);
    cursor.write_u16_le(image.hotspot_x);
    cursor.write_u16_le(image.hotspot_y);
    cursor.write_u16_le(image.width);
    cursor.write_u16_le(image.height);
    cursor.write_slice(&image.rgba);
    Bytes::from(cursor.into_vec())
}

pub fn parse_color_pointer(cursor: &mut ReadCursor<'_>, palette: &Palette) -> RdpResult<(u16, CursorImage)> {
    parse_pointer_attribute(cursor, 24, palette)
}

pub fn parse_new_pointer(cursor: &mut ReadCursor<'_>, palette: &Palette) -> RdpResult<(u16, CursorImage)> {
    let xor_bpp = cursor.read_u16_le()?;
    parse_pointer_attribute(cursor, xor_bpp, palette)
}

pub fn parse_large_pointer(cursor: &mut ReadCursor<'_>, palette: &Palette) -> RdpResult<(u16, CursorImage)> {
    let xor_bpp = cursor.read_u16_le()?;
    let cache_index = cursor.read_u16_le()?;
    let hotspot_x = cursor.read_u16_le()?;
    let hotspot_y = cursor.read_u16_le()?;
    let width = cursor.read_u16_le()?;
    let height = cursor.read_u16_le()?;
    let and_length = cursor.read_u32_le()? as usize;
    let xor_length = cursor.read_u32_le()? as usize;
    let xor_mask = cursor.read_slice(xor_length)?;
    let and_mask = cursor.read_slice(and_length)?;
    let image = build_cursor_image(hotspot_x, hotspot_y, width, height, xor_bpp, xor_mask, and_mask, palette)?;
    Ok((cache_index, image))
}

pub fn parse_cached_pointer(cursor: &mut ReadCursor<'_>) -> RdpResult<u16> {
    cursor.read_u16_le()
}

fn parse_pointer_attribute(cursor: &mut ReadCursor<'_>, xor_bpp: u16, palette: &Palette) -> RdpResult<(u16, CursorImage)> {
    let cache_index = cursor.read_u16_le()?;
    let hotspot_x = cursor.read_u16_le()?;
    let hotspot_y = cursor.read_u16_le()?;
    let width = cursor.read_u16_le()?;
    let height = cursor.read_u16_le()?;
    let and_length = cursor.read_u16_le()? as usize;
    let xor_length = cursor.read_u16_le()? as usize;
    let xor_mask = cursor.read_slice(xor_length)?;
    let and_mask = cursor.read_slice(and_length)?;
    let image = build_cursor_image(hotspot_x, hotspot_y, width, height, xor_bpp, xor_mask, and_mask, palette)?;
    Ok((cache_index, image))
}

fn build_cursor_image(hotspot_x: u16, hotspot_y: u16, width: u16, height: u16, xor_bpp: u16, xor_mask: &[u8], and_mask: &[u8], palette: &Palette) -> RdpResult<CursorImage> {
    if width == 0 || height == 0 || width > MAX_CURSOR_EDGE || height > MAX_CURSOR_EDGE {
        return Err(RdpProtocolError::protocol(format!("unexpected pointer size: {}x{}", width, height)));
    }
    let width_usize = width as usize;
    let height_usize = height as usize;
    let and_stride = aligned_stride(width_usize.div_ceil(8));
    let use_and_mask = !and_mask.is_empty();
    if use_and_mask && and_mask.len() < and_stride * height_usize {
        return Err(RdpProtocolError::underrun(format!("pointer and-mask underrun: need {} have {}", and_stride * height_usize, and_mask.len())));
    }
    if matches!(xor_bpp, 4 | 8) && !palette.loaded {
        return Err(RdpProtocolError::protocol(format!("received {}bpp pointer but the server has not sent a palette yet", xor_bpp)));
    }
    let xor_stride = match xor_bpp {
        1 => aligned_stride(width_usize.div_ceil(8)),
        4 => aligned_stride(width_usize.div_ceil(2)),
        8 => aligned_stride(width_usize),
        16 | 24 | 32 => aligned_stride(width_usize * (xor_bpp as usize >> 3)),
        _ => return Err(RdpProtocolError::unsupported(format!("unsupported pointer color depth: {}", xor_bpp))),
    };
    if xor_mask.len() < xor_stride * height_usize {
        return Err(RdpProtocolError::underrun(format!("pointer xor-mask underrun: need {} have {}", xor_stride * height_usize, xor_mask.len())));
    }
    if xor_bpp == 1 && !use_and_mask {
        return Err(RdpProtocolError::protocol("monochrome pointer is missing the and-mask".to_string()));
    }

    let flip = xor_bpp != 1;
    let mut rgba = vec![0u8; width_usize * height_usize * 4];
    for row in 0..height_usize {
        let source_row = match flip {
            true => height_usize - 1 - row,
            false => row,
        };
        for column in 0..width_usize {
            let and_pixel = use_and_mask && and_mask[source_row * and_stride + column / 8] & (0x80 >> (column % 8)) != 0;
            let pixel = match xor_bpp {
                1 => {
                    let xor_pixel = xor_mask[source_row * xor_stride + column / 8] & (0x80 >> (column % 8)) != 0;
                    match (and_pixel, xor_pixel) {
                        (false, false) => [0, 0, 0, 0xFF],
                        (false, true) => [0xFF, 0xFF, 0xFF, 0xFF],
                        (true, false) => [0, 0, 0, 0],
                        (true, true) => inverted_pixel(column, row),
                    }
                }
                _ => {
                    let raw = read_xor_pixel(xor_mask, source_row * xor_stride, column, xor_bpp, palette);
                    apply_and_mask(raw, and_pixel, column, row, xor_bpp)
                }
            };
            let target = (row * width_usize + column) * 4;
            rgba[target..target + 4].copy_from_slice(&pixel);
        }
    }
    Ok(CursorImage { hotspot_x, hotspot_y, width, height, rgba })
}

fn read_xor_pixel(xor_mask: &[u8], row_offset: usize, column: usize, xor_bpp: u16, palette: &Palette) -> [u8; 4] {
    match xor_bpp {
        4 => {
            let packed = xor_mask[row_offset + column / 2];
            let index = if column % 2 == 0 { packed >> 4 } else { packed & 0x0F };
            palette.lookup(index)
        }
        8 => palette.lookup(xor_mask[row_offset + column]),
        16 => {
            let offset = row_offset + column * 2;
            let value = u16::from_le_bytes([xor_mask[offset], xor_mask[offset + 1]]);
            let red = expand5(((value >> 10) & 0x1F) as u8);
            let green = expand5(((value >> 5) & 0x1F) as u8);
            let blue = expand5((value & 0x1F) as u8);
            [red, green, blue, 0xFF]
        }
        24 => {
            let offset = row_offset + column * 3;
            [xor_mask[offset + 2], xor_mask[offset + 1], xor_mask[offset], 0xFF]
        }
        _ => {
            let offset = row_offset + column * 4;
            [xor_mask[offset + 2], xor_mask[offset + 1], xor_mask[offset], xor_mask[offset + 3]]
        }
    }
}

fn apply_and_mask(pixel: [u8; 4], and_pixel: bool, column: usize, row: usize, xor_bpp: u16) -> [u8; 4] {
    if !and_pixel {
        return pixel;
    }
    if pixel == [0xFF, 0xFF, 0xFF, 0xFF] {
        return inverted_pixel(column, row);
    }
    if xor_bpp == 24 || pixel == [0, 0, 0, 0xFF] {
        return [0, 0, 0, 0];
    }
    pixel
}

fn inverted_pixel(column: usize, row: usize) -> [u8; 4] {
    let fill = match (column + row) & 1 {
        1 => 0x00u8,
        _ => 0xFFu8,
    };
    [fill, fill, fill, 0xFF]
}

fn expand5(value: u8) -> u8 {
    (value << 3) | (value >> 2)
}

fn aligned_stride(bytes: usize) -> usize {
    bytes + (bytes % 2)
}
