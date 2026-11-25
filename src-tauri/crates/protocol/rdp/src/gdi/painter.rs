use crate::gdi::raster;
use crate::codec::color;
use crate::core::orders;
use crate::codec::color::Palette;
use crate::gdi::brush_cache::BrushCache;
use crate::gdi::glyph_cache::GlyphCache;
use crate::gdi::bitmap_cache::BitmapCache;
use crate::core::error::{RdpProtocolError, RdpResult};
use crate::gdi::framebuffer::{Framebuffer, Pattern, Rect};
use crate::core::orders::{Bounds, Brush, DrawKind, DrawOrder, GlyphOrder};

const SO_HORIZONTAL: u8 = 0x02;
const SO_VERTICAL: u8 = 0x04;
const SO_CHAR_INC_EQUAL_BM_BASE: u8 = 0x20;
const GLYPH_FRAGMENT_USE: u8 = 0xFE;
const GLYPH_FRAGMENT_ADD: u8 = 0xFF;
const FAST_INDEX_SPECIAL: i32 = -32768;
const HATCH_PATTERNS: [[u8; 8]; 6] = [
    [0xFF, 0xFF, 0xFF, 0x00, 0xFF, 0xFF, 0xFF, 0xFF],
    [0xF7, 0xF7, 0xF7, 0xF7, 0xF7, 0xF7, 0xF7, 0xF7],
    [0xFE, 0xFD, 0xFB, 0xF7, 0xEF, 0xDF, 0xBF, 0x7F],
    [0x7F, 0xBF, 0xDF, 0xEF, 0xF7, 0xFB, 0xFD, 0xFE],
    [0xF7, 0xF7, 0xF7, 0x00, 0xF7, 0xF7, 0xF7, 0xF7],
    [0x7E, 0xBD, 0xDB, 0xE7, 0xE7, 0xDB, 0xBD, 0x7E],
];

pub fn apply(framebuffer: &mut Framebuffer, order: &DrawOrder, color_depth: u16, palette: &Palette, bitmaps: &BitmapCache, brushes: &BrushCache, glyphs: &mut GlyphCache) -> RdpResult<()> {
    let clip = resolve_clip(framebuffer.surface(), order.clip);
    match &order.kind {
        DrawKind::DstBlt(dstblt) => {
            let rect = clip_rect(clip, dstblt.left, dstblt.top, dstblt.width, dstblt.height);
            framebuffer.blt_pattern(rect, &Pattern::Solid(0), dstblt.rop)
        }
        DrawKind::PatBlt(patblt) => {
            let rect = clip_rect(clip, patblt.left, patblt.top, patblt.width, patblt.height);
            let pattern = build_pattern(&patblt.brush, patblt.fore_color, patblt.back_color, color_depth, palette, brushes)?;
            framebuffer.blt_pattern_from(rect, &pattern, patblt.rop, 0, 0)
        }
        DrawKind::ScrBlt(scrblt) => {
            let rect = clip_rect(clip, scrblt.left, scrblt.top, scrblt.width, scrblt.height);
            if rect.is_empty() {
                return Ok(());
            }
            let source_left = scrblt.x_src + (rect.left as i32 - scrblt.left);
            let source_top = scrblt.y_src + (rect.top as i32 - scrblt.top);
            if source_left < 0 || source_top < 0 {
                return Err(RdpProtocolError::protocol(format!("ScrBlt source coordinates are negative: ({},{})", source_left, source_top)));
            }
            framebuffer.blt_copy(rect, source_left as u16, source_top as u16, scrblt.rop)
        }
        DrawKind::OpaqueRect(opaque) => {
            let rect = clip_rect(clip, opaque.left, opaque.top, opaque.width, opaque.height);
            let color = color::decode_order_color(opaque.color, color_depth, palette)?;
            framebuffer.blt_pattern(rect, &Pattern::Solid(color), raster::ROP3_PATCOPY)
        }
        DrawKind::MultiOpaqueRect(multi) => {
            let color = color::decode_order_color(multi.color, color_depth, palette)?;
            for rectangle in &multi.rectangles {
                let rect = clip_rect(clip, rectangle.left, rectangle.top, rectangle.width, rectangle.height);
                framebuffer.blt_pattern(rect, &Pattern::Solid(color), raster::ROP3_PATCOPY)?;
            }
            Ok(())
        }
        DrawKind::LineTo(line) => {
            let color = color::decode_order_color(line.pen_color, color_depth, palette)?;
            framebuffer.draw_line((line.start_x, line.start_y), (line.end_x, line.end_y), clip, color, line.rop2)
        }
        DrawKind::Polyline(polyline) => {
            let color = color::decode_order_color(polyline.pen_color, color_depth, palette)?;
            let mut from = (polyline.start_x, polyline.start_y);
            for point in &polyline.points {
                let to = (from.0 + point.x, from.1 + point.y);
                framebuffer.draw_line(from, to, clip, color, polyline.rop2)?;
                from = to;
            }
            Ok(())
        }
        DrawKind::MemBlt(memblt) => {
            let rect = clip_rect(clip, memblt.left, memblt.top, memblt.width, memblt.height);
            if rect.is_empty() {
                return Ok(());
            }
            let (source_left, source_top) = source_origin(&rect, memblt.left, memblt.top, memblt.x_src, memblt.y_src)?;
            let Some(bitmap) = bitmaps.get(memblt.cache_id, memblt.cache_index) else {
                return Ok(());
            };
            framebuffer.blt_bitmap_rop(rect, bitmap, source_left, source_top, &Pattern::Solid(0), memblt.rop)
        }
        DrawKind::Mem3Blt(mem3blt) => {
            let rect = clip_rect(clip, mem3blt.left, mem3blt.top, mem3blt.width, mem3blt.height);
            if rect.is_empty() {
                return Ok(());
            }
            let (source_left, source_top) = source_origin(&rect, mem3blt.left, mem3blt.top, mem3blt.x_src, mem3blt.y_src)?;
            let pattern = build_pattern(&mem3blt.brush, mem3blt.fore_color, mem3blt.back_color, color_depth, palette, brushes)?;
            let Some(bitmap) = bitmaps.get(mem3blt.cache_id, mem3blt.cache_index) else {
                return Ok(());
            };
            framebuffer.blt_bitmap_rop(rect, bitmap, source_left, source_top, &pattern, mem3blt.rop)
        }
        DrawKind::Glyph(glyph) => draw_glyphs(framebuffer, glyph, clip, color_depth, palette, glyphs),
    }
}

