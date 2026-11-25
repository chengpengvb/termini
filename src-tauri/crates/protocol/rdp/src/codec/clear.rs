use crate::codec::nsc;
use crate::gdi::framebuffer::Rect;
use crate::core::stream::ReadCursor;
use crate::core::error::{RdpProtocolError, RdpResult};

const FLAG_GLYPH_INDEX: u8 = 0x01;
const FLAG_GLYPH_HIT: u8 = 0x02;
const FLAG_CACHE_RESET: u8 = 0x04;

const GLYPH_ENTRIES: usize = 4000;
const GLYPH_MAX_PIXELS: usize = 1024 * 1024;
const VBAR_ENTRIES: usize = 32768;
const SHORT_VBAR_ENTRIES: usize = 16384;
const VBAR_MAX_HEIGHT: usize = 52;

const SUBCODEC_RAW: u8 = 0;
const SUBCODEC_NSCODEC: u8 = 1;
const SUBCODEC_RLEX: u8 = 2;

const BYTES_PER_PIXEL: usize = 4;
const COMPOSITION_HEADER_LENGTH: usize = 12;
const BAND_HEADER_LENGTH: usize = 11;
const SUBCODEC_HEADER_LENGTH: usize = 13;
const RLEX_MAX_PALETTE: usize = 127;

const LOG2_FLOOR: [u32; 256] = build_log2_floor();

const fn build_log2_floor() -> [u32; 256] {
    let mut table = [0u32; 256];
    let mut index = 1usize;
    while index < 256 {
        let mut value = 0u32;
        let mut probe = index >> 1;
        while probe > 0 {
            value += 1;
            probe >>= 1;
        }
        table[index] = value;
        index += 1;
    }
    table
}

pub struct ClearContext {
    glyphs: Vec<Vec<u8>>,
    vbars: Vec<Vec<u8>>,
    short_vbars: Vec<Vec<u8>>,
    vbar_cursor: usize,
    short_vbar_cursor: usize,
    sequence: u8,
}

impl Default for ClearContext {
    fn default() -> Self {
        Self { glyphs: vec![Vec::new(); GLYPH_ENTRIES], vbars: vec![Vec::new(); VBAR_ENTRIES], short_vbars: vec![Vec::new(); SHORT_VBAR_ENTRIES], vbar_cursor: 0, short_vbar_cursor: 0, sequence: 0 }
    }
}

impl ClearContext {
    pub fn reset(&mut self) {
        self.sequence = 0;
    }
}

struct Target<'a> {
    pixels: &'a mut [u8],
    stride: usize,
    surface: Rect,
}

impl Target<'_> {
    fn write_pixel(&mut self, x: usize, y: usize, pixel: [u8; 4]) {
        let offset = y * self.stride + x * BYTES_PER_PIXEL;
        self.pixels[offset..offset + BYTES_PER_PIXEL].copy_from_slice(&pixel);
    }

    fn read_pixel(&self, x: usize, y: usize) -> [u8; 4] {
        let offset = y * self.stride + x * BYTES_PER_PIXEL;
        [self.pixels[offset], self.pixels[offset + 1], self.pixels[offset + 2], self.pixels[offset + 3]]
    }

    fn blit(&mut self, left: usize, top: usize, width: usize, height: usize, source: &[u8], source_stride: usize) {
        let width = width.min(self.surface.right as usize - left);
        let height = height.min(self.surface.bottom as usize - top);
        for row in 0..height {
            let origin = row * source_stride;
            let destination = (top + row) * self.stride + left * BYTES_PER_PIXEL;
            self.pixels[destination..destination + width * BYTES_PER_PIXEL].copy_from_slice(&source[origin..origin + width * BYTES_PER_PIXEL]);
        }
    }
}

