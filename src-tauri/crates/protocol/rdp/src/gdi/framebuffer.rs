use bytes::Bytes;
use crate::gdi::raster;
use crate::codec::{jpeg, png};
use crate::codec::region::Region;
use crate::codec::nv12::Nv12Image;
use crate::codec::bitmap::BitmapRect;
use crate::gdi::bitmap_cache::CachedBitmap;
use crate::core::error::{RdpProtocolError, RdpResult};

pub const FRAME_KIND_RAW: u8 = 0;
pub const FRAME_KIND_JPEG: u8 = 1;
pub const FRAME_KIND_PNG: u8 = 2;
pub const FRAME_KIND_WEBP: u8 = 3;
pub const FRAME_KIND_H264: u8 = 4;
pub const FRAME_KIND_AVC444: u8 = 5;

const BYTES_PER_PIXEL: usize = 4;
const TILE_SIZE: usize = 8;
const TILE_PIXELS: usize = TILE_SIZE * TILE_SIZE;
const JPEG_MIN_BITMAP_SIZE: u32 = 4096;
const JPEG_BLOCK_SIZE: u16 = 16;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Rect {
    pub left: u16,
    pub top: u16,
    pub right: u16,
    pub bottom: u16,
}

impl Rect {
    pub fn new(left: u16, top: u16, right: u16, bottom: u16) -> RdpResult<Self> {
        if right < left || bottom < top {
            return Err(RdpProtocolError::protocol(format!("invalid rectangle coordinates: ({},{})-({},{})", left, top, right, bottom)));
        }
        Ok(Self { left, top, right, bottom })
    }

    pub fn from_size(left: u16, top: u16, width: u16, height: u16) -> RdpResult<Self> {
        let right = left.checked_add(width).ok_or_else(|| RdpProtocolError::protocol(format!("rectangle width overflow: {}+{}", left, width)))?;
        let bottom = top.checked_add(height).ok_or_else(|| RdpProtocolError::protocol(format!("rectangle height overflow: {}+{}", top, height)))?;
        Self::new(left, top, right, bottom)
    }

    pub fn width(&self) -> u16 {
        self.right - self.left
    }

    pub fn height(&self) -> u16 {
        self.bottom - self.top
    }

    pub fn is_empty(&self) -> bool {
        self.right == self.left || self.bottom == self.top
    }

    pub fn union(&self, other: &Rect) -> Rect {
        Rect { left: self.left.min(other.left), top: self.top.min(other.top), right: self.right.max(other.right), bottom: self.bottom.max(other.bottom) }
    }

    pub fn intersect(&self, other: &Rect) -> Rect {
        let left = self.left.max(other.left);
        let top = self.top.max(other.top);
        Rect { left, top, right: self.right.min(other.right).max(left), bottom: self.bottom.min(other.bottom).max(top) }
    }
}

pub enum Pattern {
    Solid(u32),
    Tile { pixels: [u32; TILE_PIXELS], origin_x: u8, origin_y: u8 },
}

impl Pattern {
    fn sample(&self, x: u16, y: u16) -> u32 {
        match self {
            Pattern::Solid(color) => *color,
            Pattern::Tile { pixels, origin_x, origin_y } => {
                let column = (x as usize + TILE_SIZE - *origin_x as usize % TILE_SIZE) % TILE_SIZE;
                let row = (y as usize + TILE_SIZE - *origin_y as usize % TILE_SIZE) % TILE_SIZE;
                pixels[row * TILE_SIZE + column]
            }
        }
    }
}

fn bounding_rect(from: (i32, i32), to: (i32, i32), surface: Rect) -> Rect {
    let left = from.0.min(to.0).clamp(0, surface.right as i32) as u16;
    let top = from.1.min(to.1).clamp(0, surface.bottom as i32) as u16;
    let right = (from.0.max(to.0) + 1).clamp(0, surface.right as i32) as u16;
    let bottom = (from.1.max(to.1) + 1).clamp(0, surface.bottom as i32) as u16;
    Rect { left, top, right: right.max(left), bottom: bottom.max(top) }
}

