use crate::codec::bitmap;
use crate::core::stream::ReadCursor;
use crate::gdi::glyph_cache::CachedGlyph;
use crate::core::error::{RdpProtocolError, RdpResult};

const ORDER_STANDARD: u8 = 0x01;
const ORDER_SECONDARY: u8 = 0x02;
const ORDER_BOUNDS: u8 = 0x04;
const ORDER_TYPE_CHANGE: u8 = 0x08;
const ORDER_DELTA_COORDINATES: u8 = 0x10;
const ORDER_ZERO_BOUNDS_DELTAS: u8 = 0x20;
const ORDER_ZERO_FIELD_BYTE_BIT0: u8 = 0x40;
const ORDER_ZERO_FIELD_BYTE_BIT1: u8 = 0x80;

const ORDER_TYPE_DSTBLT: u8 = 0x00;
const ORDER_TYPE_PATBLT: u8 = 0x01;
const ORDER_TYPE_SCRBLT: u8 = 0x02;
const ORDER_TYPE_LINE_TO: u8 = 0x09;
const ORDER_TYPE_OPAQUE_RECT: u8 = 0x0A;
const ORDER_TYPE_MEMBLT: u8 = 0x0D;
const ORDER_TYPE_MEM3BLT: u8 = 0x0E;
const ORDER_TYPE_MULTI_OPAQUE_RECT: u8 = 0x12;
const ORDER_TYPE_FAST_INDEX: u8 = 0x13;
const ORDER_TYPE_POLYLINE: u8 = 0x16;
const ORDER_TYPE_FAST_GLYPH: u8 = 0x18;
const ORDER_TYPE_GLYPH_INDEX: u8 = 0x1B;

const DSTBLT_ORDER_FIELD_BYTES: u8 = 1;
const PATBLT_ORDER_FIELD_BYTES: u8 = 2;
const SCRBLT_ORDER_FIELD_BYTES: u8 = 1;
const LINE_TO_ORDER_FIELD_BYTES: u8 = 2;
const OPAQUE_RECT_ORDER_FIELD_BYTES: u8 = 1;
const MEMBLT_ORDER_FIELD_BYTES: u8 = 2;
const MEM3BLT_ORDER_FIELD_BYTES: u8 = 3;
const MULTI_OPAQUE_RECT_ORDER_FIELD_BYTES: u8 = 2;
const FAST_INDEX_ORDER_FIELD_BYTES: u8 = 2;
const POLYLINE_ORDER_FIELD_BYTES: u8 = 1;
const FAST_GLYPH_ORDER_FIELD_BYTES: u8 = 2;
const GLYPH_INDEX_ORDER_FIELD_BYTES: u8 = 3;

const ORDER_TYPE_BITMAP_UNCOMPRESSED: u8 = 0x00;
const ORDER_TYPE_CACHE_BITMAP_COMPRESSED: u8 = 0x02;
const ORDER_TYPE_CACHE_GLYPH: u8 = 0x03;
const ORDER_TYPE_BITMAP_UNCOMPRESSED_V2: u8 = 0x04;
const ORDER_TYPE_BITMAP_COMPRESSED_V2: u8 = 0x05;
const ORDER_TYPE_CACHE_BRUSH: u8 = 0x07;
const ORDER_TYPE_BITMAP_COMPRESSED_V3: u8 = 0x08;

const ALTSEC_ORDER_TYPE_FRAME_MARKER: u8 = 0x0D;

const SECONDARY_BODY_PADDING: usize = 7;

const BMF_1BPP: u8 = 0x01;
const BMF_8BPP: u8 = 0x03;
const BMF_16BPP: u8 = 0x04;
const BMF_24BPP: u8 = 0x05;
const BMF_32BPP: u8 = 0x06;

pub const BRUSH_EDGE: usize = 8;
const BRUSH_COMPRESSED_INDEX_LENGTH: usize = 16;
const BRUSH_COMPRESSED_PALETTE_ENTRIES: usize = 4;

pub const BITMAP_CACHE_WAITING_LIST_INDEX: u16 = 32767;

const CBR2_HEIGHT_SAME_AS_WIDTH: u16 = 0x01;
const CBR2_PERSISTENT_KEY_PRESENT: u16 = 0x02;
const CBR2_NO_BITMAP_COMPRESSION_HDR: u16 = 0x08;
const CBR2_DO_NOT_CACHE: u16 = 0x10;
const CBR3_DO_NOT_CACHE: u16 = 0x10;
const CG_GLYPH_UNICODE_PRESENT: u16 = 0x0010;

const BITMAP_COMPRESSION_HDR_LENGTH: usize = 8;

const BOUND_LEFT: u8 = 0x01;
const BOUND_TOP: u8 = 0x02;
const BOUND_RIGHT: u8 = 0x04;
const BOUND_BOTTOM: u8 = 0x08;
const BOUND_DELTA_LEFT: u8 = 0x10;
const BOUND_DELTA_TOP: u8 = 0x20;
const BOUND_DELTA_RIGHT: u8 = 0x40;
const BOUND_DELTA_BOTTOM: u8 = 0x80;

pub const BRUSH_STYLE_SOLID: u8 = 0x00;
pub const BRUSH_STYLE_HATCHED: u8 = 0x02;
pub const BRUSH_STYLE_PATTERN: u8 = 0x03;
pub const BRUSH_STYLE_CACHED: u8 = 0x80;

const MAX_DELTA_RECTS: usize = 45;