fn source_origin(rect: &Rect, left: i32, top: i32, x_src: i32, y_src: i32) -> RdpResult<(u16, u16)> {
    let source_left = x_src + (rect.left as i32 - left);
    let source_top = y_src + (rect.top as i32 - top);
    if source_left < 0 || source_top < 0 || source_left > u16::MAX as i32 || source_top > u16::MAX as i32 {
        return Err(RdpProtocolError::protocol(format!("MemBlt source coordinates are invalid: ({},{})", source_left, source_top)));
    }
    Ok((source_left as u16, source_top as u16))
}

fn resolve_clip(surface: Rect, bounds: Option<Bounds>) -> Rect {
    let Some(bounds) = bounds else {
        return surface;
    };
    let left = bounds.left.clamp(0, surface.right as i32) as u16;
    let top = bounds.top.clamp(0, surface.bottom as i32) as u16;
    let right = (bounds.right.saturating_add(1)).clamp(0, surface.right as i32) as u16;
    let bottom = (bounds.bottom.saturating_add(1)).clamp(0, surface.bottom as i32) as u16;
    Rect { left, top, right: right.max(left), bottom: bottom.max(top) }
}

fn clip_rect(clip: Rect, left: i32, top: i32, width: i32, height: i32) -> Rect {
    let right = left.saturating_add(width).clamp(clip.left as i32, clip.right as i32);
    let bottom = top.saturating_add(height).clamp(clip.top as i32, clip.bottom as i32);
    let left = left.clamp(clip.left as i32, clip.right as i32);
    let top = top.clamp(clip.top as i32, clip.bottom as i32);
    Rect { left: left as u16, top: top as u16, right: right.max(left) as u16, bottom: bottom.max(top) as u16 }
}

