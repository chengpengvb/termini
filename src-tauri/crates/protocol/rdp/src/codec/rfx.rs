use rayon::prelude::*;
use crate::codec::color;
use crate::codec::rfx_dwt;
use crate::codec::rfx_rlgr;
use crate::codec::region::Region;
use crate::gdi::framebuffer::Rect;
use crate::core::stream::ReadCursor;
use crate::codec::bitmap::BitmapRect;
use crate::codec::rfx_rlgr::RlgrMode;
use crate::codec::rfx_dwt::TILE_COEFFICIENTS;
use crate::core::error::{RdpProtocolError, RdpResult};
use crate::codec::rfx_quantization::{self, QUANT_VALUES};

const WF_MAGIC: u32 = 0xCACC_ACCA;
const WF_VERSION_1_0: u16 = 0x0100;

const WBT_SYNC: u16 = 0xCCC0;
const WBT_CODEC_VERSIONS: u16 = 0xCCC1;
const WBT_CHANNELS: u16 = 0xCCC2;
const WBT_CONTEXT: u16 = 0xCCC3;
const WBT_FRAME_BEGIN: u16 = 0xCCC4;
const WBT_FRAME_END: u16 = 0xCCC5;
const WBT_REGION: u16 = 0xCCC6;
const WBT_EXTENSION: u16 = 0xCCC7;
const CBT_REGION: u16 = 0xCAC1;
const CBT_TILESET: u16 = 0xCAC2;
const CBT_TILE: u16 = 0xCAC3;

const CLW_ENTROPY_RLGR1: u16 = 0x01;
const CLW_ENTROPY_RLGR3: u16 = 0x04;

const BLOCK_HEADER_LENGTH: usize = 6;
const CODEC_CHANNEL_LENGTH: usize = 2;
const TILE_HEADER_LENGTH: usize = 13;
const TILE_SIZE: usize = 64;
const TILE_PIXELS: usize = TILE_SIZE * TILE_SIZE;
const PARALLEL_TILE_THRESHOLD: usize = 4;

const DECODED_SYNC: u8 = 0x01;
const DECODED_CONTEXT: u8 = 0x02;
const DECODED_VERSIONS: u8 = 0x04;
const DECODED_CHANNELS: u8 = 0x08;
const DECODED_HEADERS: u8 = DECODED_SYNC | DECODED_CONTEXT | DECODED_VERSIONS | DECODED_CHANNELS;

pub struct RfxContext {
    mode: RlgrMode,
    width: u16,
    height: u16,
    expected_block: u16,
    decoded_headers: u8,
}

impl Default for RfxContext {
    fn default() -> Self {
        Self { mode: RlgrMode::Rlgr1, width: 0, height: 0, expected_block: WBT_FRAME_BEGIN, decoded_headers: 0 }
    }
}

struct Tile {
    left: u16,
    top: u16,
    pixels: Vec<u8>,
}

