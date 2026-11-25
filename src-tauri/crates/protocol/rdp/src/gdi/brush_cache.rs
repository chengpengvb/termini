use crate::codec::color;
use std::collections::HashMap;
use crate::codec::color::Palette;
use crate::core::orders::{CacheBrush, BRUSH_EDGE};
use crate::core::error::{RdpProtocolError, RdpResult};

const MAX_ENTRIES: u8 = 64;
const BRUSH_PIXELS: usize = BRUSH_EDGE * BRUSH_EDGE;
const BYTES_PER_PIXEL: usize = 4;

#[derive(Default)]
pub struct BrushCache {
    monochrome: HashMap<u8, [u8; BRUSH_EDGE]>,
    color: HashMap<u8, [u32; BRUSH_PIXELS]>,
}

impl BrushCache {
    pub fn store(&mut self, order: &CacheBrush, palette: &Palette) -> RdpResult<()> {
        if order.index >= MAX_ENTRIES {
            return Err(RdpProtocolError::protocol(format!("brush cache index out of range: {}", order.index)));
        }
        if order.bits_per_pixel == 1 {
            let rows = <[u8; BRUSH_EDGE]>::try_from(order.data.as_slice()).map_err(|_| RdpProtocolError::protocol(format!("monochrome brush data length mismatch: {}", order.data.len())))?;
            self.monochrome.insert(order.index, rows);
            return Ok(());
        }
        let decoded = color::to_rgba(&order.data, BRUSH_EDGE, BRUSH_EDGE, order.bits_per_pixel, palette)?;
        let mut pixels = [0u32; BRUSH_PIXELS];
        for (index, pixel) in pixels.iter_mut().enumerate() {
            let offset = index * BYTES_PER_PIXEL;
            *pixel = u32::from_le_bytes([decoded[offset], decoded[offset + 1], decoded[offset + 2], decoded[offset + 3]]);
        }
        self.color.insert(order.index, pixels);
        Ok(())
    }

    pub fn monochrome(&self, index: u8) -> RdpResult<&[u8; BRUSH_EDGE]> {
        self.monochrome.get(&index).ok_or_else(|| RdpProtocolError::protocol(format!("monochrome brush cache miss: {}", index)))
    }

    pub fn color(&self, index: u8) -> RdpResult<&[u32; BRUSH_PIXELS]> {
        self.color.get(&index).ok_or_else(|| RdpProtocolError::protocol(format!("color brush cache miss: {}", index)))
    }
}