#[derive(Clone, Copy, Default)]
pub struct Bounds {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

#[derive(Clone, Copy, Default)]
pub struct Brush {
    pub x: u8,
    pub y: u8,
    pub style: u8,
    pub hatch: u8,
    pub bpp: u16,
    pub data: [u8; BRUSH_EDGE],
}

#[derive(Clone, Copy, Default)]
pub struct DstBlt {
    pub left: i32,
    pub top: i32,
    pub width: i32,
    pub height: i32,
    pub rop: u8,
}

#[derive(Clone, Copy, Default)]
pub struct PatBlt {
    pub left: i32,
    pub top: i32,
    pub width: i32,
    pub height: i32,
    pub rop: u8,
    pub back_color: u32,
    pub fore_color: u32,
    pub brush: Brush,
}

#[derive(Clone, Copy, Default)]
pub struct ScrBlt {
    pub left: i32,
    pub top: i32,
    pub width: i32,
    pub height: i32,
    pub rop: u8,
    pub x_src: i32,
    pub y_src: i32,
}

#[derive(Clone, Copy, Default)]
pub struct OpaqueRect {
    pub left: i32,
    pub top: i32,
    pub width: i32,
    pub height: i32,
    pub color: u32,
}

#[derive(Clone, Copy, Default)]
pub struct DeltaRect {
    pub left: i32,
    pub top: i32,
    pub width: i32,
    pub height: i32,
}

#[derive(Clone, Default)]
pub struct MultiOpaqueRect {
    pub color: u32,
    pub rectangles: Vec<DeltaRect>,
}

#[derive(Clone, Copy, Default)]
pub struct LineTo {
    pub start_x: i32,
    pub start_y: i32,
    pub end_x: i32,
    pub end_y: i32,
    pub back_color: u32,
    pub rop2: u8,
    pub pen_style: u8,
    pub pen_width: u8,
    pub pen_color: u32,
}

#[derive(Clone, Copy, Default)]
pub struct DeltaPoint {
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Default)]
pub struct Polyline {
    pub start_x: i32,
    pub start_y: i32,
    pub rop2: u8,
    pub pen_color: u32,
    pub points: Vec<DeltaPoint>,
}

#[derive(Clone, Copy, Default)]
pub struct MemBlt {
    pub left: i32,
    pub top: i32,
    pub width: i32,
    pub height: i32,
    pub rop: u8,
    pub x_src: i32,
    pub y_src: i32,
    pub cache_id: u16,
    pub cache_index: u16,
}

#[derive(Clone, Copy, Default)]
pub struct Mem3Blt {
    pub left: i32,
    pub top: i32,
    pub width: i32,
    pub height: i32,
    pub rop: u8,
    pub x_src: i32,
    pub y_src: i32,
    pub back_color: u32,
    pub fore_color: u32,
    pub brush: Brush,
    pub cache_id: u16,
    pub cache_index: u16,
}

pub struct CacheBitmap {
    pub cache_id: u16,
    pub cache_index: u16,
    pub width: u16,
    pub height: u16,
    pub bits_per_pixel: u16,
    pub compressed: bool,
    pub persist_key: Option<(u32, u32)>,
    pub codec_id: Option<u8>,
    pub data: Vec<u8>,
}

pub struct CacheBrush {
    pub index: u8,
    pub bits_per_pixel: u16,
    pub data: Vec<u8>,
}

#[derive(Clone, Default)]
pub struct GlyphOrder {
    pub cache_id: u8,
    pub fl_accel: u8,
    pub ul_char_inc: u8,
    pub f_op_redundant: bool,
    pub back_color: u32,
    pub fore_color: u32,
    pub bk_left: i32,
    pub bk_top: i32,
    pub bk_right: i32,
    pub bk_bottom: i32,
    pub op_left: i32,
    pub op_top: i32,
    pub op_right: i32,
    pub op_bottom: i32,
    pub x: i32,
    pub y: i32,
    pub data: Vec<u8>,
    pub embedded: Option<CachedGlyph>,
}

pub struct CacheGlyph {
    pub cache_id: u8,
    pub glyphs: Vec<CachedGlyph>,
}

#[derive(Clone)]
pub enum DrawKind {
    DstBlt(DstBlt),
    PatBlt(PatBlt),
    ScrBlt(ScrBlt),
    OpaqueRect(OpaqueRect),
    MultiOpaqueRect(MultiOpaqueRect),
    LineTo(LineTo),
    Polyline(Polyline),
    MemBlt(MemBlt),
    Mem3Blt(Mem3Blt),
    Glyph(GlyphOrder),
}

#[derive(Clone)]
pub struct DrawOrder {
    pub clip: Option<Bounds>,
    pub kind: DrawKind,
}

pub enum Order {
    Draw(DrawOrder),
    CacheBitmap(CacheBitmap),
    CacheBrush(CacheBrush),
    CacheGlyph(CacheGlyph),
    FrameMarker,
}

#[derive(Default)]
pub struct OrderState {
    order_type: u8,
    field_flags: u32,
    delta_coordinates: bool,
    bounds: Bounds,
    dstblt: DstBlt,
    patblt: PatBlt,
    scrblt: ScrBlt,
    opaque_rect: OpaqueRect,
    multi_opaque_rect: MultiOpaqueRect,
    multi_opaque_bounds: OpaqueRect,
    line_to: LineTo,
    polyline: Polyline,
    memblt: MemBlt,
    mem3blt: Mem3Blt,
    glyph_index: GlyphOrder,
    fast_index: GlyphOrder,
    fast_glyph: GlyphOrder,
}

impl OrderState {
    pub fn parse_update(&mut self, cursor: &mut ReadCursor<'_>) -> RdpResult<Vec<Order>> {
        cursor.read_u16_le()?;
        let count = cursor.read_u16_le()?;
        cursor.read_u16_le()?;
        self.parse_orders(cursor, count)
    }

    pub fn parse_orders(&mut self, cursor: &mut ReadCursor<'_>, count: u16) -> RdpResult<Vec<Order>> {
        let mut orders = Vec::with_capacity(count as usize);
        for _ in 0..count {
            orders.push(self.parse_order(cursor)?);
        }
        Ok(orders)
    }

    fn parse_order(&mut self, cursor: &mut ReadCursor<'_>) -> RdpResult<Order> {
        let control = cursor.read_u8()?;
        if control & ORDER_STANDARD == 0 {
            return parse_altsec(cursor, control >> 2);
        }
        if control & ORDER_SECONDARY != 0 {
            let order_length = cursor.read_u16_le()? as i16;
            let extra_flags = cursor.read_u16_le()?;
            let order_type = cursor.read_u8()?;
            if order_length < 0 {
                return Err(RdpProtocolError::protocol(format!("invalid secondary drawing order length: {}", order_length)));
            }
            let mut body = cursor.sub_cursor(order_length as usize + SECONDARY_BODY_PADDING)?;
            return parse_secondary(&mut body, order_type, extra_flags);
        }
        if control & ORDER_TYPE_CHANGE != 0 {
            self.order_type = cursor.read_u8()?;
        }
        self.field_flags = read_field_flags(cursor, control, primary_field_bytes(self.order_type)?)?;
        let clip = if control & ORDER_BOUNDS != 0 {
            if control & ORDER_ZERO_BOUNDS_DELTAS == 0 {
                read_bounds(cursor, &mut self.bounds)?;
            }
            Some(self.bounds)
        } else {
            None
        };
        self.delta_coordinates = control & ORDER_DELTA_COORDINATES != 0;
        let kind = self.parse_primary(cursor)?;
        Ok(Order::Draw(DrawOrder { clip, kind }))
    }