pub fn decode(context: &mut ClearContext, source: &[u8], target: &mut [u8], stride: usize, destination: Rect, surface: Rect) -> RdpResult<()> {
    if destination.left > surface.right || destination.top > surface.bottom {
        return Err(RdpProtocolError::protocol(format!("ClearCodec destination out of bounds: ({},{}) exceeds {}x{}", destination.left, destination.top, surface.right, surface.bottom)));
    }
    if target.len() < stride * surface.bottom as usize {
        return Err(RdpProtocolError::protocol(format!("ClearCodec destination buffer too small: need {} have {}", stride * surface.bottom as usize, target.len())));
    }

    let width = destination.width() as usize;
    let height = destination.height() as usize;
    let left = destination.left as usize;
    let top = destination.top as usize;
    let mut view = Target { pixels: target, stride, surface };

    let mut cursor = ReadCursor::new(source);
    let flags = cursor.read_u8()?;
    let sequence = cursor.read_u8()?;
    if context.sequence == 0 && sequence != 0 {
        context.sequence = sequence;
    }
    if sequence != context.sequence {
        return Err(RdpProtocolError::protocol(format!("ClearCodec sequence mismatch: {} expected {}", sequence, context.sequence)));
    }
    context.sequence = sequence.wrapping_add(1);

    if flags & FLAG_CACHE_RESET != 0 {
        context.vbar_cursor = 0;
        context.short_vbar_cursor = 0;
    }

    let glyph = read_glyph(context, &mut cursor, flags, &mut view, left, top, width, height)?;

    if cursor.remaining() < COMPOSITION_HEADER_LENGTH {
        if flags & (FLAG_GLYPH_HIT | FLAG_GLYPH_INDEX) == FLAG_GLYPH_HIT | FLAG_GLYPH_INDEX {
            return Ok(());
        }
        return Err(RdpProtocolError::protocol(format!("ClearCodec composite payload missing but glyph flags are 0x{:02x}", flags)));
    }

    let residual_length = cursor.read_u32_le()? as usize;
    let bands_length = cursor.read_u32_le()? as usize;
    let subcodec_length = cursor.read_u32_le()? as usize;

    if residual_length > 0 {
        let data = cursor.read_slice(residual_length)?;
        read_residual(data, &mut view, left, top, width, height)?;
    }
    if bands_length > 0 {
        let data = cursor.read_slice(bands_length)?;
        read_bands(context, data, &mut view, left, top, width, height)?;
    }
    if subcodec_length > 0 {
        let data = cursor.read_slice(subcodec_length)?;
        read_subcodecs(data, &mut view, left, top, width, height)?;
    }

    let Some(index) = glyph else {
        return Ok(());
    };
    let capture_width = width.min(surface.right as usize - left);
    let capture_height = height.min(surface.bottom as usize - top);
    context.glyphs[index].resize(width * height * BYTES_PER_PIXEL, 0);
    for row in 0..capture_height {
        for column in 0..capture_width {
            let pixel = view.read_pixel(left + column, top + row);
            let offset = (row * width + column) * BYTES_PER_PIXEL;
            context.glyphs[index][offset..offset + BYTES_PER_PIXEL].copy_from_slice(&pixel);
        }
    }
    Ok(())
}

fn read_glyph(context: &mut ClearContext, cursor: &mut ReadCursor<'_>, flags: u8, view: &mut Target<'_>, left: usize, top: usize, width: usize, height: usize) -> RdpResult<Option<usize>> {
    if flags & FLAG_GLYPH_HIT != 0 && flags & FLAG_GLYPH_INDEX == 0 {
        return Err(RdpProtocolError::protocol(format!("ClearCodec invalid glyph flags: 0x{:02x}", flags)));
    }
    if flags & FLAG_GLYPH_INDEX == 0 {
        return Ok(None);
    }
    if width * height > GLYPH_MAX_PIXELS {
        return Err(RdpProtocolError::protocol(format!("ClearCodec glyph too large: {}x{}", width, height)));
    }
    let index = cursor.read_u16_le()? as usize;
    if index >= GLYPH_ENTRIES {
        return Err(RdpProtocolError::protocol(format!("ClearCodec glyph index out of range: {}", index)));
    }

    if flags & FLAG_GLYPH_HIT == 0 {
        if width == 0 || height == 0 {
            return Err(RdpProtocolError::protocol(format!("ClearCodec glyph size is zero: {}x{}", width, height)));
        }
        return Ok(Some(index));
    }

    let entry = &context.glyphs[index];
    if entry.is_empty() {
        return Err(RdpProtocolError::protocol(format!("ClearCodec glyph cache entry is empty: {}", index)));
    }
    if width * height * BYTES_PER_PIXEL > entry.len() {
        return Err(RdpProtocolError::protocol(format!("ClearCodec glyph cache entry too small: need {} have {}", width * height * BYTES_PER_PIXEL, entry.len())));
    }
    let source = entry.clone();
    view.blit(left, top, width, height, &source, width * BYTES_PER_PIXEL);
    Ok(None)
}

