use crate::codec::avc;
use crate::codec::rfx;
use crate::codec::clear;
use crate::codec::planar;
use crate::codec::progressive;
use std::collections::HashMap;
use crate::codec::region::Region;
use crate::codec::nv12::Nv12Image;
use crate::codec::rfx::RfxContext;
use crate::core::stream::ReadCursor;
use crate::codec::clear::ClearContext;
use crate::gdi::framebuffer::{Framebuffer, Rect};
use crate::codec::progressive::ProgressiveContext;
use crate::core::error::{RdpProtocolError, RdpResult};

pub const CODEC_UNCOMPRESSED: u16 = 0x0000;
pub const CODEC_REMOTEFX: u16 = 0x0003;
pub const CODEC_CLEARCODEC: u16 = 0x0008;
pub const CODEC_PROGRESSIVE: u16 = 0x0009;
pub const CODEC_PLANAR: u16 = 0x000A;
pub const CODEC_AVC420: u16 = 0x000B;
pub const CODEC_ALPHA: u16 = 0x000C;
pub const CODEC_AVC444: u16 = 0x000E;
pub const CODEC_AVC444V2: u16 = 0x000F;

pub const PIXEL_FORMAT_XRGB: u8 = 0x20;
pub const PIXEL_FORMAT_ARGB: u8 = 0x21;

pub const MAX_SURFACE_SIZE: u32 = 32766;
pub const MAX_MONITOR_COUNT: u32 = 16;

const SURFACE_ALIGNMENT: usize = 16;
const BYTES_PER_PIXEL: usize = 4;
const ALPHA_SIGNATURE: u16 = 0x414C;

pub struct SurfaceCommand<'a> {
    pub surface_id: u16,
    pub codec_id: u16,
    pub alpha: bool,
    pub rect: Rect,
    pub data: &'a [u8],
}

pub struct Point {
    pub x: u16,
    pub y: u16,
}

struct Surface {
    width: usize,
    height: usize,
    mapped_width: u16,
    mapped_height: u16,
    stride: usize,
    pixels: Vec<u8>,
    nv12: Nv12Image,
    invalid: Region,
    alpha: bool,
    output_mapped: bool,
    output_x: u16,
    output_y: u16,
    avc: Option<avc::AvcDecoder>,
}

struct CacheEntry {
    width: u16,
    height: u16,
    stride: usize,
    pixels: Vec<u8>,
}

#[derive(Default)]
pub struct GfxSurfaces {
    frame_id: u32,
    in_frame: bool,
    rfx: RfxContext,
    decoded_frames: u32,
    pub last_codec_id: u16,
    clear: Option<ClearContext>,
    surfaces: HashMap<u16, Surface>,
    cache: HashMap<u16, CacheEntry>,
    progressive: ProgressiveContext,
    pending_avc: Vec<(Rect, Vec<u8>)>,
    pending_h264: Vec<(Rect, Vec<u8>)>,
}

impl Surface {
    fn new(width: u16, height: u16, alpha: bool) -> Self {
        let aligned_width = (width as usize).div_ceil(SURFACE_ALIGNMENT) * SURFACE_ALIGNMENT;
        let aligned_height = (height as usize).div_ceil(SURFACE_ALIGNMENT) * SURFACE_ALIGNMENT;
        let stride = aligned_width * BYTES_PER_PIXEL;
        Self {
            stride,
            alpha,
            output_x: 0,
            output_y: 0,
            mapped_width: width,
            width: aligned_width,
            output_mapped: false,
            mapped_height: height,
            height: aligned_height,
            invalid: Region::default(),
            pixels: vec![0xFFu8; stride * aligned_height],
            nv12: Nv12Image::new(aligned_width as u16, aligned_height as u16),
            avc: None,
        }
    }

    fn bounds(&self) -> Rect {
        Rect { left: 0, top: 0, right: self.width as u16, bottom: self.height as u16 }
    }

    fn validate(&self, rect: &Rect) -> RdpResult<()> {
        if rect.right as usize > self.width || rect.bottom as usize > self.height {
            return Err(RdpProtocolError::protocol(format!(
                "graphics surface region out of bounds: ({},{})-({},{}) exceeds {}x{}",
                rect.left, rect.top, rect.right, rect.bottom, self.width, self.height
            )));
        }
        Ok(())
    }

