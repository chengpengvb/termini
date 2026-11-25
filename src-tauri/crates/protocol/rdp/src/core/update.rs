use bytes::Bytes;
use crate::codec::bitmap;
use crate::core::pointer;
use crate::core::surface;
use crate::core::fastpath;
use crate::codec::color::Palette;
use crate::codec::rfx::RfxContext;
use crate::gdi::framebuffer::Rect;
use crate::core::stream::ReadCursor;
use crate::codec::bitmap::BitmapRect;
use crate::core::pointer::PointerCache;
use crate::gdi::brush_cache::BrushCache;
use crate::gdi::glyph_cache::GlyphCache;
use crate::core::surface::SurfaceCommand;
use crate::core::fastpath::FastPathUpdate;
use crate::gdi::bitmap_cache::BitmapCache;
use crate::core::error::{RdpProtocolError, RdpResult};
use crate::core::orders::{DrawKind, DrawOrder, Order, OrderState};

pub const UPDATETYPE_ORDERS: u16 = 0x0000;
pub const UPDATETYPE_BITMAP: u16 = 0x0001;
pub const UPDATETYPE_PALETTE: u16 = 0x0002;
pub const UPDATETYPE_SYNCHRONIZE: u16 = 0x0003;

const PTR_MSG_TYPE_SYSTEM: u16 = 0x0001;
const PTR_MSG_TYPE_POSITION: u16 = 0x0003;
const PTR_MSG_TYPE_COLOR: u16 = 0x0006;
const PTR_MSG_TYPE_CACHED: u16 = 0x0007;
const PTR_MSG_TYPE_POINTER: u16 = 0x0008;
const PTR_MSG_TYPE_POINTER_LARGE: u16 = 0x0009;

const SYSPTR_NULL: u32 = 0x0000_0000;
const SYSPTR_DEFAULT: u32 = 0x0000_7F00;

#[derive(Default)]
pub struct UpdateState {
    pub palette: Palette,
    pub bitmaps: BitmapCache,
    pub brushes: BrushCache,
    pub glyphs: GlyphCache,
    orders: OrderState,
    pointers: PointerCache,
    remotefx: RfxContext,
}

pub enum DecodedUpdate {
    Bitmap(Vec<BitmapRect>),
    Cursor(Bytes),
    Orders(Vec<Order>),
    Surface(Vec<SurfaceCommand>),
}

pub fn decode_fastpath(update: &FastPathUpdate, state: &mut UpdateState, surface: Rect) -> RdpResult<Option<DecodedUpdate>> {
    match update.update_type {
        fastpath::UPDATE_TYPE_ORDERS => {
            let mut cursor = ReadCursor::new(&update.data);
            let count = cursor.read_u16_le()?;
            Ok(Some(DecodedUpdate::Orders(state.orders.parse_orders(&mut cursor, count)?)))
        }
        fastpath::UPDATE_TYPE_BITMAP | fastpath::UPDATE_TYPE_PALETTE => {
            let mut cursor = ReadCursor::new(&update.data);
            let update_type = cursor.read_u16_le()?;
            decode_slowpath(update_type, &mut cursor, state)
        }
        fastpath::UPDATE_TYPE_PTR_NULL => Ok(Some(DecodedUpdate::Cursor(pointer::encode_hidden()))),
        fastpath::UPDATE_TYPE_PTR_DEFAULT => Ok(Some(DecodedUpdate::Cursor(pointer::encode_default()))),
        fastpath::UPDATE_TYPE_COLOR => {
            let mut cursor = ReadCursor::new(&update.data);
            let (index, image) = pointer::parse_color_pointer(&mut cursor, &state.palette)?;
            Ok(Some(DecodedUpdate::Cursor(store_cursor(&mut state.pointers, index, &image)?)))
        }
        fastpath::UPDATE_TYPE_POINTER => {
            let mut cursor = ReadCursor::new(&update.data);
            let (index, image) = pointer::parse_new_pointer(&mut cursor, &state.palette)?;
            Ok(Some(DecodedUpdate::Cursor(store_cursor(&mut state.pointers, index, &image)?)))
        }
        fastpath::UPDATE_TYPE_LARGE_POINTER => {
            let mut cursor = ReadCursor::new(&update.data);
            let (index, image) = pointer::parse_large_pointer(&mut cursor, &state.palette)?;
            Ok(Some(DecodedUpdate::Cursor(store_cursor(&mut state.pointers, index, &image)?)))
        }
        fastpath::UPDATE_TYPE_CACHED => {
            let mut cursor = ReadCursor::new(&update.data);
            let index = pointer::parse_cached_pointer(&mut cursor)?;
            Ok(Some(DecodedUpdate::Cursor(state.pointers.get(index)?)))
        }
        fastpath::UPDATE_TYPE_SYNCHRONIZE | fastpath::UPDATE_TYPE_PTR_POSITION => Ok(None),
        fastpath::UPDATE_TYPE_SURFCMDS => {
            let mut cursor = ReadCursor::new(&update.data);
            Ok(Some(DecodedUpdate::Surface(surface::parse_commands(&mut cursor, &state.palette, &mut state.remotefx, surface)?)))
        }
        other => Err(RdpProtocolError::protocol(format!("unknown fastpath update type: {}", other))),
    }
}