fn read_residual(data: &[u8], view: &mut Target<'_>, left: usize, top: usize, width: usize, height: usize) -> RdpResult<()> {
    let mut cursor = ReadCursor::new(data);
    let pixel_count = width * height;
    let mut pixels = vec![0u8; pixel_count * BYTES_PER_PIXEL];
    let mut written = 0usize;

    while cursor.remaining() > 0 {
        let blue = cursor.read_u8()?;
        let green = cursor.read_u8()?;
        let red = cursor.read_u8()?;
        let run = read_run_length(&mut cursor)?;
        if written >= pixel_count || run > pixel_count - written {
            return Err(RdpProtocolError::protocol(format!("ClearCodec residual run out of bounds: {}+{} exceeds {}", written, run, pixel_count)));
        }
        for index in 0..run {
            let offset = (written + index) * BYTES_PER_PIXEL;
            pixels[offset..offset + BYTES_PER_PIXEL].copy_from_slice(&[red, green, blue, 0xFF]);
        }
        written += run;
    }

    if written != pixel_count {
        return Err(RdpProtocolError::protocol(format!("ClearCodec residual coverage incomplete: {} expected {}", written, pixel_count)));
    }
    view.blit(left, top, width, height, &pixels, width * BYTES_PER_PIXEL);
    Ok(())
}

fn read_bands(context: &mut ClearContext, data: &[u8], view: &mut Target<'_>, left: usize, top: usize, width: usize, height: usize) -> RdpResult<()> {
    let mut cursor = ReadCursor::new(data);

    while cursor.remaining() >= BAND_HEADER_LENGTH {
        let x_start = cursor.read_u16_le()? as usize;
        let x_end = cursor.read_u16_le()? as usize;
        let y_start = cursor.read_u16_le()? as usize;
        let y_end = cursor.read_u16_le()? as usize;
        let blue = cursor.read_u8()?;
        let green = cursor.read_u8()?;
        let red = cursor.read_u8()?;
        if x_end < x_start || y_end < y_start {
            return Err(RdpProtocolError::protocol(format!("ClearCodec invalid band coordinates: ({},{})-({},{})", x_start, y_start, x_end, y_end)));
        }
        let background = [red, green, blue, 0xFF];
        let bar_height = y_end - y_start + 1;
        if bar_height > VBAR_MAX_HEIGHT {
            return Err(RdpProtocolError::protocol(format!("ClearCodec vbar height out of range: {}", bar_height)));
        }

        for index in 0..=x_end - x_start {
            let header = cursor.read_u16_le()?;
            let bar = match header & 0xC000 {
                0x4000 => {
                    let slot = (header & 0x3FFF) as usize;
                    let start = cursor.read_u8()? as usize;
                    let short = context.short_vbars[slot].clone();
                    Some(compose_vbar(bar_height, start, &short, background))
                }
                0x0000 => {
                    let start = (header & 0x00FF) as usize;
                    let stop = ((header >> 8) & 0x3F) as usize;
                    if stop < start {
                        return Err(RdpProtocolError::protocol(format!("ClearCodec invalid short vbar range: {} < {}", stop, start)));
                    }
                    let count = stop - start;
                    let mut short = vec![0u8; count * BYTES_PER_PIXEL];
                    for pixel in 0..count {
                        let blue = cursor.read_u8()?;
                        let green = cursor.read_u8()?;
                        let red = cursor.read_u8()?;
                        short[pixel * BYTES_PER_PIXEL..pixel * BYTES_PER_PIXEL + BYTES_PER_PIXEL].copy_from_slice(&[red, green, blue, 0xFF]);
                    }
                    context.short_vbars[context.short_vbar_cursor] = short.clone();
                    context.short_vbar_cursor = (context.short_vbar_cursor + 1) % SHORT_VBAR_ENTRIES;
                    Some(compose_vbar(bar_height, start, &short, background))
                }
                _ => {
                    let slot = (header & 0x7FFF) as usize;
                    if context.vbars[slot].is_empty() {
                        context.vbars[slot] = vec![0u8; bar_height * BYTES_PER_PIXEL];
                        for pixel in 0..bar_height {
                            context.vbars[slot][pixel * BYTES_PER_PIXEL + 3] = 0xFF;
                        }
                    }
                    None
                }
            };

            let slot = match bar {
                Some(pixels) => {
                    let slot = context.vbar_cursor;
                    context.vbars[slot] = pixels;
                    context.vbar_cursor = (context.vbar_cursor + 1) % VBAR_ENTRIES;
                    slot
                }
                None => (header & 0x7FFF) as usize,
            };
            if context.vbars[slot].len() != bar_height * BYTES_PER_PIXEL {
                context.vbars[slot].resize(bar_height * BYTES_PER_PIXEL, 0);
            }

            if index >= width {
                continue;
            }
            let column = left + x_start + index;
            if column >= view.surface.right as usize {
                return Err(RdpProtocolError::protocol(format!("ClearCodec band column out of bounds: {} exceeds {}", column, view.surface.right)));
            }
            for pixel in 0..bar_height.min(height) {
                let row = top + y_start + pixel;
                if row >= view.surface.bottom as usize {
                    return Err(RdpProtocolError::protocol(format!("ClearCodec band row out of bounds: {} exceeds {}", row, view.surface.bottom)));
                }
                let offset = pixel * BYTES_PER_PIXEL;
                let source = &context.vbars[slot];
                view.write_pixel(column, row, [source[offset], source[offset + 1], source[offset + 2], source[offset + 3]]);
            }
        }
    }

    Ok(())
}