    fn parse_primary(&mut self, cursor: &mut ReadCursor<'_>) -> RdpResult<DrawKind> {
        match self.order_type {
            ORDER_TYPE_DSTBLT => {
                let delta = self.delta_coordinates;
                let flags = self.field_flags;
                let order = &mut self.dstblt;
                read_coord_field(cursor, flags, 1, delta, &mut order.left)?;
                read_coord_field(cursor, flags, 2, delta, &mut order.top)?;
                read_coord_field(cursor, flags, 3, delta, &mut order.width)?;
                read_coord_field(cursor, flags, 4, delta, &mut order.height)?;
                read_byte_field(cursor, flags, 5, &mut order.rop)?;
                Ok(DrawKind::DstBlt(*order))
            }
            ORDER_TYPE_PATBLT => {
                let delta = self.delta_coordinates;
                let flags = self.field_flags;
                let order = &mut self.patblt;
                read_coord_field(cursor, flags, 1, delta, &mut order.left)?;
                read_coord_field(cursor, flags, 2, delta, &mut order.top)?;
                read_coord_field(cursor, flags, 3, delta, &mut order.width)?;
                read_coord_field(cursor, flags, 4, delta, &mut order.height)?;
                read_byte_field(cursor, flags, 5, &mut order.rop)?;
                read_color_field(cursor, flags, 6, &mut order.back_color)?;
                read_color_field(cursor, flags, 7, &mut order.fore_color)?;
                read_brush(cursor, (flags >> 7) as u8 & 0x1F, &mut order.brush)?;
                Ok(DrawKind::PatBlt(*order))
            }
            ORDER_TYPE_SCRBLT => {
                let delta = self.delta_coordinates;
                let flags = self.field_flags;
                let order = &mut self.scrblt;
                read_coord_field(cursor, flags, 1, delta, &mut order.left)?;
                read_coord_field(cursor, flags, 2, delta, &mut order.top)?;
                read_coord_field(cursor, flags, 3, delta, &mut order.width)?;
                read_coord_field(cursor, flags, 4, delta, &mut order.height)?;
                read_byte_field(cursor, flags, 5, &mut order.rop)?;
                read_coord_field(cursor, flags, 6, delta, &mut order.x_src)?;
                read_coord_field(cursor, flags, 7, delta, &mut order.y_src)?;
                Ok(DrawKind::ScrBlt(*order))
            }
            ORDER_TYPE_OPAQUE_RECT => {
                let delta = self.delta_coordinates;
                let flags = self.field_flags;
                let order = &mut self.opaque_rect;
                read_coord_field(cursor, flags, 1, delta, &mut order.left)?;
                read_coord_field(cursor, flags, 2, delta, &mut order.top)?;
                read_coord_field(cursor, flags, 3, delta, &mut order.width)?;
                read_coord_field(cursor, flags, 4, delta, &mut order.height)?;
                read_split_color(cursor, flags, 5, &mut order.color)?;
                Ok(DrawKind::OpaqueRect(*order))
            }
            ORDER_TYPE_MULTI_OPAQUE_RECT => {
                let delta = self.delta_coordinates;
                let flags = self.field_flags;
                let bounds = &mut self.multi_opaque_bounds;
                read_coord_field(cursor, flags, 1, delta, &mut bounds.left)?;
                read_coord_field(cursor, flags, 2, delta, &mut bounds.top)?;
                read_coord_field(cursor, flags, 3, delta, &mut bounds.width)?;
                read_coord_field(cursor, flags, 4, delta, &mut bounds.height)?;
                read_split_color(cursor, flags, 5, &mut self.multi_opaque_rect.color)?;
                let mut count = self.multi_opaque_rect.rectangles.len() as u8;
                read_byte_field(cursor, flags, 8, &mut count)?;
                if field_present(flags, 9) {
                    cursor.read_u16_le()?;
                    self.multi_opaque_rect.rectangles = read_delta_rects(cursor, count as usize)?;
                } else if count as usize > self.multi_opaque_rect.rectangles.len() {
                    return Err(RdpProtocolError::protocol(format!("multi opaque rect count out of range: {}", count)));
                } else {
                    self.multi_opaque_rect.rectangles.truncate(count as usize);
                }
                Ok(DrawKind::MultiOpaqueRect(self.multi_opaque_rect.clone()))
            }
            ORDER_TYPE_MEMBLT => {
                let delta = self.delta_coordinates;
                let flags = self.field_flags;
                let order = &mut self.memblt;
                let mut cache_id = order.cache_id;
                read_u16_field(cursor, flags, 1, &mut cache_id)?;
                order.cache_id = cache_id & 0xFF;
                read_coord_field(cursor, flags, 2, delta, &mut order.left)?;
                read_coord_field(cursor, flags, 3, delta, &mut order.top)?;
                read_coord_field(cursor, flags, 4, delta, &mut order.width)?;
                read_coord_field(cursor, flags, 5, delta, &mut order.height)?;
                read_byte_field(cursor, flags, 6, &mut order.rop)?;
                read_coord_field(cursor, flags, 7, delta, &mut order.x_src)?;
                read_coord_field(cursor, flags, 8, delta, &mut order.y_src)?;
                read_u16_field(cursor, flags, 9, &mut order.cache_index)?;
                Ok(DrawKind::MemBlt(*order))
            }
            ORDER_TYPE_MEM3BLT => {
                let delta = self.delta_coordinates;
                let flags = self.field_flags;
                let order = &mut self.mem3blt;
                let mut cache_id = order.cache_id;
                read_u16_field(cursor, flags, 1, &mut cache_id)?;
                order.cache_id = cache_id & 0xFF;
                read_coord_field(cursor, flags, 2, delta, &mut order.left)?;
                read_coord_field(cursor, flags, 3, delta, &mut order.top)?;
                read_coord_field(cursor, flags, 4, delta, &mut order.width)?;
                read_coord_field(cursor, flags, 5, delta, &mut order.height)?;
                read_byte_field(cursor, flags, 6, &mut order.rop)?;
                read_coord_field(cursor, flags, 7, delta, &mut order.x_src)?;
                read_coord_field(cursor, flags, 8, delta, &mut order.y_src)?;
                read_color_field(cursor, flags, 9, &mut order.back_color)?;
                read_color_field(cursor, flags, 10, &mut order.fore_color)?;
                read_brush(cursor, (flags >> 10) as u8 & 0x1F, &mut order.brush)?;
                read_u16_field(cursor, flags, 16, &mut order.cache_index)?;
                Ok(DrawKind::Mem3Blt(*order))
            }
            ORDER_TYPE_LINE_TO => {
                let delta = self.delta_coordinates;
                let flags = self.field_flags;
                let order = &mut self.line_to;
                let mut back_mode = 0u16;
                read_u16_field(cursor, flags, 1, &mut back_mode)?;
                read_coord_field(cursor, flags, 2, delta, &mut order.start_x)?;
                read_coord_field(cursor, flags, 3, delta, &mut order.start_y)?;
                read_coord_field(cursor, flags, 4, delta, &mut order.end_x)?;
                read_coord_field(cursor, flags, 5, delta, &mut order.end_y)?;
                read_color_field(cursor, flags, 6, &mut order.back_color)?;
                read_byte_field(cursor, flags, 7, &mut order.rop2)?;
                read_byte_field(cursor, flags, 8, &mut order.pen_style)?;
                read_byte_field(cursor, flags, 9, &mut order.pen_width)?;
                read_color_field(cursor, flags, 10, &mut order.pen_color)?;
                Ok(DrawKind::LineTo(*order))
            }
            ORDER_TYPE_POLYLINE => {
                let delta = self.delta_coordinates;
                let flags = self.field_flags;
                let order = &mut self.polyline;
                read_coord_field(cursor, flags, 1, delta, &mut order.start_x)?;
                read_coord_field(cursor, flags, 2, delta, &mut order.start_y)?;
                read_byte_field(cursor, flags, 3, &mut order.rop2)?;
                let mut brush_cache_entry = 0u16;
                read_u16_field(cursor, flags, 4, &mut brush_cache_entry)?;
                read_color_field(cursor, flags, 5, &mut order.pen_color)?;
                let mut count = order.points.len() as u8;
                read_byte_field(cursor, flags, 6, &mut count)?;
                if field_present(flags, 7) {
                    if count == 0 {
                        return Err(RdpProtocolError::protocol("polyline point count is zero".to_string()));
                    }
                    cursor.read_u8()?;
                    order.points = read_delta_points(cursor, count as usize)?;
                } else if count as usize > order.points.len() {
                    return Err(RdpProtocolError::protocol(format!("polyline point count out of range: {}", count)));
                } else {
                    order.points.truncate(count as usize);
                }
                Ok(DrawKind::Polyline(order.clone()))
            }
            ORDER_TYPE_GLYPH_INDEX => {
                let flags = self.field_flags;
                let order = &mut self.glyph_index;
                read_byte_field(cursor, flags, 1, &mut order.cache_id)?;
                read_byte_field(cursor, flags, 2, &mut order.fl_accel)?;
                read_byte_field(cursor, flags, 3, &mut order.ul_char_inc)?;
                let mut redundant = u8::from(order.f_op_redundant);
                read_byte_field(cursor, flags, 4, &mut redundant)?;
                order.f_op_redundant = redundant != 0;
                read_color_field(cursor, flags, 5, &mut order.back_color)?;
                read_color_field(cursor, flags, 6, &mut order.fore_color)?;
                read_i16_field(cursor, flags, 7, &mut order.bk_left)?;
                read_i16_field(cursor, flags, 8, &mut order.bk_top)?;
                read_i16_field(cursor, flags, 9, &mut order.bk_right)?;
                read_i16_field(cursor, flags, 10, &mut order.bk_bottom)?;
                read_i16_field(cursor, flags, 11, &mut order.op_left)?;
                read_i16_field(cursor, flags, 12, &mut order.op_top)?;
                read_i16_field(cursor, flags, 13, &mut order.op_right)?;
                read_i16_field(cursor, flags, 14, &mut order.op_bottom)?;
                let mut brush = Brush::default();
                read_brush(cursor, (flags >> 14) as u8 & 0x1F, &mut brush)?;
                read_i16_field(cursor, flags, 20, &mut order.x)?;
                read_i16_field(cursor, flags, 21, &mut order.y)?;
                if field_present(flags, 22) {
                    order.data = read_glyph_data(cursor)?;
                }
                order.embedded = None;
                Ok(DrawKind::Glyph(order.clone()))
            }
            ORDER_TYPE_FAST_INDEX => {
                let delta = self.delta_coordinates;
                let flags = self.field_flags;
                let order = &mut self.fast_index;
                read_byte_field(cursor, flags, 1, &mut order.cache_id)?;
                read_two_bytes_field(cursor, flags, 2, &mut order.ul_char_inc, &mut order.fl_accel)?;
                read_color_field(cursor, flags, 3, &mut order.back_color)?;
                read_color_field(cursor, flags, 4, &mut order.fore_color)?;
                read_coord_field(cursor, flags, 5, delta, &mut order.bk_left)?;
                read_coord_field(cursor, flags, 6, delta, &mut order.bk_top)?;
                read_coord_field(cursor, flags, 7, delta, &mut order.bk_right)?;
                read_coord_field(cursor, flags, 8, delta, &mut order.bk_bottom)?;
                read_coord_field(cursor, flags, 9, delta, &mut order.op_left)?;
                read_coord_field(cursor, flags, 10, delta, &mut order.op_top)?;
                read_coord_field(cursor, flags, 11, delta, &mut order.op_right)?;
                read_coord_field(cursor, flags, 12, delta, &mut order.op_bottom)?;
                read_coord_field(cursor, flags, 13, delta, &mut order.x)?;
                read_coord_field(cursor, flags, 14, delta, &mut order.y)?;
                if field_present(flags, 15) {
                    order.data = read_glyph_data(cursor)?;
                }
                order.f_op_redundant = false;
                order.embedded = None;
                Ok(DrawKind::Glyph(order.clone()))
            }
            ORDER_TYPE_FAST_GLYPH => {
                let delta = self.delta_coordinates;
                let flags = self.field_flags;
                let order = &mut self.fast_glyph;
                read_byte_field(cursor, flags, 1, &mut order.cache_id)?;
                if order.cache_id > 9 {
                    return Err(RdpProtocolError::protocol(format!("FastGlyph cacheId out of range: {}", order.cache_id)));
                }
                read_two_bytes_field(cursor, flags, 2, &mut order.ul_char_inc, &mut order.fl_accel)?;
                read_color_field(cursor, flags, 3, &mut order.back_color)?;
                read_color_field(cursor, flags, 4, &mut order.fore_color)?;
                read_coord_field(cursor, flags, 5, delta, &mut order.bk_left)?;
                read_coord_field(cursor, flags, 6, delta, &mut order.bk_top)?;
                read_coord_field(cursor, flags, 7, delta, &mut order.bk_right)?;
                read_coord_field(cursor, flags, 8, delta, &mut order.bk_bottom)?;
                read_coord_field(cursor, flags, 9, delta, &mut order.op_left)?;
                read_coord_field(cursor, flags, 10, delta, &mut order.op_top)?;
                read_coord_field(cursor, flags, 11, delta, &mut order.op_right)?;
                read_coord_field(cursor, flags, 12, delta, &mut order.op_bottom)?;
                read_coord_field(cursor, flags, 13, delta, &mut order.x)?;
                read_coord_field(cursor, flags, 14, delta, &mut order.y)?;
                order.f_op_redundant = false;
                order.embedded = None;
                if field_present(flags, 15) {
                    let data = read_glyph_data(cursor)?;
                    if data.is_empty() {
                        return Err(RdpProtocolError::protocol("FastGlyph data is empty".to_string()));
                    }
                    if data.len() > 1 {
                        let mut body = ReadCursor::new(&data[1..]);
                        let x = read_2byte_signed(&mut body)?;
                        let y = read_2byte_signed(&mut body)?;
                        let cx = read_2byte_unsigned(&mut body)?;
                        let cy = read_2byte_unsigned(&mut body)?;
                        if cx == 0 || cy == 0 {
                            return Err(RdpProtocolError::protocol(format!("FastGlyph glyph size is zero: {}x{}", cx, cy)));
                        }
                        order.embedded = Some(CachedGlyph { index: data[0] as u16, x, y, cx, cy, aj: body.rest().to_vec() });
                    }
                    order.data = vec![data[0], 0];
                }
                Ok(DrawKind::Glyph(order.clone()))
            }
            other => Err(RdpProtocolError::unsupported(format!("unimplemented primary drawing order: 0x{:02x}", other))),
        }
    }
}