pub struct Framebuffer {
    width: u16,
    height: u16,
    dirty: Region,
    nv12: Nv12Image,
    pixels: Vec<u8>,
}

impl Framebuffer {
    pub fn new(width: u16, height: u16) -> Self {
        Self { width, height, pixels: vec![0u8; width as usize * height as usize * BYTES_PER_PIXEL], nv12: Nv12Image::new(width, height), dirty: Region::default() }
    }

    pub fn stride(&self) -> usize {
        self.width as usize * BYTES_PER_PIXEL
    }

    pub fn resize(&mut self, width: u16, height: u16) -> bool {
        if self.width == width && self.height == height {
            return false;
        }
        self.width = width;
        self.height = height;
        self.pixels = vec![0u8; width as usize * height as usize * BYTES_PER_PIXEL];
        self.nv12 = Nv12Image::new(width, height);
        self.dirty = Region::default();
        true
    }

    pub fn invalidate(&mut self, rect: Rect) {
        if rect.is_empty() {
            return;
        }
        self.dirty.union_rect(rect);
    }

    pub fn dirty_len(&self) -> usize {
        self.dirty.rects().len()
    }

    pub fn clear_dirty(&mut self) {
        self.dirty = Region::default();
    }

    pub fn blit_bitmap(&mut self, bitmap: &BitmapRect) -> RdpResult<()> {
        if bitmap.dest_right < bitmap.left || bitmap.dest_bottom < bitmap.top {
            return Ok(());
        }
        let dest = Rect::new(bitmap.left, bitmap.top, bitmap.dest_right.saturating_add(1), bitmap.dest_bottom.saturating_add(1))?.intersect(&self.surface());
        if dest.is_empty() {
            return Ok(());
        }
        let source_stride = bitmap.width as usize * BYTES_PER_PIXEL;
        let required = source_stride * bitmap.height as usize;
        if bitmap.rgba.len() != required {
            return Err(RdpProtocolError::protocol(format!("bitmap pixel length mismatch: need {} have {}", required, bitmap.rgba.len())));
        }
        let source_left = (dest.left - bitmap.left) as usize;
        let source_top = (dest.top - bitmap.top) as usize;
        let copy_width = (dest.width() as usize).min(bitmap.width as usize - source_left);
        let copy_height = (dest.height() as usize).min(bitmap.height as usize - source_top);
        if copy_width == 0 || copy_height == 0 {
            return Ok(());
        }
        let stride = self.stride();
        let copy_bytes = copy_width * BYTES_PER_PIXEL;
        for row in 0..copy_height {
            let target = (dest.top as usize + row) * stride + dest.left as usize * BYTES_PER_PIXEL;
            let origin = (bitmap.height as usize - 1 - (source_top + row)) * source_stride + source_left * BYTES_PER_PIXEL;
            self.pixels[target..target + copy_bytes].copy_from_slice(&bitmap.rgba[origin..origin + copy_bytes]);
        }
        let painted = Rect::from_size(dest.left, dest.top, copy_width as u16, copy_height as u16)?;
        self.invalidate(painted);
        self.sync_nv12(painted);
        Ok(())
    }

    pub fn blit_surface(&mut self, rect: Rect, source: &[u8], source_stride: usize, source_left: u16, source_top: u16) -> RdpResult<()> {
        let dest = rect.intersect(&self.surface());
        if dest.is_empty() {
            return Ok(());
        }
        let source_left = source_left + (dest.left - rect.left);
        let source_top = source_top + (dest.top - rect.top);
        let stride = self.stride();
        let width = dest.width() as usize * BYTES_PER_PIXEL;
        for row in 0..dest.height() as usize {
            let origin = (source_top as usize + row) * source_stride + source_left as usize * BYTES_PER_PIXEL;
            let target = (dest.top as usize + row) * stride + dest.left as usize * BYTES_PER_PIXEL;
            self.pixels[target..target + width].copy_from_slice(&source[origin..origin + width]);
        }
        self.invalidate(dest);
        Ok(())
    }

