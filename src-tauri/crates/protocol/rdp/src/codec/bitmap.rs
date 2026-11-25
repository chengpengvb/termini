use crate::codec::color;
use crate::codec::planar;
use crate::codec::interleaved;
use crate::codec::color::Palette;
use crate::core::stream::ReadCursor;
use crate::core::error::{RdpProtocolError, RdpResult};

pub const BITMAP_COMPRESSION: u16 = 0x0001;
pub const NO_BITMAP_COMPRESSION_HDR: u16 = 0x0400;

pub struct BitmapRect {
    pub top: u16,
    pub left: u16,
    pub width: u16,
    pub height: u16,
    pub rgba: Vec<u8>,
    pub dest_right: u16,
    pub dest_bottom: u16,
}

pub fn parse_bitmap_update(cursor: &mut ReadCursor<'_>, palette: &Palette) -> RdpResult<Vec<BitmapRect>> {
    let count = cursor.read_u16_le()? as usize;
    let mut rectangles = Vec::with_capacity(count);
    for _ in 0..count {
        rectangles.push(parse_bitmap_data(cursor, palette)?);
    }
    Ok(rectangles)
}

fn parse_bitmap_data(cursor: &mut ReadCursor<'_>, palette: &Palette) -> RdpResult<BitmapRect> {
    let left = cursor.read_u16_le()?;
    let top = cursor.read_u16_le()?;
    let dest_right = cursor.read_u16_le()?;
    let dest_bottom = cursor.read_u16_le()?;
    let width = cursor.read_u16_le()?;
    let height = cursor.read_u16_le()?;
    let bits_per_pixel = cursor.read_u16_le()?;
    let flags = cursor.read_u16_le()?;
    let mut length = cursor.read_u16_le()? as usize;

    let compressed = flags & BITMAP_COMPRESSION != 0;
    if compressed && flags & NO_BITMAP_COMPRESSION_HDR == 0 {
        cursor.read_u16_le()?;
        length = cursor.read_u16_le()? as usize;
        cursor.read_u16_le()?;
        cursor.read_u16_le()?;
    }
    let data = cursor.read_slice(length)?;

    if width == 0 || height == 0 {
        return Err(RdpProtocolError::protocol("bitmap rectangle size is zero".to_string()));
    }
    let rgba = decode(data, width as usize, height as usize, bits_per_pixel, compressed, palette)?;
    Ok(BitmapRect { left, top, dest_right, dest_bottom, width, height, rgba })
}

pub fn decode(data: &[u8], width: usize, height: usize, bits_per_pixel: u16, compressed: bool, palette: &Palette) -> RdpResult<Vec<u8>> {
    if !compressed {
        return color::to_rgba(data, width, height, bits_per_pixel, palette);
    }
    if bits_per_pixel >= 32 {
        return planar::decompress(data, width, height, false, true);
    }
    let raw = interleaved::decompress(data, width, height, bits_per_pixel)?;
    color::to_rgba(&raw, width, height, bits_per_pixel, palette)
}
