use crate::core::error::{RdpProtocolError, RdpResult};

const FORMAT_HEADER_CLL_MASK: u8 = 0x07;
const FORMAT_HEADER_CS: u8 = 0x08;
const FORMAT_HEADER_RLE: u8 = 0x10;
const FORMAT_HEADER_NA: u8 = 0x20;

pub fn decompress(source: &[u8], width: usize, height: usize, use_alpha: bool, bgr: bool) -> RdpResult<Vec<u8>> {
    if source.is_empty() {
        return Err(RdpProtocolError::underrun("planar data is empty".to_string()));
    }
    if width == 0 || height == 0 {
        return Err(RdpProtocolError::protocol("planar bitmap size is zero".to_string()));
    }

    let format_header = source[0];
    let color_loss_level = format_header & FORMAT_HEADER_CLL_MASK;
    let chroma_subsampling = format_header & FORMAT_HEADER_CS != 0;
    let rle = format_header & FORMAT_HEADER_RLE != 0;
    let alpha = format_header & FORMAT_HEADER_NA == 0;

    if color_loss_level == 0 && chroma_subsampling {
        return Err(RdpProtocolError::protocol("planar chroma subsampling requires YCoCg".to_string()));
    }

    let body = &source[1..];
    let plane_size = width * height;
    let (chroma_width, chroma_height) = match chroma_subsampling {
        true => (width.div_ceil(2), height.div_ceil(2)),
        false => (width, height),
    };
    let chroma_size = chroma_width * chroma_height;
    let mut planes = [vec![0u8; plane_size], vec![0u8; chroma_size], vec![0u8; chroma_size], vec![0xFFu8; plane_size]];

    match rle {
        true => {
            let mut offset = 0usize;
            if alpha {
                offset += decompress_plane_rle(&body[offset..], width, height, &mut planes[3])?;
            }
            offset += decompress_plane_rle(&body[offset..], width, height, &mut planes[0])?;
            offset += decompress_plane_rle(&body[offset..], chroma_width, chroma_height, &mut planes[1])?;
            decompress_plane_rle(&body[offset..], chroma_width, chroma_height, &mut planes[2])?;
        }
        false => {
            let required = plane_size
                + chroma_size * 2
                + match alpha {
                    true => plane_size,
                    false => 0,
                };
            if body.len() < required {
                return Err(RdpProtocolError::underrun(format!("planar raw plane data underrun: need {} have {}", required, body.len())));
            }
            let mut offset = 0usize;
            if alpha {
                planes[3].copy_from_slice(&body[offset..offset + plane_size]);
                offset += plane_size;
            }
            planes[0].copy_from_slice(&body[offset..offset + plane_size]);
            offset += plane_size;
            planes[1].copy_from_slice(&body[offset..offset + chroma_size]);
            offset += chroma_size;
            planes[2].copy_from_slice(&body[offset..offset + chroma_size]);
        }
    }

    if chroma_subsampling {
        planes[1] = expand_chroma(&planes[1], width, height, chroma_width);
        planes[2] = expand_chroma(&planes[2], width, height, chroma_width);
    }

    let mut output = vec![0u8; plane_size * 4];
    for index in 0..plane_size {
        let target = index * 4;
        let opacity = match alpha && use_alpha {
            true => planes[3][index],
            false => 0xFF,
        };
        let mut pixel = match color_loss_level {
            0 => [planes[0][index], planes[1][index], planes[2][index], opacity],
            shift => ycocg_to_rgba(planes[0][index], planes[1][index], planes[2][index], opacity, shift),
        };
        if bgr && alpha {
            pixel.swap(0, 2);
        }
        output[target..target + 4].copy_from_slice(&pixel);
    }
    Ok(output)
}

fn expand_chroma(plane: &[u8], width: usize, height: usize, plane_width: usize) -> Vec<u8> {
    let mut expanded = vec![0u8; width * height];
    for row in 0..height {
        let origin = row / 2 * plane_width;
        for column in 0..width {
            expanded[row * width + column] = plane[origin + column / 2];
        }
    }
    expanded
}

fn ycocg_to_rgba(luma: u8, orange: u8, green: u8, opacity: u8, shift: u8) -> [u8; 4] {
    let orange_chroma = shift_chroma(orange, shift);
    let green_chroma = shift_chroma(green, shift);
    let luma = i16::from(luma);
    let temp = luma - green_chroma;
    [clamp_byte(temp - orange_chroma), clamp_byte(luma + green_chroma), clamp_byte(temp + orange_chroma), opacity]
}

fn shift_chroma(value: u8, shift: u8) -> i16 {
    i16::from((value << (shift - 1)) as i8)
}

fn decompress_plane_rle(source: &[u8], width: usize, height: usize, plane: &mut [u8]) -> RdpResult<usize> {
    let mut position = 0usize;
    let mut previous_row: Option<usize> = None;

    for row in 0..height {
        let row_offset = row * width;
        let mut column = 0usize;
        let mut pixel = 0u8;
        let mut delta = 0i16;
        while column < width {
            let control = *source.get(position).ok_or_else(|| RdpProtocolError::underrun("planar RLE data exhausted".to_string()))?;
            position += 1;
            let mut run_length = (control & 0x0F) as usize;
            let mut raw_bytes = ((control >> 4) & 0x0F) as usize;
            if run_length == 1 {
                run_length = raw_bytes + 16;
                raw_bytes = 0;
            } else if run_length == 2 {
                run_length = raw_bytes + 32;
                raw_bytes = 0;
            }
            if column + raw_bytes + run_length > width {
                return Err(RdpProtocolError::protocol("planar RLE scanline overflow".to_string()));
            }

            match previous_row {
                None => {
                    for _ in 0..raw_bytes {
                        pixel = *source.get(position).ok_or_else(|| RdpProtocolError::underrun("planar RLE data exhausted".to_string()))?;
                        position += 1;
                        plane[row_offset + column] = pixel;
                        column += 1;
                    }
                    for _ in 0..run_length {
                        plane[row_offset + column] = pixel;
                        column += 1;
                    }
                }
                Some(previous_offset) => {
                    for _ in 0..raw_bytes {
                        let encoded = *source.get(position).ok_or_else(|| RdpProtocolError::underrun("planar RLE data exhausted".to_string()))?;
                        position += 1;
                        delta = match encoded & 1 {
                            1 => -(((encoded >> 1) + 1) as i16),
                            _ => (encoded >> 1) as i16,
                        };
                        plane[row_offset + column] = plane[previous_offset + column].wrapping_add(delta as u8);
                        column += 1;
                    }
                    for _ in 0..run_length {
                        plane[row_offset + column] = plane[previous_offset + column].wrapping_add(delta as u8);
                        column += 1;
                    }
                }
            }
        }
        previous_row = Some(row_offset);
    }

    Ok(position)
}

fn clamp_byte(value: i16) -> u8 {
    value.clamp(0, 255) as u8
}