    pub fn blit_nv12(&mut self, rect: Rect, source: &Nv12Image, source_left: u16, source_top: u16) {
        self.nv12.blit_from(source, rect, source_left, source_top);
    }

    fn sync_nv12(&mut self, rect: Rect) {
        self.nv12.blit_rgba(rect, &self.pixels, self.stride(), rect.left, rect.top);
    }

    pub fn copy_nv12(&self) -> (Vec<u8>, Vec<u8>) {
        self.nv12.copy_planes()
    }

    pub fn copy_nv12_aligned(&self, align: u16) -> (Vec<u8>, Vec<u8>, u16, u16) {
        self.nv12.copy_planes_aligned(align)
    }

    pub fn surface(&self) -> Rect {
        Rect { left: 0, top: 0, right: self.width, bottom: self.height }
    }

    fn read_pixel(&self, x: u16, y: u16) -> u32 {
        let offset = y as usize * self.stride() + x as usize * BYTES_PER_PIXEL;
        u32::from_le_bytes([self.pixels[offset], self.pixels[offset + 1], self.pixels[offset + 2], self.pixels[offset + 3]])
    }

    fn write_pixel(&mut self, x: u16, y: u16, value: u32) {
        let offset = y as usize * self.stride() + x as usize * BYTES_PER_PIXEL;
        self.pixels[offset..offset + BYTES_PER_PIXEL].copy_from_slice(&value.to_le_bytes());
    }

    pub fn blt_pattern(&mut self, rect: Rect, pattern: &Pattern, rop: u8) -> RdpResult<()> {
        self.blt_raster(rect, pattern, rop, None)
    }

    pub fn blt_pattern_from(&mut self, rect: Rect, pattern: &Pattern, rop: u8, source_left: u16, source_top: u16) -> RdpResult<()> {
        self.blt_raster(rect, pattern, rop, Some((source_left, source_top)))
    }

    fn blt_raster(&mut self, rect: Rect, pattern: &Pattern, rop: u8, origin: Option<(u16, u16)>) -> RdpResult<()> {
        let rect = rect.intersect(&self.surface());
        if rect.is_empty() {
            return Ok(());
        }
        let (rect, source) = match raster::uses_source(rop) {
            false => (rect, None),
            true => {
                let (left, top) = origin.ok_or_else(|| RdpProtocolError::protocol(format!("raster operation 0x{:02x} requires a source operand but the drawing order provided none", rop)))?;
                let source = Rect::from_size(left, top, rect.width(), rect.height())?;
                let source = source.intersect(&self.surface());
                if source.is_empty() {
                    return Ok(());
                }
                let dest = Rect::from_size(rect.left + (source.left - left), rect.top + (source.top - top), source.width(), source.height())?;
                (dest, Some(source))
            }
        };
        for y in rect.top..rect.bottom {
            for x in rect.left..rect.right {
                let sample = match source {
                    Some(source) => self.read_pixel(source.left + (x - rect.left), source.top + (y - rect.top)),
                    None => 0,
                };
                let value = raster::rop3(rop, pattern.sample(x, y), sample, self.read_pixel(x, y));
                self.write_pixel(x, y, value);
            }
        }
        self.invalidate(rect);
        self.sync_nv12(rect);
        Ok(())
    }