fn primary_field_bytes(order_type: u8) -> RdpResult<u8> {
    match order_type {
        ORDER_TYPE_DSTBLT => Ok(DSTBLT_ORDER_FIELD_BYTES),
        ORDER_TYPE_PATBLT => Ok(PATBLT_ORDER_FIELD_BYTES),
        ORDER_TYPE_SCRBLT => Ok(SCRBLT_ORDER_FIELD_BYTES),
        ORDER_TYPE_LINE_TO => Ok(LINE_TO_ORDER_FIELD_BYTES),
        ORDER_TYPE_MEMBLT => Ok(MEMBLT_ORDER_FIELD_BYTES),
        ORDER_TYPE_MEM3BLT => Ok(MEM3BLT_ORDER_FIELD_BYTES),
        ORDER_TYPE_POLYLINE => Ok(POLYLINE_ORDER_FIELD_BYTES),
        ORDER_TYPE_FAST_INDEX => Ok(FAST_INDEX_ORDER_FIELD_BYTES),
        ORDER_TYPE_FAST_GLYPH => Ok(FAST_GLYPH_ORDER_FIELD_BYTES),
        ORDER_TYPE_GLYPH_INDEX => Ok(GLYPH_INDEX_ORDER_FIELD_BYTES),
        ORDER_TYPE_OPAQUE_RECT => Ok(OPAQUE_RECT_ORDER_FIELD_BYTES),
        ORDER_TYPE_MULTI_OPAQUE_RECT => Ok(MULTI_OPAQUE_RECT_ORDER_FIELD_BYTES),
        other => Err(RdpProtocolError::unsupported(format!("unimplemented primary drawing order: 0x{:02x}", other))),
    }
}