    fn write_rows(&mut self, rect: &Rect, source: &[u8], source_stride: usize, flip: bool) -> RdpResult<()> {
        self.validate(rect)?;
        let width = rect.width() as usize * BYTES_PER_PIXEL;
        let height = rect.height() as usize;
        if source.len() < source_stride * height {
            return Err(RdpProtocolError::underrun(format!("graphics surface pixel underrun: need {} have {}", source_stride * height, source.len())));
        }
        for row in 0..height {
            let origin = match flip {
                true => (height - 1 - row) * source_stride,
                false => row * source_stride,
            };
            let target = (rect.top as usize + row) * self.stride + rect.left as usize * BYTES_PER_PIXEL;
            self.pixels[target..target + width].copy_from_slice(&source[origin..origin + width]);
        }
        self.nv12.blit_rgba(*rect, &self.pixels, self.stride, rect.left, rect.top);
        self.invalid.union_rect(*rect);
        Ok(())
    }
}

impl GfxSurfaces {
    pub fn reset(&mut self) {
        for surface in self.surfaces.values_mut() {
            surface.pixels.fill(0xFF);
            surface.nv12.fill_rgb(0xFF, 0xFF, 0xFF);
            surface.invalid = Region::default();
            surface.avc = None;
        }
        if let Some(clear) = self.clear.as_mut() {
            clear.reset();
        }
        self.pending_avc.clear();
        self.pending_h264.clear();
    }

    pub fn take_avc_patches(&mut self) -> Vec<(Rect, Vec<u8>)> {
        std::mem::take(&mut self.pending_avc)
    }

    pub fn take_h264_patches(&mut self) -> Vec<(Rect, Vec<u8>)> {
        std::mem::take(&mut self.pending_h264)
    }

    pub fn create(&mut self, surface_id: u16, width: u16, height: u16, pixel_format: u8) -> RdpResult<()> {
        let alpha = match pixel_format {
            PIXEL_FORMAT_XRGB => false,
            PIXEL_FORMAT_ARGB => true,
            other => return Err(RdpProtocolError::protocol(format!("invalid graphics surface pixel format: 0x{:02x}", other))),
        };
        self.delete(surface_id);
        self.surfaces.insert(surface_id, Surface::new(width, height, alpha));
        Ok(())
    }

    pub fn delete(&mut self, surface_id: u16) {
        self.surfaces.remove(&surface_id);
        progressive::delete_surface(&mut self.progressive, surface_id);
    }

    pub fn start_frame(&mut self, frame_id: u32) {
        self.frame_id = frame_id;
        self.in_frame = true;
    }

    pub fn end_frame(&mut self) -> u32 {
        self.in_frame = false;
        self.decoded_frames += 1;
        self.decoded_frames
    }

    pub fn in_frame(&self) -> bool {
        self.in_frame
    }

    pub fn map_to_output(&mut self, surface_id: u16, origin_x: u32, origin_y: u32) -> RdpResult<()> {
        let surface = self.surface_mut(surface_id)?;
        if origin_x > u32::from(u16::MAX) || origin_y > u32::from(u16::MAX) {
            return Err(RdpProtocolError::protocol(format!("graphics surface output origin out of bounds: ({},{})", origin_x, origin_y)));
        }
        surface.output_mapped = true;
        surface.output_x = origin_x as u16;
        surface.output_y = origin_y as u16;
        surface.invalid = Region::default();
        Ok(())
    }

    pub fn solid_fill(&mut self, surface_id: u16, color: [u8; 4], rects: &[Rect]) -> RdpResult<()> {
        let surface = self.surface_mut(surface_id)?;
        for rect in rects {
            if rect.left as usize > surface.width || rect.top as usize > surface.height {
                return Err(RdpProtocolError::protocol(format!("graphics solid fill region out of bounds: ({},{})", rect.left, rect.top)));
            }
            let clipped = rect.intersect(&surface.bounds());
            if clipped.is_empty() {
                continue;
            }
            for row in clipped.top..clipped.bottom {
                for column in clipped.left..clipped.right {
                    let offset = row as usize * surface.stride + column as usize * BYTES_PER_PIXEL;
                    surface.pixels[offset..offset + BYTES_PER_PIXEL].copy_from_slice(&color);
                }
            }
            surface.invalid.union_rect(clipped);
            surface.nv12.blit_rgba(clipped, &surface.pixels, surface.stride, clipped.left, clipped.top);
        }
        Ok(())
    }