    pub fn blt_copy(&mut self, rect: Rect, source_left: u16, source_top: u16, rop: u8) -> RdpResult<()> {
        if rect.is_empty() {
            return Ok(());
        }
        let dest = rect.intersect(&self.surface());
        if dest.is_empty() {
            return Ok(());
        }
        let source_left = source_left + (dest.left - rect.left);
        let source_top = source_top + (dest.top - rect.top);
        let source = Rect::from_size(source_left, source_top, dest.width(), dest.height())?;
        let source = source.intersect(&self.surface());
        if source.is_empty() {
            return Ok(());
        }
        let dest = Rect::from_size(dest.left + (source.left - source_left), dest.top + (source.top - source_top), source.width(), source.height())?;
        let rows: Vec<u16> = if dest.top > source.top { (dest.top..dest.bottom).rev().collect() } else { (dest.top..dest.bottom).collect() };
        let columns: Vec<u16> = if dest.left > source.left { (dest.left..dest.right).rev().collect() } else { (dest.left..dest.right).collect() };
        for y in rows {
            for &x in &columns {
                let origin_x = source.left + (x - dest.left);
                let origin_y = source.top + (y - dest.top);
                let value = raster::rop3(rop, 0, self.read_pixel(origin_x, origin_y), self.read_pixel(x, y));
                self.write_pixel(x, y, value);
            }
        }
        self.invalidate(dest);
        self.sync_nv12(dest);
        Ok(())
    }

    pub fn blt_bitmap_rop(&mut self, rect: Rect, source: &CachedBitmap, source_left: u16, source_top: u16, pattern: &Pattern, rop: u8) -> RdpResult<()> {
        if rect.is_empty() {
            return Ok(());
        }
        let rect = rect.intersect(&self.surface());
        if rect.is_empty() {
            return Ok(());
        }
        if source_left >= source.width || source_top >= source.height {
            return Ok(());
        }
        let width = rect.width().min(source.width - source_left);
        let height = rect.height().min(source.height - source_top);
        let rect = Rect::from_size(rect.left, rect.top, width, height)?;
        let source_rect = Rect::from_size(source_left, source_top, width, height)?;
        for y in rect.top..rect.bottom {
            for x in rect.left..rect.right {
                let sample = source.pixel(source_rect.left + (x - rect.left), source_rect.top + (y - rect.top));
                let value = raster::rop3(rop, pattern.sample(x, y), sample, self.read_pixel(x, y));
                self.write_pixel(x, y, value);
            }
        }
        self.invalidate(rect);
        self.sync_nv12(rect);
        Ok(())
    }

    pub fn blit_mono(&mut self, dest_x: i32, dest_y: i32, width: i32, height: i32, source_x: i32, source_y: i32, bits: &[u8], stride: usize, color: u32, clip: Rect) -> RdpResult<()> {
        if width <= 0 || height <= 0 {
            return Ok(());
        }
        let mut touched = false;
        for row in 0..height {
            let dest_row = dest_y + row;
            let source_row = source_y + row;
            if dest_row < clip.top as i32 || dest_row >= clip.bottom as i32 || source_row < 0 {
                continue;
            }
            for column in 0..width {
                let dest_column = dest_x + column;
                let source_column = source_x + column;
                if dest_column < clip.left as i32 || dest_column >= clip.right as i32 || source_column < 0 {
                    continue;
                }
                let offset = source_row as usize * stride + source_column as usize / 8;
                if offset >= bits.len() {
                    return Err(RdpProtocolError::underrun(format!("glyph sample out of bounds: offset={} len={}", offset, bits.len())));
                }
                if bits[offset] & (0x80 >> (source_column as usize % 8)) == 0 {
                    continue;
                }
                self.write_pixel(dest_column as u16, dest_row as u16, color);
                touched = true;
            }
        }
        if touched {
            let rect = Rect {
                left: dest_x.max(clip.left as i32) as u16,
                top: dest_y.max(clip.top as i32) as u16,
                right: (dest_x + width).min(clip.right as i32) as u16,
                bottom: (dest_y + height).min(clip.bottom as i32) as u16,
            };
            if !rect.is_empty() {
                self.invalidate(rect);
                self.sync_nv12(rect);
            }
        }
        Ok(())
    }