pub fn decode_slowpath(update_type: u16, cursor: &mut ReadCursor<'_>, state: &mut UpdateState) -> RdpResult<Option<DecodedUpdate>> {
    match update_type {
        UPDATETYPE_ORDERS => Ok(Some(DecodedUpdate::Orders(state.orders.parse_update(cursor)?))),
        UPDATETYPE_BITMAP => Ok(Some(DecodedUpdate::Bitmap(bitmap::parse_bitmap_update(cursor, &state.palette)?))),
        UPDATETYPE_PALETTE => {
            parse_palette_update(cursor, &mut state.palette)?;
            Ok(None)
        }
        UPDATETYPE_SYNCHRONIZE => Ok(None),
        other => Err(RdpProtocolError::protocol(format!("unknown slowpath update type: {}", other))),
    }
}

pub fn decode_pointer(cursor: &mut ReadCursor<'_>, state: &mut UpdateState) -> RdpResult<Option<DecodedUpdate>> {
    let message_type = cursor.read_u16_le()?;
    cursor.read_u16_le()?;
    match message_type {
        PTR_MSG_TYPE_SYSTEM => match cursor.read_u32_le()? {
            SYSPTR_NULL => Ok(Some(DecodedUpdate::Cursor(pointer::encode_hidden()))),
            SYSPTR_DEFAULT => Ok(Some(DecodedUpdate::Cursor(pointer::encode_default()))),
            other => Err(RdpProtocolError::protocol(format!("unknown system pointer type: 0x{:08x}", other))),
        },
        PTR_MSG_TYPE_COLOR => {
            let (index, image) = pointer::parse_color_pointer(cursor, &state.palette)?;
            Ok(Some(DecodedUpdate::Cursor(store_cursor(&mut state.pointers, index, &image)?)))
        }
        PTR_MSG_TYPE_POINTER => {
            let (index, image) = pointer::parse_new_pointer(cursor, &state.palette)?;
            Ok(Some(DecodedUpdate::Cursor(store_cursor(&mut state.pointers, index, &image)?)))
        }
        PTR_MSG_TYPE_POINTER_LARGE => {
            let (index, image) = pointer::parse_large_pointer(cursor, &state.palette)?;
            Ok(Some(DecodedUpdate::Cursor(store_cursor(&mut state.pointers, index, &image)?)))
        }
        PTR_MSG_TYPE_CACHED => {
            let index = pointer::parse_cached_pointer(cursor)?;
            Ok(Some(DecodedUpdate::Cursor(state.pointers.get(index)?)))
        }
        PTR_MSG_TYPE_POSITION => Ok(None),
        other => Err(RdpProtocolError::protocol(format!("unknown slowpath pointer update type: {}", other))),
    }
}

pub fn consume_order(order: Order, state: &mut UpdateState) -> RdpResult<Option<DrawOrder>> {
    match order {
        Order::Draw(draw) => {
            if let DrawKind::Glyph(glyph) = &draw.kind {
                if let Some(embedded) = &glyph.embedded {
                    state.glyphs.store(glyph.cache_id, embedded.clone())?;
                }
            }
            Ok(Some(draw))
        }
        Order::CacheBitmap(entry) => {
            state.bitmaps.store(&entry, &state.palette, &mut state.remotefx)?;
            Ok(None)
        }
        Order::CacheBrush(entry) => {
            state.brushes.store(&entry, &state.palette)?;
            Ok(None)
        }
        Order::CacheGlyph(entry) => {
            for glyph in entry.glyphs {
                state.glyphs.store(entry.cache_id, glyph)?;
            }
            Ok(None)
        }
        Order::FrameMarker => Ok(None),
    }
}

fn parse_palette_update(cursor: &mut ReadCursor<'_>, palette: &mut Palette) -> RdpResult<()> {
    cursor.read_u16_le()?;
    let count = cursor.read_u32_le()? as usize;
    let raw = cursor.read_slice(count.saturating_mul(3))?;
    let entries = raw.chunks_exact(3).map(|chunk| [chunk[0], chunk[1], chunk[2]]).collect::<Vec<[u8; 3]>>();
    palette.load(&entries)
}

fn store_cursor(cache: &mut PointerCache, index: u16, image: &pointer::CursorImage) -> RdpResult<Bytes> {
    let payload = pointer::encode_image(image);
    cache.store(index, payload.clone())?;
    Ok(payload)
}