fn compose_vbar(bar_height: usize, start: usize, short: &[u8], background: [u8; 4]) -> Vec<u8> {
    let mut pixels = Vec::with_capacity(bar_height * BYTES_PER_PIXEL);
    let short_count = short.len() / BYTES_PER_PIXEL;

    let leading = if start > bar_height { bar_height } else { start };
    for _ in 0..leading {
        pixels.extend_from_slice(&background);
    }
    let middle = if start + short_count > bar_height { bar_height.saturating_sub(start) } else { short_count };
    pixels.extend_from_slice(&short[..middle * BYTES_PER_PIXEL]);
    let trailing = bar_height.saturating_sub(start + short_count);
    for _ in 0..trailing {
        pixels.extend_from_slice(&background);
    }

    pixels.resize(bar_height * BYTES_PER_PIXEL, 0);
    pixels
}

fn read_subcodecs(data: &[u8], view: &mut Target<'_>, left: usize, top: usize, width: usize, height: usize) -> RdpResult<()> {
    let mut cursor = ReadCursor::new(data);

    while cursor.remaining() >= SUBCODEC_HEADER_LENGTH {
        let x_start = cursor.read_u16_le()? as usize;
        let y_start = cursor.read_u16_le()? as usize;
        let block_width = cursor.read_u16_le()? as usize;
        let block_height = cursor.read_u16_le()? as usize;
        let length = cursor.read_u32_le()? as usize;
        let subcodec = cursor.read_u8()?;
        let body = cursor.read_slice(length)?;

        let block_left = left + x_start;
        let block_top = top + y_start;
        if block_left + block_width > view.surface.right as usize || block_top + block_height > view.surface.bottom as usize {
            return Err(RdpProtocolError::protocol(format!("ClearCodec subcodec block out of bounds: ({},{}) {}x{}", block_left, block_top, block_width, block_height)));
        }
        if x_start + block_width > width || y_start + block_height > height {
            return Err(RdpProtocolError::protocol(format!("ClearCodec subcodec block exceeds bitmap: ({},{}) {}x{}", x_start, y_start, block_width, block_height)));
        }

        match subcodec {
            SUBCODEC_RAW => {
                let required = block_width * block_height * 3;
                if length != required {
                    return Err(RdpProtocolError::protocol(format!("ClearCodec raw subcodec length mismatch: {} expected {}", length, required)));
                }
                let mut pixels = vec![0u8; block_width * block_height * BYTES_PER_PIXEL];
                for index in 0..block_width * block_height {
                    let origin = index * 3;
                    let offset = index * BYTES_PER_PIXEL;
                    pixels[offset..offset + BYTES_PER_PIXEL].copy_from_slice(&[body[origin + 2], body[origin + 1], body[origin], 0xFF]);
                }
                view.blit(block_left, block_top, block_width, block_height, &pixels, block_width * BYTES_PER_PIXEL);
            }
            SUBCODEC_NSCODEC => {
                let pixels = nsc::decode(body, block_width, block_height)?;
                view.blit(block_left, block_top, block_width, block_height, &pixels, block_width * BYTES_PER_PIXEL);
            }
            SUBCODEC_RLEX => read_rlex(body, view, block_left, block_top, block_width, block_height)?,
            other => return Err(RdpProtocolError::unsupported(format!("ClearCodec unknown subcodec: {}", other))),
        }
    }

    Ok(())
}