fn build_pattern(brush: &Brush, fore_color: u32, back_color: u32, color_depth: u16, palette: &Palette, brushes: &BrushCache) -> RdpResult<Pattern> {
    let fore = color::decode_order_color(fore_color, color_depth, palette)?;
    if brush.style & orders::BRUSH_STYLE_CACHED != 0 {
        return match brush.bpp {
            1 => Ok(monochrome_tile(brushes.monochrome(brush.hatch)?, fore, color::decode_order_color(back_color, color_depth, palette)?, brush)),
            _ => Ok(Pattern::Tile { pixels: *brushes.color(brush.hatch)?, origin_x: brush.x, origin_y: brush.y }),
        };
    }
    match brush.style {
        orders::BRUSH_STYLE_SOLID => Ok(Pattern::Solid(fore)),
        orders::BRUSH_STYLE_HATCHED => {
            let rows = HATCH_PATTERNS.get(brush.hatch as usize).ok_or_else(|| RdpProtocolError::protocol(format!("invalid brush hatch style: {}", brush.hatch)))?;
            Ok(monochrome_tile(rows, fore, color::decode_order_color(back_color, color_depth, palette)?, brush))
        }
        orders::BRUSH_STYLE_PATTERN => Ok(monochrome_tile(&brush.data, fore, color::decode_order_color(back_color, color_depth, palette)?, brush)),
        other => Err(RdpProtocolError::unsupported(format!("unimplemented brush style: 0x{:02x}", other))),
    }
}

fn monochrome_tile(rows: &[u8; 8], fore: u32, back: u32, brush: &Brush) -> Pattern {
    let mut pixels = [0u32; 64];
    for (row, bits) in rows.iter().enumerate() {
        for column in 0..8usize {
            pixels[row * 8 + column] = if bits & (0x80 >> column) != 0 { back } else { fore };
        }
    }
    Pattern::Tile { pixels, origin_x: brush.x, origin_y: brush.y }
}

fn draw_glyphs(framebuffer: &mut Framebuffer, order: &GlyphOrder, clip: Rect, color_depth: u16, palette: &Palette, glyphs: &mut GlyphCache) -> RdpResult<()> {
    let back = color::decode_order_color(order.back_color, color_depth, palette)?;
    let fore = color::decode_order_color(order.fore_color, color_depth, palette)?;
    let surface = framebuffer.surface();
    let mut op_left = order.op_left;
    let mut op_top = order.op_top;
    let mut op_right = order.op_right;
    let mut op_bottom = order.op_bottom;
    let mut x = order.x;
    let mut y = order.y;
    if op_bottom == FAST_INDEX_SPECIAL {
        let flags = (op_top & 0x0F) as u8;
        if flags & 0x01 != 0 {
            op_bottom = order.bk_bottom;
        }
        if flags & 0x02 != 0 {
            op_right = order.bk_right;
        }
        if flags & 0x04 != 0 {
            op_top = order.bk_top;
        }
        if flags & 0x08 != 0 {
            op_left = order.bk_left;
        }
    }
    if op_left == 0 {
        op_left = order.bk_left;
    }
    if op_right == 0 {
        op_right = order.bk_right;
    }
    if op_right > surface.right as i32 {
        op_right = surface.right as i32;
    }
    if x == FAST_INDEX_SPECIAL {
        x = order.bk_left;
    }
    if y == FAST_INDEX_SPECIAL {
        y = order.bk_top;
    }
    if !order.f_op_redundant {
        let op_width = if op_right > op_left { op_right - op_left + 1 } else { 0 };
        let op_height = if op_bottom > op_top { op_bottom - op_top + 1 } else { 0 };
        let rect = clip_rect(clip, op_left, op_top, op_width, op_height);
        framebuffer.blt_pattern(rect, &Pattern::Solid(fore), raster::ROP3_PATCOPY)?;
    }
    let bk_width = if order.bk_right > order.bk_left { order.bk_right - order.bk_left + 1 } else { 0 };
    let bk_height = if order.bk_bottom > order.bk_top { order.bk_bottom - order.bk_top + 1 } else { 0 };
    let bound = clip_rect(clip, order.bk_left, order.bk_top, bk_width, bk_height);
    paint_glyph_stream(framebuffer, glyphs, &order.data, order.cache_id, order.ul_char_inc, order.fl_accel, &mut x, &mut y, bound, back)?;
    Ok(())
}

