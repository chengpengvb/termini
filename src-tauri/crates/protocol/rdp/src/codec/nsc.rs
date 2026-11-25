use crate::core::stream::ReadCursor;
use crate::core::error::{RdpProtocolError, RdpResult};

const PLANE_COUNT: usize = 4;
const PLANE_LUMA: usize = 0;
const PLANE_CHROMA_ORANGE: usize = 1;
const PLANE_CHROMA_GREEN: usize = 2;
const PLANE_ALPHA: usize = 3;
const RLE_TAIL_LENGTH: usize = 4;
const RLE_RUN_ESCAPE: u8 = 0xFF;
const RLE_RUN_BIAS: usize = 2;
const LUMA_ALIGNMENT: usize = 8;
const CHROMA_ALIGNMENT: usize = 2;

pub fn decode(source: &[u8], width: usize, height: usize) -> RdpResult<Vec<u8>> {
    if width == 0 || height == 0 {
        return Err(RdpProtocolError::protocol("NSCodec bitmap size is zero".to_string()));
    }

    let mut cursor = ReadCursor::new(source);
    let mut plane_sizes = [0usize; PLANE_COUNT];
    for size in plane_sizes.iter_mut() {
        *size = cursor.read_u32_le()? as usize;
    }
    let color_loss_level = cursor.read_u8()?;
    if !(1..=7).contains(&color_loss_level) {
        return Err(RdpProtocolError::protocol(format!("NSCodec ColorLossLevel out of range: {}", color_loss_level)));
    }
    let subsampling = cursor.read_u8()? != 0;
    cursor.skip(2)?;

    let padded_width = round_up(width, LUMA_ALIGNMENT);
    let padded_height = round_up(height, CHROMA_ALIGNMENT);
    let plane_length = padded_width * padded_height;

    let mut original_sizes = [width * height; PLANE_COUNT];
    if subsampling {
        original_sizes[PLANE_LUMA] = padded_width * height;
        original_sizes[PLANE_CHROMA_ORANGE] = (padded_width / 2) * (padded_height / 2);
        original_sizes[PLANE_CHROMA_GREEN] = original_sizes[PLANE_CHROMA_ORANGE];
    }

    let mut planes = Vec::with_capacity(PLANE_COUNT);
    for index in 0..PLANE_COUNT {
        let raw = cursor.read_slice(plane_sizes[index])?;
        planes.push(decompress_plane(raw, original_sizes[index], plane_length)?);
    }

    let shift = color_loss_level - 1;
    let luma_stride = if subsampling { padded_width } else { width };
    let chroma_stride = if subsampling { padded_width / 2 } else { width };
    let mut output = vec![0u8; width * height * 4];

    for y in 0..height {
        let luma_row = y * luma_stride;
        let chroma_row = if subsampling { (y / 2) * chroma_stride } else { y * chroma_stride };
        let alpha_row = y * width;
        for x in 0..width {
            let chroma_column = if subsampling { x / 2 } else { x };
            let luma = i16::from(planes[PLANE_LUMA][luma_row + x]);
            let orange = signed_chroma(planes[PLANE_CHROMA_ORANGE][chroma_row + chroma_column], shift);
            let green = signed_chroma(planes[PLANE_CHROMA_GREEN][chroma_row + chroma_column], shift);
            let target = (alpha_row + x) * 4;
            output[target] = clamp(luma + orange - green);
            output[target + 1] = clamp(luma + green);
            output[target + 2] = clamp(luma - orange - green);
            output[target + 3] = planes[PLANE_ALPHA][alpha_row + x];
        }
    }

    Ok(output)
}

fn decompress_plane(raw: &[u8], original: usize, length: usize) -> RdpResult<Vec<u8>> {
    if original > length {
        return Err(RdpProtocolError::protocol(format!("NSCodec plane original length {} exceeds buffer {}", original, length)));
    }

    let mut plane = vec![0u8; length];
    match raw.len() {
        0 => plane[..original].fill(RLE_RUN_ESCAPE),
        size if size < original => rle_decode(raw, &mut plane, original)?,
        _ => plane[..original].copy_from_slice(&raw[..original]),
    }
    Ok(plane)
}

fn rle_decode(raw: &[u8], plane: &mut [u8], original: usize) -> RdpResult<()> {
    let mut cursor = ReadCursor::new(raw);
    let mut left = original;
    let mut position = 0usize;

    while left > RLE_TAIL_LENGTH {
        let value = cursor.read_u8()?;
        if left > RLE_TAIL_LENGTH + 1 && cursor.peek_u8()? == value {
            cursor.skip(1)?;
            let run = match cursor.peek_u8()? {
                RLE_RUN_ESCAPE => {
                    cursor.skip(1)?;
                    cursor.read_u32_le()? as usize
                }
                _ => usize::from(cursor.read_u8()?) + RLE_RUN_BIAS,
            };
            if run > left || position + run > plane.len() {
                return Err(RdpProtocolError::protocol(format!("NSCodec RLE run {} exceeds remaining {}", run, left)));
            }
            plane[position..position + run].fill(value);
            position += run;
            left -= run;
            continue;
        }
        if position >= plane.len() {
            return Err(RdpProtocolError::protocol("NSCodec RLE literal exceeds plane".to_string()));
        }
        plane[position] = value;
        position += 1;
        left -= 1;
    }

    let tail = cursor.read_slice(RLE_TAIL_LENGTH)?;
    if position + RLE_TAIL_LENGTH > plane.len() {
        return Err(RdpProtocolError::protocol("NSCodec RLE trailer exceeds plane".to_string()));
    }
    plane[position..position + RLE_TAIL_LENGTH].copy_from_slice(tail);
    Ok(())
}

fn signed_chroma(value: u8, shift: u8) -> i16 {
    i16::from((i16::from(value) << shift) as i8)
}

fn clamp(value: i16) -> u8 {
    value.clamp(0, 0xFF) as u8
}

fn round_up(value: usize, alignment: usize) -> usize {
    value.div_ceil(alignment) * alignment
}
