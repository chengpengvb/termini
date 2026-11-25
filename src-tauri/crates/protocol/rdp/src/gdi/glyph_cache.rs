use crate::core::error::{RdpProtocolError, RdpResult};

pub const GLYPH_CACHE_CELLS: [(u16, u16); 10] = [(254, 4), (254, 4), (254, 8), (254, 8), (254, 16), (254, 32), (254, 64), (254, 128), (254, 256), (64, 2048)];

pub const FRAG_CACHE_ENTRIES: u16 = 256;
pub const FRAG_CACHE_CELL_SIZE: u16 = 256;

#[derive(Clone)]
pub struct CachedGlyph {
    pub index: u16,
    pub x: i32,
    pub y: i32,
    pub cx: u16,
    pub cy: u16,
    pub aj: Vec<u8>,
}

pub struct GlyphCache {
    cells: Vec<Vec<Option<CachedGlyph>>>,
    fragments: Vec<Option<Vec<u8>>>,
}

impl Default for GlyphCache {
    fn default() -> Self {
        Self { cells: GLYPH_CACHE_CELLS.iter().map(|(entries, _)| vec![None; *entries as usize]).collect(), fragments: vec![None; FRAG_CACHE_ENTRIES as usize] }
    }
}

impl GlyphCache {
    pub fn store(&mut self, cache_id: u8, glyph: CachedGlyph) -> RdpResult<()> {
        let cell = self.cells.get_mut(cache_id as usize).ok_or_else(|| RdpProtocolError::protocol(format!("glyph cache cacheId out of range: {}", cache_id)))?;
        let index = glyph.index as usize;
        if index >= cell.len() {
            return Err(RdpProtocolError::protocol(format!("glyph cache cacheIndex out of range: cell {} index {} limit {}", cache_id, glyph.index, cell.len())));
        }
        let stride = (glyph.cx as usize + 7) / 8;
        let required = stride * glyph.cy as usize;
        if glyph.aj.len() < required {
            return Err(RdpProtocolError::underrun(format!("glyph bitmap underrun: need {} have {}", required, glyph.aj.len())));
        }
        cell[index] = Some(glyph);
        Ok(())
    }

    pub fn get(&self, cache_id: u8, cache_index: u16) -> RdpResult<&CachedGlyph> {
        let cell = self.cells.get(cache_id as usize).ok_or_else(|| RdpProtocolError::protocol(format!("glyph cache cacheId out of range: {}", cache_id)))?;
        let glyph = cell.get(cache_index as usize).ok_or_else(|| RdpProtocolError::protocol(format!("glyph cache cacheIndex out of range: cell {} index {}", cache_id, cache_index)))?;
        glyph.as_ref().ok_or_else(|| RdpProtocolError::protocol(format!("glyph cache miss: cell {} index {}", cache_id, cache_index)))
    }

    pub fn fragment_put(&mut self, index: u8, data: &[u8]) -> RdpResult<()> {
        if data.is_empty() {
            return Err(RdpProtocolError::protocol(format!("glyph fragment length is 0: index={}", index)));
        }
        self.fragments[index as usize] = Some(data.to_vec());
        Ok(())
    }

    pub fn fragment_get(&self, index: u8) -> RdpResult<&[u8]> {
        self.fragments[index as usize].as_deref().ok_or_else(|| RdpProtocolError::protocol(format!("glyph fragment cache miss: index={}", index)))
    }
}