fn parse_altsec(cursor: &mut ReadCursor<'_>, order_type: u8) -> RdpResult<Order> {
    match order_type {
        ALTSEC_ORDER_TYPE_FRAME_MARKER => {
            cursor.read_u32_le()?;
            Ok(Order::FrameMarker)
        }
        other => Err(RdpProtocolError::unsupported(format!("unimplemented altsec drawing order: 0x{:02x}", other))),
    }
}

fn parse_secondary(cursor: &mut ReadCursor<'_>, order_type: u8, extra_flags: u16) -> RdpResult<Order> {
    match order_type {
        ORDER_TYPE_BITMAP_UNCOMPRESSED => Ok(Order::CacheBitmap(read_cache_bitmap(cursor, false, extra_flags)?)),
        ORDER_TYPE_CACHE_BITMAP_COMPRESSED => Ok(Order::CacheBitmap(read_cache_bitmap(cursor, true, extra_flags)?)),
        ORDER_TYPE_BITMAP_UNCOMPRESSED_V2 => Ok(Order::CacheBitmap(read_cache_bitmap_v2(cursor, false, extra_flags)?)),
        ORDER_TYPE_BITMAP_COMPRESSED_V2 => Ok(Order::CacheBitmap(read_cache_bitmap_v2(cursor, true, extra_flags)?)),
        ORDER_TYPE_BITMAP_COMPRESSED_V3 => Ok(Order::CacheBitmap(read_cache_bitmap_v3(cursor, extra_flags)?)),
        ORDER_TYPE_CACHE_GLYPH => Ok(Order::CacheGlyph(read_cache_glyph_v2(cursor, extra_flags)?)),
        ORDER_TYPE_CACHE_BRUSH => Ok(Order::CacheBrush(read_cache_brush(cursor)?)),
        other => Err(RdpProtocolError::unsupported(format!("unimplemented secondary drawing order: 0x{:02x}", other))),
    }
}

