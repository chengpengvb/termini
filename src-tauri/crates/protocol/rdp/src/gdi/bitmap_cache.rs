use crate::codec::nsc;
use crate::codec::rfx;
use crate::codec::jpeg;
use crate::core::orders;
use crate::codec::bitmap;
use std::collections::HashMap;
use crate::codec::color::Palette;
use crate::codec::rfx::RfxContext;
use crate::gdi::framebuffer::Rect;
use crate::core::orders::CacheBitmap;
use crate::core::error::{RdpProtocolError, RdpResult};

pub const CELL_ENTRIES: [u16; 5] = [600, 600, 2048, 4096, 2048];

const BYTES_PER_PIXEL: usize = 4;
const CODEC_ID_NONE: u8 = 0x00;
const CODEC_ID_NSCODEC: u8 = 0x01;
const CODEC_ID_JPEG: u8 = 0x02;
const CODEC_ID_REMOTEFX: u8 = 0x03;
const CODEC_ID_IMAGE_REMOTEFX: u8 = 0x04;

pub struct CachedBitmap {
    pub width: u16,
    pub height: u16,
    pub persist_key: Option<(u32, u32)>,
    pixels: Vec<u8>,
}

impl CachedBitmap {
    pub fn pixel(&self, x: u16, y: u16) -> u32 {
        let offset = (y as usize * self.width as usize + x as usize) * BYTES_PER_PIXEL;
        u32::from_le_bytes([self.pixels[offset], self.pixels[offset + 1], self.pixels[offset + 2], self.pixels[offset + 3]])
    }
}

#[derive(Default)]
pub struct BitmapCache {
    entries: HashMap<(u16, u16), CachedBitmap>,
}

impl BitmapCache {
    pub fn store(&mut self, order: &CacheBitmap, palette: &Palette, remotefx: &mut RfxContext) -> RdpResult<()> {
        let limit = CELL_ENTRIES.get(order.cache_id as usize).ok_or_else(|| RdpProtocolError::protocol(format!("bitmap cache cacheId out of range: {}", order.cache_id)))?;
        if order.cache_index != orders::BITMAP_CACHE_WAITING_LIST_INDEX && order.cache_index >= *limit {
            return Err(RdpProtocolError::protocol(format!("bitmap cache cacheIndex out of range: cell {} index {} limit {}", order.cache_id, order.cache_index, limit)));
        }
        if order.width == 0 || order.height == 0 {
            return Err(RdpProtocolError::protocol(format!("cached bitmap size is zero: {}x{}", order.width, order.height)));
        }
        let width = order.width as usize;
        let height = order.height as usize;
        let decoded = decode_cache_bitmap(order, palette, remotefx)?;
        let stride = width * BYTES_PER_PIXEL;
        if decoded.len() != stride * height {
            return Err(RdpProtocolError::protocol(format!("cached bitmap pixel length mismatch: need {} have {}", stride * height, decoded.len())));
        }
        let mut pixels = vec![0u8; stride * height];
        for row in 0..height {
            let origin = (height - 1 - row) * stride;
            pixels[row * stride..(row + 1) * stride].copy_from_slice(&decoded[origin..origin + stride]);
        }
        self.entries.insert((order.cache_id, order.cache_index), CachedBitmap { width: order.width, height: order.height, persist_key: order.persist_key, pixels });
        Ok(())
    }

    pub fn get(&self, cache_id: u16, cache_index: u16) -> Option<&CachedBitmap> {
        self.entries.get(&(cache_id, cache_index))
    }

    pub fn persist_cells(&self) -> [Vec<u64>; 5] {
        let mut cells: [Vec<(u16, u64)>; 5] = std::array::from_fn(|_| Vec::new());
        for ((cache_id, cache_index), bitmap) in &self.entries {
            let Some((key1, key2)) = bitmap.persist_key else {
                continue;
            };
            cells[*cache_id as usize].push((*cache_index, u64::from(key1) | (u64::from(key2) << 32)));
        }
        cells.map(|mut cell| {
            cell.sort_by_key(|(index, _)| *index);
            cell.into_iter().map(|(_, key)| key).collect()
        })
    }
}

fn decode_cache_bitmap(order: &CacheBitmap, palette: &Palette, remotefx: &mut RfxContext) -> RdpResult<Vec<u8>> {
    let width = order.width as usize;
    let height = order.height as usize;
    let Some(codec_id) = order.codec_id else {
        return bitmap::decode(&order.data, width, height, order.bits_per_pixel, order.compressed, palette);
    };
    match codec_id {
        CODEC_ID_NONE => bitmap::decode(&order.data, width, height, order.bits_per_pixel, false, palette),
        CODEC_ID_NSCODEC => nsc::decode(&order.data, width, height),
        CODEC_ID_JPEG => jpeg::decode(&order.data, width, height),
        CODEC_ID_REMOTEFX | CODEC_ID_IMAGE_REMOTEFX => decode_rfx_cache(remotefx, &order.data, order.width, order.height),
        other => Err(RdpProtocolError::unsupported(format!("cached bitmap uses an unnegotiated codec: {}", other))),
    }
}

fn decode_rfx_cache(remotefx: &mut RfxContext, data: &[u8], width: u16, height: u16) -> RdpResult<Vec<u8>> {
    let surface = Rect::from_size(0, 0, width, height)?;
    let rects = rfx::decode(remotefx, data, 0, 0, surface)?;
    if rects.is_empty() {
        return Err(RdpProtocolError::protocol("cached bitmap RemoteFX decoded no rectangles".to_string()));
    }
    let stride = width as usize * BYTES_PER_PIXEL;
    let mut decoded = vec![0u8; stride * height as usize];
    for rect in rects {
        let source_stride = rect.width as usize * BYTES_PER_PIXEL;
        for row in 0..rect.height as usize {
            let origin = (rect.height as usize - 1 - row) * source_stride;
            let dest_y = height as usize - 1 - (rect.top as usize + row);
            let dest = dest_y * stride + rect.left as usize * BYTES_PER_PIXEL;
            decoded[dest..dest + source_stride].copy_from_slice(&rect.rgba[origin..origin + source_stride]);
        }
    }
    Ok(decoded)
}