    pub fn surface_to_surface(&mut self, source_id: u16, target_id: u16, source_rect: Rect, points: &[Point]) -> RdpResult<()> {
        let source = self.surface(source_id)?;
        source.validate(&source_rect)?;
        let width = source_rect.width();
        let height = source_rect.height();
        let mut rows = Vec::with_capacity(width as usize * height as usize * BYTES_PER_PIXEL);
        for row in 0..height as usize {
            let origin = (source_rect.top as usize + row) * source.stride + source_rect.left as usize * BYTES_PER_PIXEL;
            rows.extend_from_slice(&source.pixels[origin..origin + width as usize * BYTES_PER_PIXEL]);
        }

        let target = self.surface_mut(target_id)?;
        for point in points {
            let rect = Rect::from_size(point.x, point.y, width, height)?;
            target.write_rows(&rect, &rows, width as usize * BYTES_PER_PIXEL, false)?;
        }
        Ok(())
    }

    pub fn surface_to_cache(&mut self, surface_id: u16, slot: u16, rect: Rect) -> RdpResult<()> {
        let surface = self.surface(surface_id)?;
        surface.validate(&rect)?;
        let stride = rect.width() as usize * BYTES_PER_PIXEL;
        let mut pixels = Vec::with_capacity(stride * rect.height() as usize);
        for row in 0..rect.height() as usize {
            let origin = (rect.top as usize + row) * surface.stride + rect.left as usize * BYTES_PER_PIXEL;
            pixels.extend_from_slice(&surface.pixels[origin..origin + stride]);
        }
        self.cache.insert(slot, CacheEntry { width: rect.width(), height: rect.height(), stride, pixels });
        Ok(())
    }

    pub fn cache_to_surface(&mut self, slot: u16, surface_id: u16, points: &[Point]) -> RdpResult<()> {
        let Some(entry) = self.cache.get(&slot) else {
            return Err(RdpProtocolError::protocol(format!("graphics cache slot is empty: {}", slot)));
        };
        let Some(surface) = self.surfaces.get_mut(&surface_id) else {
            return Err(RdpProtocolError::protocol(format!("graphics surface does not exist: {}", surface_id)));
        };
        for point in points {
            let rect = Rect::from_size(point.x, point.y, entry.width, entry.height)?;
            if rect.is_empty() {
                continue;
            }
            surface.write_rows(&rect, &entry.pixels, entry.stride, false)?;
        }
        Ok(())
    }

    pub fn evict(&mut self, slot: u16) {
        self.cache.remove(&slot);
    }

    pub fn surface_command(&mut self, command: &SurfaceCommand<'_>) -> RdpResult<()> {
        self.last_codec_id = command.codec_id;
        self.surface(command.surface_id)?.validate(&command.rect)?;
        match command.codec_id {
            CODEC_UNCOMPRESSED => self.decode_uncompressed(command),
            CODEC_REMOTEFX => self.decode_remotefx(command),
            CODEC_CLEARCODEC => self.decode_clearcodec(command),
            CODEC_PROGRESSIVE => self.decode_progressive(command),
            CODEC_PLANAR => self.decode_planar(command),
            CODEC_ALPHA => self.decode_alpha(command),
            CODEC_AVC420 => self.handle_avc420(command),
            CODEC_AVC444 | CODEC_AVC444V2 => self.passthrough_avc(command),
            other => Err(RdpProtocolError::unsupported(format!("graphics codec 0x{:04x} not implemented", other))),
        }
    }

    pub fn update(&mut self, framebuffer: &mut Framebuffer) -> RdpResult<()> {
        for surface in self.surfaces.values_mut() {
            if !surface.output_mapped {
                continue;
            }
            let mapped = Rect::from_size(0, 0, surface.mapped_width, surface.mapped_height)?;
            let invalid = surface.invalid.intersect_rect(&mapped);
            for rect in invalid.rects() {
                let target = Rect::from_size(surface.output_x.saturating_add(rect.left), surface.output_y.saturating_add(rect.top), rect.width(), rect.height())?;
                let clipped = target.intersect(&framebuffer.surface());
                if clipped.is_empty() {
                    continue;
                }
                framebuffer.blit_surface(clipped, &surface.pixels, surface.stride, rect.left, rect.top)?;
                framebuffer.blit_nv12(clipped, &surface.nv12, rect.left, rect.top);
            }
            surface.invalid = Region::default();
        }
        Ok(())
    }

    fn surface(&self, surface_id: u16) -> RdpResult<&Surface> {
        self.surfaces.get(&surface_id).ok_or_else(|| RdpProtocolError::protocol(format!("graphics surface does not exist: {}", surface_id)))
    }

    fn surface_mut(&mut self, surface_id: u16) -> RdpResult<&mut Surface> {
        self.surfaces.get_mut(&surface_id).ok_or_else(|| RdpProtocolError::protocol(format!("graphics surface does not exist: {}", surface_id)))
    }