    pub fn draw_line(&mut self, from: (i32, i32), to: (i32, i32), clip: Rect, color: u32, rop: u8) -> RdpResult<()> {
        let delta_x = (to.0 - from.0).abs();
        let delta_y = -(to.1 - from.1).abs();
        let step_x = if from.0 < to.0 { 1 } else { -1 };
        let step_y = if from.1 < to.1 { 1 } else { -1 };
        let mut error = delta_x + delta_y;
        let (mut x, mut y) = from;
        let mut touched = false;
        loop {
            if x >= clip.left as i32 && x < clip.right as i32 && y >= clip.top as i32 && y < clip.bottom as i32 {
                let value = raster::rop2(rop, color, self.read_pixel(x as u16, y as u16))?;
                self.write_pixel(x as u16, y as u16, value);
                touched = true;
            }
            if x == to.0 && y == to.1 {
                break;
            }
            let doubled = error * 2;
            if doubled >= delta_y {
                error += delta_y;
                x += step_x;
            }
            if doubled <= delta_x {
                error += delta_x;
                y += step_y;
            }
        }
        if touched {
            let rect = clip.intersect(&bounding_rect(from, to, self.surface()));
            self.invalidate(rect);
            self.sync_nv12(rect);
        }
        Ok(())
    }

    pub fn take_dirty_extents(&mut self) -> Option<Rect> {
        let region = std::mem::take(&mut self.dirty);
        if region.rects().is_empty() {
            return None;
        }
        let extents = region.extents();
        if extents.is_empty() {
            return None;
        }
        Some(extents)
    }

    pub fn copy_rgba_rect(&self, rect: &Rect) -> Vec<u8> {
        self.copy_rgba(rect)
    }

    pub fn encode_rgba_patches(patches: &[(Rect, Vec<u8>)], quality: u8) -> RdpResult<Vec<u8>> {
        let mut body = Vec::new();
        for (rect, rgba) in patches {
            let expected = rect.width() as usize * rect.height() as usize * BYTES_PER_PIXEL;
            if rgba.len() != expected {
                return Err(RdpProtocolError::protocol(format!("dirty region pixel length mismatch: need {} have {}", expected, rgba.len())));
            }
            let area = u32::from(rect.width()) * u32::from(rect.height());
            let (kind, bytes) = if area >= JPEG_MIN_BITMAP_SIZE {
                (FRAME_KIND_JPEG, jpeg::encode(rgba, rect.width(), rect.height(), quality)?)
            } else {
                (FRAME_KIND_PNG, png::encode(rgba, rect.width(), rect.height())?)
            };
            let data_len = u32::try_from(bytes.len()).map_err(|_| RdpProtocolError::encoding(format!("frame rectangle too large: {}", bytes.len())))?;
            body.push(kind);
            body.extend_from_slice(&rect.left.to_le_bytes());
            body.extend_from_slice(&rect.top.to_le_bytes());
            body.extend_from_slice(&rect.width().to_le_bytes());
            body.extend_from_slice(&rect.height().to_le_bytes());
            body.extend_from_slice(&data_len.to_le_bytes());
            body.extend_from_slice(&bytes);
        }
        Ok(body)
    }

    pub fn encode_lossless_patch(rect: Rect, rgba: &[u8]) -> RdpResult<Vec<u8>> {
        let expected = rect.width() as usize * rect.height() as usize * BYTES_PER_PIXEL;
        if rgba.len() != expected {
            return Err(RdpProtocolError::protocol(format!("focus region pixel length mismatch: need {} have {}", expected, rgba.len())));
        }
        let bytes = png::encode_fast(rgba, rect.width(), rect.height())?;
        Self::pack_patch(FRAME_KIND_PNG, rect, &bytes)
    }