fn read_cache_brush(cursor: &mut ReadCursor<'_>) -> RdpResult<CacheBrush> {
    let index = cursor.read_u8()?;
    let bits_per_pixel = bmf_bits_per_pixel(cursor.read_u8()?)?;
    let width = cursor.read_u8()? as usize;
    let height = cursor.read_u8()? as usize;
    cursor.read_u8()?;
    let length = cursor.read_u8()? as usize;
    if width != BRUSH_EDGE || height != BRUSH_EDGE {
        return Err(RdpProtocolError::protocol(format!("invalid cache brush size: {}x{}", width, height)));
    }
    if bits_per_pixel == 1 {
        if length != BRUSH_EDGE {
            return Err(RdpProtocolError::protocol(format!("invalid monochrome cache brush length: {}", length)));
        }
        return Ok(CacheBrush { index, bits_per_pixel, data: flip_rows(cursor.read_slice(BRUSH_EDGE)?, 1) });
    }
    let pixel_size = bits_per_pixel as usize / 8;
    if length == BRUSH_COMPRESSED_INDEX_LENGTH + BRUSH_COMPRESSED_PALETTE_ENTRIES * pixel_size {
        return Ok(CacheBrush { index, bits_per_pixel, data: decompress_brush(cursor, pixel_size)? });
    }
    let scanline = pixel_size * BRUSH_EDGE;
    Ok(CacheBrush { index, bits_per_pixel, data: flip_rows(cursor.read_slice(scanline * BRUSH_EDGE)?, scanline) })
}

fn decompress_brush(cursor: &mut ReadCursor<'_>, pixel_size: usize) -> RdpResult<Vec<u8>> {
    let indices = cursor.read_slice(BRUSH_COMPRESSED_INDEX_LENGTH)?;
    let palette = cursor.read_slice(BRUSH_COMPRESSED_PALETTE_ENTRIES * pixel_size)?;
    let scanline = pixel_size * BRUSH_EDGE;
    let mut data = vec![0u8; scanline * BRUSH_EDGE];
    for row in 0..BRUSH_EDGE {
        for column in 0..BRUSH_EDGE {
            let packed = indices[row * 2 + column / 4];
            let entry = ((packed >> ((3 - column % 4) * 2)) & 0x03) as usize * pixel_size;
            let target = (BRUSH_EDGE - 1 - row) * scanline + column * pixel_size;
            data[target..target + pixel_size].copy_from_slice(&palette[entry..entry + pixel_size]);
        }
    }
    Ok(data)
}

fn flip_rows(source: &[u8], scanline: usize) -> Vec<u8> {
    let mut data = vec![0u8; source.len()];
    for row in 0..BRUSH_EDGE {
        let origin = row * scanline;
        let target = (BRUSH_EDGE - 1 - row) * scanline;
        data[target..target + scanline].copy_from_slice(&source[origin..origin + scanline]);
    }
    data
}

fn read_cache_bitmap(cursor: &mut ReadCursor<'_>, compressed: bool, extra_flags: u16) -> RdpResult<CacheBitmap> {
    let cache_id = cursor.read_u8()? as u16;
    cursor.skip(1)?;
    let width = cursor.read_u8()? as u16;
    let height = cursor.read_u8()? as u16;
    let bits_per_pixel = cursor.read_u8()? as u16;
    if bits_per_pixel < 1 || bits_per_pixel > 32 {
        return Err(RdpProtocolError::protocol(format!("invalid cache bitmap color depth: {}", bits_per_pixel)));
    }
    let mut length = cursor.read_u16_le()? as usize;
    let cache_index = cursor.read_u16_le()?;
    if compressed && extra_flags & bitmap::NO_BITMAP_COMPRESSION_HDR == 0 {
        cursor.skip(BITMAP_COMPRESSION_HDR_LENGTH)?;
        if length < BITMAP_COMPRESSION_HDR_LENGTH {
            return Err(RdpProtocolError::protocol(format!("cache bitmap length too short for compression header: {}", length)));
        }
        length -= BITMAP_COMPRESSION_HDR_LENGTH;
    }
    if length == 0 {
        return Err(RdpProtocolError::protocol("cache bitmap data is empty".to_string()));
    }
    let data = cursor.read_vec(length)?;
    Ok(CacheBitmap { cache_id, cache_index, width, height, bits_per_pixel, compressed, persist_key: None, codec_id: None, data })
}

fn read_cache_bitmap_v2(cursor: &mut ReadCursor<'_>, compressed: bool, extra_flags: u16) -> RdpResult<CacheBitmap> {
    let cache_id = extra_flags & 0x0007;
    let cache_flags = (extra_flags & 0xFF80) >> 7;
    let bits_per_pixel = cbr2_bits_per_pixel((extra_flags & 0x0078) >> 3)?;
    let persist_key = if cache_flags & CBR2_PERSISTENT_KEY_PRESENT != 0 { Some((cursor.read_u32_le()?, cursor.read_u32_le()?)) } else { None };
    let width = read_2byte_unsigned(cursor)?;
    let height = if cache_flags & CBR2_HEIGHT_SAME_AS_WIDTH != 0 { width } else { read_2byte_unsigned(cursor)? };
    let mut length = read_4byte_unsigned(cursor)? as usize;
    let cache_index = read_2byte_unsigned(cursor)?;
    let cache_index = match cache_flags & CBR2_DO_NOT_CACHE != 0 {
        true => BITMAP_CACHE_WAITING_LIST_INDEX,
        false => cache_index,
    };
    if compressed && cache_flags & CBR2_NO_BITMAP_COMPRESSION_HDR == 0 {
        cursor.read_u16_le()?;
        length = cursor.read_u16_le()? as usize;
        cursor.read_u16_le()?;
        cursor.read_u16_le()?;
    }
    if length == 0 {
        return Err(RdpProtocolError::protocol("cache bitmap data is empty".to_string()));
    }
    let data = cursor.read_vec(length)?;
    Ok(CacheBitmap { cache_id, cache_index, width, height, bits_per_pixel, compressed, persist_key, codec_id: None, data })
}