    fn decode_uncompressed(&mut self, command: &SurfaceCommand<'_>) -> RdpResult<()> {
        let width = command.rect.width() as usize;
        let height = command.rect.height() as usize;
        let stride = width * BYTES_PER_PIXEL;
        if command.data.len() < stride * height {
            return Err(RdpProtocolError::underrun(format!("graphics uncompressed bitmap data underrun: need {} have {}", stride * height, command.data.len())));
        }
        let mut pixels = vec![0u8; stride * height];
        for index in 0..width * height {
            let offset = index * BYTES_PER_PIXEL;
            let alpha = match command.alpha {
                true => command.data[offset + 3],
                false => 0xFF,
            };
            pixels[offset..offset + BYTES_PER_PIXEL].copy_from_slice(&[command.data[offset + 2], command.data[offset + 1], command.data[offset], alpha]);
        }
        self.surface_mut(command.surface_id)?.write_rows(&command.rect, &pixels, stride, false)
    }

    fn decode_planar(&mut self, command: &SurfaceCommand<'_>) -> RdpResult<()> {
        let width = command.rect.width() as usize;
        let height = command.rect.height() as usize;
        let surface = self.surface_mut(command.surface_id)?;
        let pixels = planar::decompress(command.data, width, height, surface.alpha, false)?;
        surface.write_rows(&command.rect, &pixels, width * BYTES_PER_PIXEL, false)
    }

    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    fn decode_avc420(&mut self, command: &SurfaceCommand<'_>) -> RdpResult<()> {
        let stream = avc::parse_avc420(command.data)?;
        let surface = self.surfaces.get_mut(&command.surface_id).ok_or_else(|| RdpProtocolError::protocol(format!("graphics surface does not exist: {}", command.surface_id)))?;
        let mut decoder = match surface.avc.take() {
            Some(decoder) => decoder,
            None => avc::AvcDecoder::new()?,
        };
        let wrote = decoder.blit(stream.bitstream, &mut surface.pixels, surface.stride, surface.width, surface.height, &stream.rects);
        surface.avc = Some(decoder);
        if !wrote? {
            return Ok(());
        }
        for rect in &stream.rects {
            surface.nv12.blit_rgba(*rect, &surface.pixels, surface.stride, rect.left, rect.top);
            surface.invalid.union_rect(*rect);
        }
        Ok(())
    }

    fn handle_avc420(&mut self, command: &SurfaceCommand<'_>) -> RdpResult<()> {
        #[cfg(any(target_os = "android", target_os = "ios"))]
        {
            return self.passthrough_avc420(command);
        }
        #[cfg(not(any(target_os = "android", target_os = "ios")))]
        {
            self.decode_avc420(command)
        }
    }

    #[cfg(any(target_os = "android", target_os = "ios"))]
    fn passthrough_avc420(&mut self, command: &SurfaceCommand<'_>) -> RdpResult<()> {
        let stream = avc::parse_avc420(command.data)?;
        let surface = self.surfaces.get(&command.surface_id).ok_or_else(|| RdpProtocolError::protocol(format!("graphics surface does not exist: {}", command.surface_id)))?;
        if !surface.output_mapped {
            return Err(RdpProtocolError::protocol(format!("graphics surface is not mapped to output: {}", command.surface_id)));
        }
        let dest = Rect::new(surface.output_x, surface.output_y, surface.output_x + surface.mapped_width, surface.output_y + surface.mapped_height)?;
        self.pending_h264.push((dest, stream.bitstream.to_vec()));
        Ok(())
    }

    fn decode_remotefx(&mut self, command: &SurfaceCommand<'_>) -> RdpResult<()> {
        let bounds = self.surface(command.surface_id)?.bounds();
        let rectangles = rfx::decode(&mut self.rfx, command.data, command.rect.left, command.rect.top, bounds)?;
        let surface = self.surface_mut(command.surface_id)?;
        for rectangle in &rectangles {
            let rect = Rect::from_size(rectangle.left, rectangle.top, rectangle.width, rectangle.height)?;
            surface.write_rows(&rect, &rectangle.rgba, rectangle.width as usize * BYTES_PER_PIXEL, true)?;
        }
        Ok(())
    }