pub fn decode(context: &mut RfxContext, source: &[u8], left: u16, top: u16, surface: Rect) -> RdpResult<Vec<BitmapRect>> {
    let mut cursor = ReadCursor::new(source);
    let mut clipping = Region::default();
    let mut tiles = Vec::new();

    while cursor.remaining() > BLOCK_HEADER_LENGTH {
        let block_type = cursor.read_u16_le()?;
        let block_length = cursor.read_u32_le()? as usize;
        if block_length < BLOCK_HEADER_LENGTH {
            return Err(RdpProtocolError::protocol(format!("RemoteFX block length too small: {}", block_length)));
        }
        if block_type > WBT_CONTEXT && context.decoded_headers != DECODED_HEADERS {
            return Err(RdpProtocolError::protocol("RemoteFX header blocks not yet complete".to_string()));
        }

        let mut extra = 0usize;
        if (WBT_CONTEXT..=WBT_EXTENSION).contains(&block_type) {
            let codec_id = cursor.read_u8()?;
            let channel_id = cursor.read_u8()?;
            extra = CODEC_CHANNEL_LENGTH;
            if codec_id != 0x01 {
                return Err(RdpProtocolError::protocol(format!("RemoteFX invalid codecId: 0x{:02x}", codec_id)));
            }
            let expected_channel = if block_type == WBT_CONTEXT { 0xFF } else { 0x00 };
            if channel_id != expected_channel {
                return Err(RdpProtocolError::protocol(format!("RemoteFX invalid channelId: 0x{:02x} block type 0x{:04x}", channel_id, block_type)));
            }
        }

        let body_length = block_length - BLOCK_HEADER_LENGTH;
        if body_length < extra {
            return Err(RdpProtocolError::protocol(format!("RemoteFX block length too small: {} requires at least {}", block_length, BLOCK_HEADER_LENGTH + extra)));
        }
        let mut body = cursor.sub_cursor(body_length - extra)?;

        match block_type {
            WBT_SYNC => read_sync(context, &mut body)?,
            WBT_CODEC_VERSIONS => read_codec_versions(context, &mut body)?,
            WBT_CHANNELS => read_channels(context, &mut body)?,
            WBT_CONTEXT => read_context(context, &mut body)?,
            WBT_FRAME_BEGIN => read_frame_begin(context, &mut body)?,
            WBT_REGION => clipping = read_region(context, &mut body, left, top, surface)?,
            WBT_EXTENSION => tiles = read_tileset(context, &mut body)?,
            WBT_FRAME_END => read_frame_end(context)?,
            other => return Err(RdpProtocolError::protocol(format!("unknown RemoteFX block type: 0x{:04x}", other))),
        }
    }

    let mut rectangles = Vec::new();
    for tile in tiles {
        let origin_x = left.saturating_add(tile.left);
        let origin_y = top.saturating_add(tile.top);
        let bounds = Rect::from_size(origin_x, origin_y, TILE_SIZE as u16, TILE_SIZE as u16)?;
        for target in clipping.intersect_rect(&bounds).rects() {
            rectangles.push(crop(&tile, origin_x, origin_y, target));
        }
    }
    Ok(rectangles)
}

fn crop(tile: &Tile, origin_x: u16, origin_y: u16, target: &Rect) -> BitmapRect {
    let width = target.width() as usize;
    let height = target.height() as usize;
    let offset_x = (target.left - origin_x) as usize;
    let offset_y = (target.top - origin_y) as usize;
    let mut rgba = vec![0u8; width * height * 4];
    for row in 0..height {
        let origin = ((offset_y + row) * TILE_SIZE + offset_x) * 4;
        let destination = (height - 1 - row) * width * 4;
        rgba[destination..destination + width * 4].copy_from_slice(&tile.pixels[origin..origin + width * 4]);
    }
    BitmapRect { left: target.left, top: target.top, dest_right: target.right.saturating_sub(1), dest_bottom: target.bottom.saturating_sub(1), width: target.width(), height: target.height(), rgba }
}

fn read_sync(context: &mut RfxContext, cursor: &mut ReadCursor<'_>) -> RdpResult<()> {
    context.decoded_headers &= !DECODED_SYNC;
    let magic = cursor.read_u32_le()?;
    if magic != WF_MAGIC {
        return Err(RdpProtocolError::protocol(format!("RemoteFX invalid magic: 0x{:08x}", magic)));
    }
    let version = cursor.read_u16_le()?;
    if version != WF_VERSION_1_0 {
        return Err(RdpProtocolError::protocol(format!("RemoteFX invalid version: 0x{:04x}", version)));
    }
    context.decoded_headers |= DECODED_SYNC;
    Ok(())
}

fn read_codec_versions(context: &mut RfxContext, cursor: &mut ReadCursor<'_>) -> RdpResult<()> {
    context.decoded_headers &= !DECODED_VERSIONS;
    let count = cursor.read_u8()?;
    let codec_id = cursor.read_u8()?;
    let version = cursor.read_u16_le()?;
    if count != 1 {
        return Err(RdpProtocolError::protocol(format!("RemoteFX invalid codec count: {}", count)));
    }
    if codec_id != 0x01 {
        return Err(RdpProtocolError::protocol(format!("RemoteFX invalid codecId: 0x{:02x}", codec_id)));
    }
    if version != WF_VERSION_1_0 {
        return Err(RdpProtocolError::protocol(format!("RemoteFX invalid codec version: 0x{:04x}", version)));
    }
    context.decoded_headers |= DECODED_VERSIONS;
    Ok(())
}