    pub fn pack_patch(kind: u8, rect: Rect, bytes: &[u8]) -> RdpResult<Vec<u8>> {
        let data_len = u32::try_from(bytes.len()).map_err(|_| RdpProtocolError::encoding(format!("frame rectangle too large: {}", bytes.len())))?;
        let mut body = Vec::with_capacity(13 + bytes.len());
        body.push(kind);
        body.extend_from_slice(&rect.left.to_le_bytes());
        body.extend_from_slice(&rect.top.to_le_bytes());
        body.extend_from_slice(&rect.width().to_le_bytes());
        body.extend_from_slice(&rect.height().to_le_bytes());
        body.extend_from_slice(&data_len.to_le_bytes());
        body.extend_from_slice(bytes);
        Ok(body)
    }

    pub fn take_frame(&mut self, quality: u8) -> RdpResult<Option<Bytes>> {
        let region = std::mem::take(&mut self.dirty);
        if region.rects().is_empty() {
            return Ok(None);
        }
        let extents = region.extents();
        let box_area = u32::from(extents.width()) * u32::from(extents.height());
        let dirty_area: u32 = region.rects().iter().map(|rect| u32::from(rect.width()) * u32::from(rect.height())).sum();
        let rects: Vec<Rect> = if region.rects().len() > 1 && box_area > 0 && dirty_area.saturating_mul(100) / box_area >= 40 { vec![extents] } else { region.rects().to_vec() };
        let mut body = Vec::new();
        for rect in rects {
            if rect.is_empty() {
                continue;
            }
            let area = u32::from(rect.width()) * u32::from(rect.height());
            let (kind, rect, bytes) = if area >= JPEG_MIN_BITMAP_SIZE {
                let expanded = self.expand_to_grid(rect, JPEG_BLOCK_SIZE);
                let rgba = self.copy_rgba(&expanded);
                (FRAME_KIND_JPEG, expanded, jpeg::encode(&rgba, expanded.width(), expanded.height(), quality)?)
            } else {
                let rgba = self.copy_rgba(&rect);
                (FRAME_KIND_PNG, rect, png::encode(&rgba, rect.width(), rect.height())?)
            };
            let data_len = u32::try_from(bytes.len()).map_err(|_| RdpProtocolError::encoding(format!("frame rectangle too large: {}", bytes.len())))?;
            body.push(kind);
            body.extend_from_slice(&rect.left.to_le_bytes());
            body.extend_from_slice(&rect.top.to_le_bytes());
            body.extend_from_slice(&rect.width().to_le_bytes());
            body.extend_from_slice(&rect.height().to_le_bytes());
            body.extend_from_slice(&data_len.to_le_bytes());
            body.extend_from_slice(&bytes);
        }
        Ok(Some(Bytes::from(body)))
    }

    fn copy_rgba(&self, rect: &Rect) -> Vec<u8> {
        let width = rect.width() as usize * BYTES_PER_PIXEL;
        let mut output = vec![0u8; width * rect.height() as usize];
        let stride = self.stride();
        for row in 0..rect.height() as usize {
            let origin = (rect.top as usize + row) * stride + rect.left as usize * BYTES_PER_PIXEL;
            let target = row * width;
            output[target..target + width].copy_from_slice(&self.pixels[origin..origin + width]);
        }
        output
    }

    fn expand_to_grid(&self, rect: Rect, block: u16) -> Rect {
        let block = u32::from(block);
        let left = u32::from(rect.left) / block * block;
        let top = u32::from(rect.top) / block * block;
        let right = ((u32::from(rect.right) + block - 1) / block * block).min(u32::from(self.width));
        let bottom = ((u32::from(rect.bottom) + block - 1) / block * block).min(u32::from(self.height));
        Rect { left: left as u16, top: top as u16, right: right.max(left) as u16, bottom: bottom.max(top) as u16 }
    }
}

pub use super::frame_quality::{network_bars, smooth_frame_duration, suggest_bitrate, suggest_quality};