fn read_cache_bitmap_v3(cursor: &mut ReadCursor<'_>, extra_flags: u16) -> RdpResult<CacheBitmap> {
    let cache_id = extra_flags & 0x0007;
    let cache_flags = (extra_flags >> 7) & 0x01FF;
    let bits_per_pixel = match cbr2_bits_per_pixel((extra_flags & 0x0078) >> 3) {
        Ok(value) => value,
        Err(_) => 0,
    };
    let cache_index = cursor.read_u16_le()?;
    let key1 = cursor.read_u32_le()?;
    let key2 = cursor.read_u32_le()?;
    let cache_index = if cache_flags & CBR3_DO_NOT_CACHE != 0 { BITMAP_CACHE_WAITING_LIST_INDEX } else { cache_index };
    let data_bpp = cursor.read_u8()? as u16;
    if data_bpp < 1 || data_bpp > 32 {
        return Err(RdpProtocolError::protocol(format!("invalid cache bitmap v3 color depth: {}", data_bpp)));
    }
    let bits_per_pixel = if bits_per_pixel == 0 { data_bpp } else { bits_per_pixel };
    cursor.skip(2)?;
    let codec_id = cursor.read_u8()?;
    let width = cursor.read_u16_le()?;
    let height = cursor.read_u16_le()?;
    let length = cursor.read_u32_le()? as usize;
    if length == 0 {
        return Err(RdpProtocolError::protocol("cache bitmap data is empty".to_string()));
    }
    let data = cursor.read_vec(length)?;
    Ok(CacheBitmap { cache_id, cache_index, width, height, bits_per_pixel, compressed: codec_id != 0, persist_key: Some((key1, key2)), codec_id: Some(codec_id), data })
}

fn read_cache_glyph_v2(cursor: &mut ReadCursor<'_>, extra_flags: u16) -> RdpResult<CacheGlyph> {
    let cache_id = (extra_flags & 0x000F) as u8;
    let count = (extra_flags >> 8) as usize;
    let mut glyphs = Vec::with_capacity(count);
    for _ in 0..count {
        let index = cursor.read_u8()? as u16;
        let x = read_2byte_signed(cursor)?;
        let y = read_2byte_signed(cursor)?;
        let cx = read_2byte_unsigned(cursor)?;
        let cy = read_2byte_unsigned(cursor)?;
        let mut cb = ((cx as usize + 7) / 8) * cy as usize;
        if cb % 4 != 0 {
            cb += 4 - cb % 4;
        }
        let aj = cursor.read_vec(cb)?;
        glyphs.push(CachedGlyph { index, x, y, cx, cy, aj });
    }
    if extra_flags & CG_GLYPH_UNICODE_PRESENT != 0 && count > 0 {
        cursor.skip(count * 2)?;
    }
    Ok(CacheGlyph { cache_id, glyphs })
}

fn cbr2_bits_per_pixel(identifier: u16) -> RdpResult<u16> {
    match identifier {
        3 => Ok(8),
        4 => Ok(16),
        5 => Ok(24),
        6 => Ok(32),
        other => Err(RdpProtocolError::protocol(format!("invalid cache bitmap color depth id: {}", other))),
    }
}

fn read_2byte_unsigned(cursor: &mut ReadCursor<'_>) -> RdpResult<u16> {
    let byte = cursor.read_u8()?;
    if byte & 0x80 == 0 {
        return Ok((byte & 0x7F) as u16);
    }
    Ok((((byte & 0x7F) as u16) << 8) | cursor.read_u8()? as u16)
}

fn read_4byte_unsigned(cursor: &mut ReadCursor<'_>) -> RdpResult<u32> {
    let byte = cursor.read_u8()?;
    let mut value = (byte & 0x3F) as u32;
    for _ in 0..(byte >> 6) {
        value = (value << 8) | cursor.read_u8()? as u32;
    }
    Ok(value)
}

fn read_field_flags(cursor: &mut ReadCursor<'_>, control: u8, field_bytes: u8) -> RdpResult<u32> {
    let mut count = field_bytes;
    if control & ORDER_ZERO_FIELD_BYTE_BIT0 != 0 {
        count = count.saturating_sub(1);
    }
    if control & ORDER_ZERO_FIELD_BYTE_BIT1 != 0 {
        count = if count > 1 { count - 2 } else { 0 };
    }
    let mut flags = 0u32;
    for index in 0..count {
        flags |= (cursor.read_u8()? as u32) << (index * 8);
    }
    Ok(flags)
}

fn read_bounds(cursor: &mut ReadCursor<'_>, bounds: &mut Bounds) -> RdpResult<()> {
    let flags = cursor.read_u8()?;
    read_bound_coord(cursor, flags, BOUND_LEFT, BOUND_DELTA_LEFT, &mut bounds.left)?;
    read_bound_coord(cursor, flags, BOUND_TOP, BOUND_DELTA_TOP, &mut bounds.top)?;
    read_bound_coord(cursor, flags, BOUND_RIGHT, BOUND_DELTA_RIGHT, &mut bounds.right)?;
    read_bound_coord(cursor, flags, BOUND_BOTTOM, BOUND_DELTA_BOTTOM, &mut bounds.bottom)?;
    Ok(())
}

fn read_bound_coord(cursor: &mut ReadCursor<'_>, flags: u8, absolute: u8, delta: u8, target: &mut i32) -> RdpResult<()> {
    if flags & absolute != 0 {
        *target = read_coord(cursor, *target, false)?;
    } else if flags & delta != 0 {
        *target = read_coord(cursor, *target, true)?;
    }
    Ok(())
}

fn field_present(flags: u32, number: u8) -> bool {
    flags & (1u32 << (number - 1)) != 0
}

fn read_coord(cursor: &mut ReadCursor<'_>, current: i32, delta: bool) -> RdpResult<i32> {
    if delta {
        Ok(current + cursor.read_u8()? as i8 as i32)
    } else {
        Ok(cursor.read_u16_le()? as i16 as i32)
    }
}

fn read_coord_field(cursor: &mut ReadCursor<'_>, flags: u32, number: u8, delta: bool, target: &mut i32) -> RdpResult<()> {
    if field_present(flags, number) {
        *target = read_coord(cursor, *target, delta)?;
    }
    Ok(())
}

fn read_byte_field(cursor: &mut ReadCursor<'_>, flags: u32, number: u8, target: &mut u8) -> RdpResult<()> {
    if field_present(flags, number) {
        *target = cursor.read_u8()?;
    }
    Ok(())
}