fn read_channels(context: &mut RfxContext, cursor: &mut ReadCursor<'_>) -> RdpResult<()> {
    context.decoded_headers &= !DECODED_CHANNELS;
    let count = cursor.read_u8()?;
    if count < 1 {
        return Err(RdpProtocolError::protocol("RemoteFX declared no channels".to_string()));
    }
    let channel_id = cursor.read_u8()?;
    if channel_id != 0x00 {
        return Err(RdpProtocolError::protocol(format!("RemoteFX invalid channelId: 0x{:02x}", channel_id)));
    }
    context.width = cursor.read_u16_le()?;
    context.height = cursor.read_u16_le()?;
    if context.width == 0 || context.height == 0 {
        return Err(RdpProtocolError::protocol(format!("RemoteFX invalid channel size: {}x{}", context.width, context.height)));
    }
    cursor.skip(5 * (count as usize - 1))?;
    context.decoded_headers |= DECODED_CHANNELS;
    Ok(())
}

fn read_context(context: &mut RfxContext, cursor: &mut ReadCursor<'_>) -> RdpResult<()> {
    context.decoded_headers &= !DECODED_CONTEXT;
    cursor.skip(3)?;
    let properties = cursor.read_u16_le()?;
    context.mode = match (properties & 0x1E00) >> 9 {
        CLW_ENTROPY_RLGR1 => RlgrMode::Rlgr1,
        CLW_ENTROPY_RLGR3 => RlgrMode::Rlgr3,
        other => return Err(RdpProtocolError::protocol(format!("unknown RemoteFX entropy codec: {}", other))),
    };
    context.decoded_headers |= DECODED_CONTEXT;
    Ok(())
}

fn read_frame_begin(context: &mut RfxContext, cursor: &mut ReadCursor<'_>) -> RdpResult<()> {
    if context.expected_block != WBT_FRAME_BEGIN {
        return Err(RdpProtocolError::protocol("RemoteFX expected frame begin block".to_string()));
    }
    context.expected_block = WBT_REGION;
    cursor.skip(6)?;
    Ok(())
}

fn read_frame_end(context: &mut RfxContext) -> RdpResult<()> {
    if context.expected_block != WBT_FRAME_END {
        return Err(RdpProtocolError::protocol("RemoteFX expected frame end block".to_string()));
    }
    context.expected_block = WBT_FRAME_BEGIN;
    Ok(())
}

fn read_region(context: &mut RfxContext, cursor: &mut ReadCursor<'_>, left: u16, top: u16, surface: Rect) -> RdpResult<Region> {
    if context.expected_block != WBT_REGION {
        return Err(RdpProtocolError::protocol("RemoteFX expected region block".to_string()));
    }
    context.expected_block = WBT_EXTENSION;

    cursor.skip(1)?;
    let count = cursor.read_u16_le()?;
    let mut region = Region::default();
    if count < 1 {
        region.union_rect(clip_rect(left, top, context.width, context.height, surface));
        return Ok(region);
    }

    for _ in 0..count {
        let x = cursor.read_u16_le()?;
        let y = cursor.read_u16_le()?;
        let width = cursor.read_u16_le()?;
        let height = cursor.read_u16_le()?;
        region.union_rect(clip_rect(left.saturating_add(x), top.saturating_add(y), width, height, surface));
    }

    let region_type = cursor.read_u16_le()?;
    let tileset_count = cursor.read_u16_le()?;
    if region_type != CBT_REGION {
        return Err(RdpProtocolError::protocol(format!("RemoteFX invalid region type: 0x{:04x}", region_type)));
    }
    if tileset_count != 0x0001 {
        return Err(RdpProtocolError::protocol(format!("RemoteFX invalid tileset count: {}", tileset_count)));
    }
    Ok(region)
}

fn clip_rect(left: u16, top: u16, width: u16, height: u16, surface: Rect) -> Rect {
    let clipped_left = left.min(surface.right);
    let clipped_top = top.min(surface.bottom);
    Rect { left: clipped_left, top: clipped_top, right: clipped_left.saturating_add(width).min(surface.right), bottom: clipped_top.saturating_add(height).min(surface.bottom) }
}