fn read_rlex(data: &[u8], view: &mut Target<'_>, left: usize, top: usize, width: usize, height: usize) -> RdpResult<()> {
    let mut cursor = ReadCursor::new(data);
    let palette_count = cursor.read_u8()? as usize;
    if palette_count < 1 || palette_count > RLEX_MAX_PALETTE {
        return Err(RdpProtocolError::protocol(format!("ClearCodec RLEX invalid palette count: {}", palette_count)));
    }
    let mut palette = Vec::with_capacity(palette_count);
    for _ in 0..palette_count {
        let blue = cursor.read_u8()?;
        let green = cursor.read_u8()?;
        let red = cursor.read_u8()?;
        palette.push([red, green, blue, 0xFF]);
    }

    let bits = LOG2_FLOOR[palette_count - 1] + 1;
    let pixel_count = width * height;
    let mut written = 0usize;
    let mut x = 0usize;
    let mut y = 0usize;

    while cursor.remaining() > 0 {
        let packed = cursor.read_u8()?;
        let depth = (packed >> bits) & mask_bits(8 - bits);
        let stop = packed & mask_bits(bits);
        let start = stop.wrapping_sub(depth);
        let run = read_run_length(&mut cursor)?;
        if start as usize >= palette_count || stop as usize >= palette_count {
            return Err(RdpProtocolError::protocol(format!("ClearCodec RLEX palette index out of range: {}..{} exceeds {}", start, stop, palette_count)));
        }
        if written + run > pixel_count {
            return Err(RdpProtocolError::protocol(format!("ClearCodec RLEX run out of bounds: {}+{} exceeds {}", written, run, pixel_count)));
        }

        let color = palette[start as usize];
        for _ in 0..run {
            if left + x < view.surface.right as usize && top + y < view.surface.bottom as usize {
                view.write_pixel(left + x, top + y, color);
            }
            x += 1;
            if x >= width {
                y += 1;
                x = 0;
            }
        }
        written += run;

        if written + depth as usize + 1 > pixel_count {
            return Err(RdpProtocolError::protocol(format!("ClearCodec RLEX gradient out of bounds: {}+{} exceeds {}", written, depth as usize + 1, pixel_count)));
        }
        for step in 0..=depth {
            if left + x < view.surface.right as usize && top + y < view.surface.bottom as usize {
                view.write_pixel(left + x, top + y, palette[(start + step) as usize]);
            }
            x += 1;
            if x >= width {
                y += 1;
                x = 0;
            }
        }
        written += depth as usize + 1;
    }

    if written != pixel_count {
        return Err(RdpProtocolError::protocol(format!("ClearCodec RLEX coverage incomplete: {} expected {}", written, pixel_count)));
    }
    Ok(())
}

fn read_run_length(cursor: &mut ReadCursor<'_>) -> RdpResult<usize> {
    let short = cursor.read_u8()?;
    if short < 0xFF {
        return Ok(short as usize);
    }
    let medium = cursor.read_u16_le()?;
    if medium < 0xFFFF {
        return Ok(medium as usize);
    }
    Ok(cursor.read_u32_le()? as usize)
}

fn mask_bits(count: u32) -> u8 {
    ((1u16 << count) - 1) as u8
}