    fn decode_clearcodec(&mut self, command: &SurfaceCommand<'_>) -> RdpResult<()> {
        let surface = self.surfaces.get_mut(&command.surface_id).ok_or_else(|| RdpProtocolError::protocol(format!("graphics surface does not exist: {}", command.surface_id)))?;
        let bounds = surface.bounds();
        clear::decode(self.clear.get_or_insert_with(ClearContext::default), command.data, &mut surface.pixels, surface.stride, command.rect, bounds)?;
        surface.nv12.blit_rgba(command.rect, &surface.pixels, surface.stride, command.rect.left, command.rect.top);
        surface.invalid.union_rect(command.rect);
        Ok(())
    }

    fn decode_progressive(&mut self, command: &SurfaceCommand<'_>) -> RdpResult<()> {
        let surface = self.surfaces.get_mut(&command.surface_id).ok_or_else(|| RdpProtocolError::protocol(format!("graphics surface does not exist: {}", command.surface_id)))?;
        progressive::create_surface(&mut self.progressive, command.surface_id, surface.width as u16, surface.height as u16)?;
        let invalid =
            progressive::decode(&mut self.progressive, command.surface_id, self.frame_id, command.data, &mut surface.pixels, surface.stride, &mut surface.nv12, command.rect.left, command.rect.top)?;
        for rect in invalid.rects() {
            surface.invalid.union_rect(*rect);
        }
        Ok(())
    }

    fn passthrough_avc(&mut self, command: &SurfaceCommand<'_>) -> RdpResult<()> {
        let surface = self.surfaces.get(&command.surface_id).ok_or_else(|| RdpProtocolError::protocol(format!("graphics surface does not exist: {}", command.surface_id)))?;
        if !surface.output_mapped {
            return Err(RdpProtocolError::protocol(format!("graphics surface is not mapped to output: {}", command.surface_id)));
        }
        let payload = avc::pack_avc444_passthrough(command.codec_id == CODEC_AVC444V2, command.data)?;
        let dest = Rect::new(surface.output_x, surface.output_y, surface.output_x + surface.mapped_width, surface.output_y + surface.mapped_height)?;
        self.pending_avc.push((dest, payload));
        Ok(())
    }

    fn decode_alpha(&mut self, command: &SurfaceCommand<'_>) -> RdpResult<()> {
        let surface = self.surfaces.get_mut(&command.surface_id).ok_or_else(|| RdpProtocolError::protocol(format!("graphics surface does not exist: {}", command.surface_id)))?;
        let mut cursor = ReadCursor::new(command.data);
        let signature = cursor.read_u16_le()?;
        let compressed = cursor.read_u16_le()?;
        if signature != ALPHA_SIGNATURE {
            return Err(RdpProtocolError::protocol(format!("invalid graphics alpha codec signature: 0x{:04x}", signature)));
        }

        match compressed {
            0 => {
                for row in command.rect.top..command.rect.bottom {
                    for column in command.rect.left..command.rect.right {
                        let alpha = cursor.read_u8()?;
                        let offset = row as usize * surface.stride + column as usize * BYTES_PER_PIXEL + 3;
                        surface.pixels[offset] = match surface.alpha {
                            true => alpha,
                            false => 0xFF,
                        };
                    }
                }
            }
            _ => {
                let width = u32::from(command.rect.width());
                let mut top = command.rect.top;
                let mut offset_x = 0u32;
                while top < command.rect.bottom {
                    let alpha = cursor.read_u8()?;
                    let mut count = u32::from(cursor.read_u8()?);
                    if count >= 0xFF {
                        count = u32::from(cursor.read_u16_le()?);
                        if count >= 0xFFFF {
                            count = cursor.read_u32_le()?;
                        }
                    }
                    apply_alpha(surface, &Rect { left: command.rect.left, top, right: command.rect.right, bottom: command.rect.bottom }, offset_x, count, alpha);
                    offset_x += count;
                    while offset_x >= width {
                        offset_x -= width;
                        top += 1;
                    }
                }
            }
        }

        surface.invalid.union_rect(command.rect);
        Ok(())
    }
}

fn apply_alpha(surface: &mut Surface, rect: &Rect, offset_x: u32, count: u32, alpha: u8) {
    let mut written = 0u32;
    let mut first = true;
    for row in rect.top..rect.bottom {
        let start = match first {
            true => rect.left as u32 + offset_x,
            false => rect.left as u32,
        };
        first = false;
        for column in start..rect.right as u32 {
            if written >= count {
                return;
            }
            let offset = row as usize * surface.stride + column as usize * BYTES_PER_PIXEL + 3;
            surface.pixels[offset] = match surface.alpha {
                true => alpha,
                false => 0xFF,
            };
            written += 1;
        }
    }
}