fn read_tileset(context: &mut RfxContext, cursor: &mut ReadCursor<'_>) -> RdpResult<Vec<Tile>> {
    if context.expected_block != WBT_EXTENSION {
        return Err(RdpProtocolError::protocol("RemoteFX expected tileset block".to_string()));
    }
    context.expected_block = WBT_FRAME_END;

    let subtype = cursor.read_u16_le()?;
    if subtype != CBT_TILESET {
        return Err(RdpProtocolError::protocol(format!("RemoteFX invalid tileset subtype: 0x{:04x}", subtype)));
    }
    cursor.skip(4)?;
    let quant_count = cursor.read_u8()? as usize;
    cursor.skip(1)?;
    if quant_count < 1 {
        return Err(RdpProtocolError::protocol("RemoteFX provided no quantization tables".to_string()));
    }
    let tile_count = cursor.read_u16_le()? as usize;
    if tile_count < 1 {
        return Ok(Vec::new());
    }
    cursor.read_u32_le()?;

    let mut quants = Vec::with_capacity(quant_count);
    for _ in 0..quant_count {
        let raw = cursor.read_slice(QUANT_VALUES / 2)?;
        let mut factors = [0u32; QUANT_VALUES];
        for (index, byte) in raw.iter().enumerate() {
            factors[index * 2] = u32::from(byte & 0x0F);
            factors[index * 2 + 1] = u32::from(byte >> 4);
        }
        quants.push(factors);
    }

    let mut inputs = Vec::with_capacity(tile_count);
    for _ in 0..tile_count {
        let block_type = cursor.read_u16_le()?;
        let block_length = cursor.read_u32_le()? as usize;
        if block_type != CBT_TILE {
            return Err(RdpProtocolError::protocol(format!("RemoteFX invalid block type: 0x{:04x} expected CBT_TILE", block_type)));
        }
        if block_length < BLOCK_HEADER_LENGTH + TILE_HEADER_LENGTH {
            return Err(RdpProtocolError::protocol(format!("RemoteFX tile length too small: {}", block_length)));
        }
        let mut body = cursor.sub_cursor(block_length - BLOCK_HEADER_LENGTH)?;

        let quant_y = body.read_u8()? as usize;
        let quant_cb = body.read_u8()? as usize;
        let quant_cr = body.read_u8()? as usize;
        for index in [quant_y, quant_cb, quant_cr] {
            if index >= quant_count {
                return Err(RdpProtocolError::protocol(format!("RemoteFX quantization index out of range: {} >= {}", index, quant_count)));
            }
        }
        let column = body.read_u16_le()?;
        let row = body.read_u16_le()?;
        let luma_length = body.read_u16_le()? as usize;
        let blue_length = body.read_u16_le()? as usize;
        let red_length = body.read_u16_le()? as usize;
        let luma = body.read_slice(luma_length)?;
        let blue = body.read_slice(blue_length)?;
        let red = body.read_slice(red_length)?;
        inputs.push(TileInput { left: column.saturating_mul(TILE_SIZE as u16), top: row.saturating_mul(TILE_SIZE as u16), luma, blue, red, quant_y, quant_cb, quant_cr });
    }

    let mode = context.mode;
    if inputs.len() < PARALLEL_TILE_THRESHOLD {
        return inputs.iter().map(|input| decode_tile_pixels(input, mode, &quants)).collect();
    }
    inputs.par_iter().map(|input| decode_tile_pixels(input, mode, &quants)).collect()
}

struct TileInput<'a> {
    left: u16,
    top: u16,
    luma: &'a [u8],
    blue: &'a [u8],
    red: &'a [u8],
    quant_y: usize,
    quant_cb: usize,
    quant_cr: usize,
}

fn decode_tile_pixels(input: &TileInput<'_>, mode: RlgrMode, quants: &[[u32; QUANT_VALUES]]) -> RdpResult<Tile> {
    let mut scratch = [0i16; TILE_COEFFICIENTS];
    let mut planes = [[0i16; TILE_COEFFICIENTS]; 3];
    for (plane, (data, quant)) in [(input.luma, input.quant_y), (input.blue, input.quant_cb), (input.red, input.quant_cr)].into_iter().enumerate() {
        rfx_rlgr::decode(mode, data, &mut planes[plane])?;
        differential_decode(&mut planes[plane]);
        rfx_quantization::decode(&mut planes[plane], &quants[quant])?;
        rfx_dwt::decode_2d(&mut planes[plane], &mut scratch);
    }
    let mut pixels = vec![0u8; TILE_PIXELS * 4];
    for index in 0..TILE_PIXELS {
        let rgba = color::ycbcr_to_rgba(planes[0][index], planes[1][index], planes[2][index]);
        pixels[index * 4..index * 4 + 4].copy_from_slice(&rgba);
    }
    Ok(Tile { left: input.left, top: input.top, pixels })
}

fn differential_decode(coefficients: &mut [i16; TILE_COEFFICIENTS]) {
    for index in 4033..TILE_COEFFICIENTS {
        coefficients[index] = (i32::from(coefficients[index - 1]) + i32::from(coefficients[index])) as i16;
    }
}