fn paint_glyph_stream(
    framebuffer: &mut Framebuffer,
    glyphs: &mut GlyphCache,
    data: &[u8],
    cache_id: u8,
    ul_char_inc: u8,
    fl_accel: u8,
    x: &mut i32,
    y: &mut i32,
    bound: Rect,
    color: u32,
) -> RdpResult<()> {
    let mut index = 0usize;
    while index < data.len() {
        let opcode = data[index];
        index += 1;
        match opcode {
            GLYPH_FRAGMENT_USE => {
                if index >= data.len() {
                    return Err(RdpProtocolError::underrun(format!("glyph USE fragment is missing cacheIndex: data={:02x?}", data)));
                }
                let fragment_id = data[index];
                index += 1;
                let fragment = glyphs.fragment_get(fragment_id)?.to_vec();
                if index < data.len() {
                    let mut use_x = *x;
                    let mut use_y = *y;
                    index = glyph_offset(data, index, &mut use_x, &mut use_y, ul_char_inc, fl_accel)?;
                    if fragment.get(1) == Some(&0) {
                        *x = use_x;
                        *y = use_y;
                    }
                }
                let mut nested = 0usize;
                while nested < fragment.len() {
                    let cache_index = fragment[nested];
                    nested += 1;
                    nested = glyph_offset(&fragment, nested, x, y, ul_char_inc, fl_accel)?;
                    paint_glyph(framebuffer, glyphs, cache_id, cache_index as u16, x, y, ul_char_inc, fl_accel, bound, color)?;
                }
            }
            GLYPH_FRAGMENT_ADD => {
                if index + 2 > data.len() {
                    return Err(RdpProtocolError::underrun(format!("glyph ADD fragment is missing cacheIndex/size: data={:02x?}", data)));
                }
                let fragment_id = data[index];
                let size = data[index + 1] as usize;
                let end = index - 1;
                index += 2;
                if size > end {
                    return Err(RdpProtocolError::underrun(format!("glyph ADD fragment size {} exceeds preceding bytes {}: data={:02x?}", size, end, data)));
                }
                glyphs.fragment_put(fragment_id, &data[end - size..end])?;
            }
            _ => {
                index = glyph_offset(data, index, x, y, ul_char_inc, fl_accel)?;
                paint_glyph(framebuffer, glyphs, cache_id, opcode as u16, x, y, ul_char_inc, fl_accel, bound, color)?;
            }
        }
    }
    Ok(())
}

fn glyph_offset(data: &[u8], mut index: usize, x: &mut i32, y: &mut i32, ul_char_inc: u8, fl_accel: u8) -> RdpResult<usize> {
    if ul_char_inc != 0 || fl_accel & SO_CHAR_INC_EQUAL_BM_BASE != 0 {
        return Ok(index);
    }
    if index >= data.len() {
        return Err(RdpProtocolError::underrun(format!("glyph offset data underrun at {}: data={:02x?}", index, data)));
    }
    let mut offset = data[index] as i32;
    index += 1;
    if offset & 0x80 != 0 {
        if index + 1 >= data.len() {
            return Err(RdpProtocolError::underrun(format!("glyph 16-bit offset data underrun at {}: data={:02x?}", index, data)));
        }
        offset = data[index] as i32 | ((data[index + 1] as i32) << 8);
        index += 2;
    }
    if fl_accel & SO_VERTICAL != 0 {
        *y += offset;
    }
    if fl_accel & SO_HORIZONTAL != 0 {
        *x += offset;
    }
    Ok(index)
}

fn paint_glyph(framebuffer: &mut Framebuffer, glyphs: &GlyphCache, cache_id: u8, cache_index: u16, x: &mut i32, y: &mut i32, ul_char_inc: u8, fl_accel: u8, bound: Rect, color: u32) -> RdpResult<()> {
    let glyph = glyphs.get(cache_id, cache_index)?;
    let dest_x = glyph.x + *x;
    let dest_y = glyph.y + *y;
    let stride = (glyph.cx as usize + 7) / 8;
    framebuffer.blit_mono(dest_x, dest_y, glyph.cx as i32, glyph.cy as i32, 0, 0, &glyph.aj, stride, color, bound)?;
    if ul_char_inc != 0 {
        *x += ul_char_inc as i32;
    } else if fl_accel & SO_CHAR_INC_EQUAL_BM_BASE != 0 {
        *x += glyph.cx as i32;
    }
    Ok(())
}