fn read_two_bytes_field(cursor: &mut ReadCursor<'_>, flags: u32, number: u8, first: &mut u8, second: &mut u8) -> RdpResult<()> {
    if field_present(flags, number) {
        *first = cursor.read_u8()?;
        *second = cursor.read_u8()?;
    }
    Ok(())
}

fn read_i16_field(cursor: &mut ReadCursor<'_>, flags: u32, number: u8, target: &mut i32) -> RdpResult<()> {
    if field_present(flags, number) {
        *target = cursor.read_u16_le()? as i16 as i32;
    }
    Ok(())
}

fn read_glyph_data(cursor: &mut ReadCursor<'_>) -> RdpResult<Vec<u8>> {
    let length = cursor.read_u8()? as usize;
    cursor.read_vec(length)
}

fn read_2byte_signed(cursor: &mut ReadCursor<'_>) -> RdpResult<i32> {
    let byte = cursor.read_u8()?;
    let negative = byte & 0x40 != 0;
    let mut value = (byte & 0x3F) as i32;
    if byte & 0x80 != 0 {
        value = (value << 8) | cursor.read_u8()? as i32;
    }
    if negative {
        value = -value;
    }
    Ok(value)
}

fn read_u16_field(cursor: &mut ReadCursor<'_>, flags: u32, number: u8, target: &mut u16) -> RdpResult<()> {
    if field_present(flags, number) {
        *target = cursor.read_u16_le()?;
    }
    Ok(())
}

fn read_color(cursor: &mut ReadCursor<'_>) -> RdpResult<u32> {
    let low = cursor.read_u8()? as u32;
    let mid = cursor.read_u8()? as u32;
    let high = cursor.read_u8()? as u32;
    Ok(low | (mid << 8) | (high << 16))
}

fn read_color_field(cursor: &mut ReadCursor<'_>, flags: u32, number: u8, target: &mut u32) -> RdpResult<()> {
    if field_present(flags, number) {
        *target = read_color(cursor)?;
    }
    Ok(())
}

fn read_split_color(cursor: &mut ReadCursor<'_>, flags: u32, first: u8, target: &mut u32) -> RdpResult<()> {
    if field_present(flags, first) {
        *target = (*target & 0x00FF_FF00) | cursor.read_u8()? as u32;
    }
    if field_present(flags, first + 1) {
        *target = (*target & 0x00FF_00FF) | ((cursor.read_u8()? as u32) << 8);
    }
    if field_present(flags, first + 2) {
        *target = (*target & 0x0000_FFFF) | ((cursor.read_u8()? as u32) << 16);
    }
    Ok(())
}

fn read_delta(cursor: &mut ReadCursor<'_>) -> RdpResult<i32> {
    let byte = cursor.read_u8()?;
    let mut value = if byte & 0x40 != 0 { byte as u32 | 0xFFFF_FFC0 } else { (byte & 0x3F) as u32 };
    if byte & 0x80 != 0 {
        value = (value << 8) | cursor.read_u8()? as u32;
    }
    Ok(value as i32)
}

fn read_delta_rects(cursor: &mut ReadCursor<'_>, count: usize) -> RdpResult<Vec<DeltaRect>> {
    if count > MAX_DELTA_RECTS {
        return Err(RdpProtocolError::protocol(format!("invalid delta rectangle count: {}", count)));
    }
    let zero_bits = cursor.read_slice((count + 1) / 2)?;
    let mut rectangles = vec![DeltaRect::default(); count];
    let mut flags = 0u8;
    for index in 0..count {
        if index % 2 == 0 {
            flags = zero_bits[index / 2];
        }
        if flags & 0x80 == 0 {
            rectangles[index].left = read_delta(cursor)?;
        }
        if flags & 0x40 == 0 {
            rectangles[index].top = read_delta(cursor)?;
        }
        if flags & 0x20 == 0 {
            rectangles[index].width = read_delta(cursor)?;
        } else if index > 0 {
            rectangles[index].width = rectangles[index - 1].width;
        }
        if flags & 0x10 == 0 {
            rectangles[index].height = read_delta(cursor)?;
        } else if index > 0 {
            rectangles[index].height = rectangles[index - 1].height;
        }
        if index > 0 {
            rectangles[index].left += rectangles[index - 1].left;
            rectangles[index].top += rectangles[index - 1].top;
        }
        flags <<= 4;
    }
    Ok(rectangles)
}

fn read_delta_points(cursor: &mut ReadCursor<'_>, count: usize) -> RdpResult<Vec<DeltaPoint>> {
    let zero_bits = cursor.read_slice((count + 3) / 4)?;
    let mut points = vec![DeltaPoint::default(); count];
    let mut flags = 0u8;
    for index in 0..count {
        if index % 4 == 0 {
            flags = zero_bits[index / 4];
        }
        if flags & 0x80 == 0 {
            points[index].x = read_delta(cursor)?;
        }
        if flags & 0x40 == 0 {
            points[index].y = read_delta(cursor)?;
        }
        flags <<= 2;
    }
    Ok(points)
}

fn read_brush(cursor: &mut ReadCursor<'_>, flags: u8, brush: &mut Brush) -> RdpResult<()> {
    if flags & 0x01 != 0 {
        brush.x = cursor.read_u8()?;
    }
    if flags & 0x02 != 0 {
        brush.y = cursor.read_u8()?;
    }
    if flags & 0x04 != 0 {
        brush.style = cursor.read_u8()?;
    }
    if flags & 0x08 != 0 {
        brush.hatch = cursor.read_u8()?;
    }
    if brush.style & BRUSH_STYLE_CACHED != 0 {
        brush.bpp = bmf_bits_per_pixel(brush.style)?;
    }
    if flags & 0x10 != 0 {
        for index in (1..BRUSH_EDGE).rev() {
            brush.data[index] = cursor.read_u8()?;
        }
        brush.data[0] = brush.hatch;
    }
    Ok(())
}

fn bmf_bits_per_pixel(bitmap_format: u8) -> RdpResult<u16> {
    match bitmap_format & !BRUSH_STYLE_CACHED {
        BMF_1BPP => Ok(1),
        BMF_8BPP => Ok(8),
        BMF_16BPP => Ok(16),
        BMF_24BPP => Ok(24),
        BMF_32BPP => Ok(32),
        other => Err(RdpProtocolError::protocol(format!("invalid brush bitmap format: 0x{:02x}", other))),
    }
}
