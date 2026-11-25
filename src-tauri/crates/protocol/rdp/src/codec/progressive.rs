use rayon::prelude::*;
use std::cell::RefCell;
use crate::codec::color;
use crate::codec::rfx_dwt;
use crate::codec::rfx_rlgr;
use std::collections::HashMap;
use crate::codec::region::Region;
use crate::codec::nv12::Nv12Image;
use crate::gdi::framebuffer::Rect;
use crate::core::stream::ReadCursor;
use crate::codec::rfx_rlgr::RlgrMode;
use crate::codec::bitstream::BitStream;
use crate::codec::rfx_dwt::TILE_COEFFICIENTS;
use crate::core::error::{RdpProtocolError, RdpResult};

const WBT_SYNC: u16 = 0xCCC0;
const WBT_FRAME_BEGIN: u16 = 0xCCC1;
const WBT_FRAME_END: u16 = 0xCCC2;
const WBT_CONTEXT: u16 = 0xCCC3;
const WBT_REGION: u16 = 0xCCC4;
const WBT_TILE_SIMPLE: u16 = 0xCCC5;
const WBT_TILE_FIRST: u16 = 0xCCC6;
const WBT_TILE_UPGRADE: u16 = 0xCCC7;

const SYNC_MAGIC: u32 = 0xCACC_ACCA;
const SYNC_VERSION: u16 = 0x0100;

const FLAG_FRAME_BEGIN: u8 = 0x01;
const FLAG_FRAME_END: u8 = 0x02;

const TILE_DIFFERENCE: u8 = 0x01;
const DWT_REDUCE_EXTRAPOLATE: u8 = 0x01;

const BLOCK_HEADER_LENGTH: usize = 6;
const REGION_HEADER_LENGTH: usize = 12;
const TILE_SIZE: usize = 64;
const TILE_PIXELS: usize = TILE_SIZE * TILE_SIZE;
const PARALLEL_TILE_THRESHOLD: usize = 4;
const TILE_STRIDE: usize = TILE_SIZE * 4;
const COMPONENT_COUNT: usize = 3;
const QUANT_LENGTH: usize = 5;
const PROG_QUANT_LENGTH: usize = 16;
const RECT_LENGTH: usize = 8;
const MAX_QUANT_COUNT: usize = 7;
const QUANT_FLOOR: u32 = 6;
const QUANT_CEIL: u32 = 15;
const FULL_QUALITY: u8 = 0xFF;

const BAND_COUNT: usize = 10;
const BAND_HL1: usize = 0;
const BAND_LH1: usize = 1;
const BAND_HH1: usize = 2;
const BAND_HL2: usize = 3;
const BAND_LH2: usize = 4;
const BAND_HH2: usize = 5;
const BAND_HL3: usize = 6;
const BAND_LH3: usize = 7;
const BAND_HH3: usize = 8;
const BAND_LL3: usize = 9;

const CLASSIC_BANDS: [(usize, usize); BAND_COUNT] = [(0, 1024), (1024, 1024), (2048, 1024), (3072, 256), (3328, 256), (3584, 256), (3840, 64), (3904, 64), (3968, 64), (4032, 64)];
const EXTRAPOLATE_BANDS: [(usize, usize); BAND_COUNT] = [(0, 1023), (1023, 1023), (2046, 961), (3007, 272), (3279, 272), (3551, 256), (3807, 72), (3879, 72), (3951, 64), (4015, 81)];

const SRL_KP_INITIAL: u32 = 8;
const SRL_KP_MAX: u32 = 80;
const SRL_KP_STEP_UP: u32 = 4;
const SRL_KP_STEP_DOWN: u32 = 6;

#[derive(Default)]
pub struct ProgressiveContext {
    surfaces: HashMap<u16, Surface>,
}

struct Surface {
    width: usize,
    height: usize,
    grid_width: usize,
    tiles: Vec<Tile>,
    updated: Vec<usize>,
    frame_id: u32,
}

struct Tile {
    top: usize,
    dirty: bool,
    left: usize,
    data: Vec<u8>,
    luma: Box<[u8; TILE_PIXELS]>,
    bit_pos: [Quant; COMPONENT_COUNT],
    chroma: Box<[u8; TILE_PIXELS / 2]>,
    sign: Box<[[i16; TILE_COEFFICIENTS]; COMPONENT_COUNT]>,
    current: Box<[[i16; TILE_COEFFICIENTS]; COMPONENT_COUNT]>,
}

#[derive(Clone, Copy, Default)]
struct Quant {
    bands: [u32; BAND_COUNT],
}

#[derive(Clone, Copy, Default)]
struct ProgQuant {
    components: [Quant; COMPONENT_COUNT],
}

struct RegionBlock<'a> {
    rects: Vec<Rect>,
    quants: Vec<Quant>,
    prog_quants: Vec<ProgQuant>,
    flags: u8,
    tiles: Vec<TileBlock<'a>>,
}

struct TileBlock<'a> {
    index: usize,
    quant_indexes: [usize; COMPONENT_COUNT],
    quality: u8,
    flags: u8,
    payload: TilePayload<'a>,
}

enum TilePayload<'a> {
    First { components: [&'a [u8]; COMPONENT_COUNT] },
    Upgrade { srl: [&'a [u8]; COMPONENT_COUNT], raw: [&'a [u8]; COMPONENT_COUNT] },
}

struct UpgradeState<'a> {
    srl: BitStream<'a>,
    raw: BitStream<'a>,
    kp: u32,
    mode: bool,
    nz: i32,
    non_ll: bool,
}

struct Scratch {
    buffer: Box<[[i16; TILE_COEFFICIENTS]; COMPONENT_COUNT]>,
    temp: Box<[i16; TILE_COEFFICIENTS]>,
}

impl Quant {
    fn add(&self, other: &Quant) -> Quant {
        let mut bands = [0u32; BAND_COUNT];
        for index in 0..BAND_COUNT {
            bands[index] = self.bands[index] + other.bands[index];
        }
        Quant { bands }
    }

    fn sub(&self, other: &Quant) -> RdpResult<Quant> {
        let mut bands = [0u32; BAND_COUNT];
        for index in 0..BAND_COUNT {
            if self.bands[index] < other.bands[index] {
                return Err(RdpProtocolError::protocol(format!("Progressive quantization difference is negative: {} < {}", self.bands[index], other.bands[index])));
            }
            bands[index] = self.bands[index] - other.bands[index];
        }
        Ok(Quant { bands })
    }

    fn lsub(&self, value: u32) -> RdpResult<Quant> {
        let mut bands = [0u32; BAND_COUNT];
        for index in 0..BAND_COUNT {
            if self.bands[index] < value {
                return Err(RdpProtocolError::protocol(format!("Progressive quantization too small to subtract {}: {}", value, self.bands[index])));
            }
            bands[index] = self.bands[index] - value;
        }
        Ok(Quant { bands })
    }
}

impl Tile {
    fn new(left: usize, top: usize) -> Self {
        Self {
            left,
            top,
            dirty: false,
            luma: Box::new([0u8; TILE_PIXELS]),
            data: vec![0xFFu8; TILE_PIXELS * 4],
            chroma: Box::new([128u8; TILE_PIXELS / 2]),
            bit_pos: [Quant::default(); COMPONENT_COUNT],
            sign: Box::new([[0i16; TILE_COEFFICIENTS]; COMPONENT_COUNT]),
            current: Box::new([[0i16; TILE_COEFFICIENTS]; COMPONENT_COUNT]),
        }
    }
}

impl Surface {
    fn new(width: usize, height: usize) -> Self {
        let grid_width = (width + (TILE_SIZE - width % TILE_SIZE)) / TILE_SIZE;
        let grid_height = (height + (TILE_SIZE - height % TILE_SIZE)) / TILE_SIZE;
        let mut tiles = Vec::with_capacity(grid_width * grid_height);
        for index in 0..grid_width * grid_height {
            tiles.push(Tile::new((index % grid_width) * TILE_SIZE, (index / grid_width) * TILE_SIZE));
        }
        Self { width, height, grid_width, tiles, updated: Vec::new(), frame_id: 0 }
    }
}

impl Scratch {
    fn new() -> Self {
        Self { buffer: Box::new([[0i16; TILE_COEFFICIENTS]; COMPONENT_COUNT]), temp: Box::new([0i16; TILE_COEFFICIENTS]) }
    }
}

pub fn create_surface(context: &mut ProgressiveContext, surface_id: u16, width: u16, height: u16) -> RdpResult<()> {
    if width == 0 || height == 0 {
        return Err(RdpProtocolError::protocol(format!("Progressive surface size is zero: {}x{}", width, height)));
    }
    context.surfaces.entry(surface_id).or_insert_with(|| Surface::new(width as usize, height as usize));
    Ok(())
}

pub fn delete_surface(context: &mut ProgressiveContext, surface_id: u16) {
    context.surfaces.remove(&surface_id);
}

pub fn decode(context: &mut ProgressiveContext, surface_id: u16, frame_id: u32, source: &[u8], target: &mut [u8], stride: usize, nv12: &mut Nv12Image, left: u16, top: u16) -> RdpResult<Region> {
    let Some(surface) = context.surfaces.get_mut(&surface_id) else {
        return Err(RdpProtocolError::protocol(format!("Progressive surface not created: {}", surface_id)));
    };
    if surface.frame_id != frame_id {
        surface.frame_id = frame_id;
        surface.updated.clear();
    }

    let mut cursor = ReadCursor::new(source);
    let mut scratch = Scratch::new();
    let mut state = 0u8;
    let mut invalid = Region::default();

    while cursor.remaining() > 0 {
        let block_type = cursor.read_u16_le()?;
        let block_length = cursor.read_u32_le()? as usize;
        if block_length < BLOCK_HEADER_LENGTH {
            return Err(RdpProtocolError::protocol(format!("Progressive block length too small: {}", block_length)));
        }
        let mut body = cursor.sub_cursor(block_length - BLOCK_HEADER_LENGTH)?;

        match block_type {
            WBT_SYNC => read_sync(&mut body, block_length)?,
            WBT_FRAME_BEGIN => read_frame_begin(&mut body, block_length, &mut state)?,
            WBT_FRAME_END => read_frame_end(&mut body, block_length, &mut state)?,
            WBT_CONTEXT => read_context(&mut body, block_length)?,
            WBT_REGION => match read_region(&mut body, surface, state)? {
                None => body.skip(body.remaining())?,
                Some(region) => {
                    decode_region_tiles(&mut surface.tiles, &region, &mut scratch, nv12.width() >= 1280)?;
                    update_tiles(surface, &region, target, stride, nv12, left, top, &mut invalid)?;
                }
            },
            other => return Err(RdpProtocolError::protocol(format!("unknown Progressive block type: 0x{:04x}", other))),
        }

        if body.remaining() > 0 {
            return Err(RdpProtocolError::protocol(format!("Progressive block 0x{:04x} has {} unparsed bytes remaining", block_type, body.remaining())));
        }
    }

    Ok(invalid)
}

fn read_sync(cursor: &mut ReadCursor<'_>, block_length: usize) -> RdpResult<()> {
    if block_length != 12 {
        return Err(RdpProtocolError::protocol(format!("Progressive invalid sync block length: {}", block_length)));
    }
    let magic = cursor.read_u32_le()?;
    if magic != SYNC_MAGIC {
        return Err(RdpProtocolError::protocol(format!("Progressive invalid magic: 0x{:08x}", magic)));
    }
    let version = cursor.read_u16_le()?;
    if version != SYNC_VERSION {
        return Err(RdpProtocolError::protocol(format!("Progressive invalid version: 0x{:04x}", version)));
    }
    Ok(())
}

fn read_frame_begin(cursor: &mut ReadCursor<'_>, block_length: usize, state: &mut u8) -> RdpResult<()> {
    if block_length != 12 {
        return Err(RdpProtocolError::protocol(format!("Progressive invalid frame begin block length: {}", block_length)));
    }
    if *state & FLAG_FRAME_BEGIN != 0 {
        return Err(RdpProtocolError::protocol("Progressive duplicate frame begin".to_string()));
    }
    if *state & FLAG_FRAME_END != 0 {
        return Err(RdpProtocolError::protocol("Progressive frame begin after frame end".to_string()));
    }
    cursor.read_u32_le()?;
    cursor.read_u16_le()?;
    *state |= FLAG_FRAME_BEGIN;
    Ok(())
}

fn read_frame_end(cursor: &mut ReadCursor<'_>, block_length: usize, state: &mut u8) -> RdpResult<()> {
    if block_length != BLOCK_HEADER_LENGTH {
        return Err(RdpProtocolError::protocol(format!("Progressive invalid frame end block length: {}", block_length)));
    }
    if cursor.remaining() != 0 {
        return Err(RdpProtocolError::protocol(format!("Progressive frame end has {} extra bytes", cursor.remaining())));
    }
    *state |= FLAG_FRAME_END;
    Ok(())
}

fn read_context(cursor: &mut ReadCursor<'_>, block_length: usize) -> RdpResult<()> {
    if block_length != 10 {
        return Err(RdpProtocolError::protocol(format!("Progressive invalid context block length: {}", block_length)));
    }
    cursor.read_u8()?;
    let tile_size = cursor.read_u16_le()?;
    cursor.read_u8()?;
    if tile_size != TILE_SIZE as u16 {
        return Err(RdpProtocolError::protocol(format!("Progressive invalid tileSize: {}", tile_size)));
    }
    Ok(())
}

fn read_region<'a>(cursor: &mut ReadCursor<'a>, surface: &mut Surface, state: u8) -> RdpResult<Option<RegionBlock<'a>>> {
    if cursor.remaining() < REGION_HEADER_LENGTH {
        return Err(RdpProtocolError::underrun(format!("Progressive region header underrun: {}", cursor.remaining())));
    }
    let tile_size = cursor.read_u8()?;
    let rect_count = cursor.read_u16_le()? as usize;
    let quant_count = cursor.read_u8()? as usize;
    let prog_quant_count = cursor.read_u8()? as usize;
    let flags = cursor.read_u8()?;
    let tile_count = cursor.read_u16_le()? as usize;
    let tile_data_size = cursor.read_u32_le()? as usize;

    if tile_size != TILE_SIZE as u8 {
        return Err(RdpProtocolError::protocol(format!("Progressive invalid region tileSize: {}", tile_size)));
    }
    if rect_count < 1 {
        return Err(RdpProtocolError::protocol("Progressive region declares no rectangles".to_string()));
    }
    if quant_count > MAX_QUANT_COUNT {
        return Err(RdpProtocolError::protocol(format!("Progressive region has too many quantization tables: {}", quant_count)));
    }
    let expected = rect_count * RECT_LENGTH + quant_count * QUANT_LENGTH + prog_quant_count * PROG_QUANT_LENGTH + tile_data_size;
    if expected != cursor.remaining() {
        return Err(RdpProtocolError::protocol(format!("Progressive region length mismatch: need {} have {}", expected, cursor.remaining())));
    }
    if state & FLAG_FRAME_BEGIN == 0 || state & FLAG_FRAME_END != 0 {
        return Ok(None);
    }

    let mut rects = Vec::with_capacity(rect_count);
    for _ in 0..rect_count {
        let x = cursor.read_u16_le()?;
        let y = cursor.read_u16_le()?;
        let width = cursor.read_u16_le()?;
        let height = cursor.read_u16_le()?;
        rects.push(Rect::from_size(x, y, width, height)?);
    }

    let mut quants = Vec::with_capacity(quant_count);
    for _ in 0..quant_count {
        let quant = read_quant(cursor)?;
        if quant.bands.iter().any(|value| *value < QUANT_FLOOR) {
            return Err(RdpProtocolError::protocol("Progressive quantization value less than 6".to_string()));
        }
        if quant.bands.iter().any(|value| *value > QUANT_CEIL) {
            return Err(RdpProtocolError::protocol("Progressive quantization value greater than 15".to_string()));
        }
        quants.push(quant);
    }

    let mut prog_quants = Vec::with_capacity(prog_quant_count);
    for _ in 0..prog_quant_count {
        cursor.read_u8()?;
        let mut components = [Quant::default(); COMPONENT_COUNT];
        for component in components.iter_mut() {
            *component = read_quant(cursor)?;
        }
        prog_quants.push(ProgQuant { components });
    }

    let mut body = cursor.sub_cursor(tile_data_size)?;
    let mut tiles = Vec::with_capacity(tile_count);
    while body.remaining() >= BLOCK_HEADER_LENGTH {
        let block_type = body.read_u16_le()?;
        let block_length = body.read_u32_le()? as usize;
        if block_length < BLOCK_HEADER_LENGTH {
            return Err(RdpProtocolError::protocol(format!("Progressive tile block length too small: {}", block_length)));
        }
        let mut tile = body.sub_cursor(block_length - BLOCK_HEADER_LENGTH)?;
        let block = match block_type {
            WBT_TILE_SIMPLE => read_tile(&mut tile, true, surface)?,
            WBT_TILE_FIRST => read_tile(&mut tile, false, surface)?,
            WBT_TILE_UPGRADE => read_tile_upgrade(&mut tile, surface)?,
            other => return Err(RdpProtocolError::protocol(format!("Progressive invalid tile block type: 0x{:04x}", other))),
        };
        if tile.remaining() > 0 {
            return Err(RdpProtocolError::protocol(format!("Progressive tile block has {} unparsed bytes remaining", tile.remaining())));
        }
        if tiles.len() >= tile_count {
            return Err(RdpProtocolError::protocol(format!("Progressive tile count exceeds declared count: {}", tile_count)));
        }
        tiles.push(block);
    }

    if body.remaining() > 0 {
        return Err(RdpProtocolError::protocol(format!("Progressive tile data has {} bytes remaining", body.remaining())));
    }
    if tiles.len() != tile_count {
        return Err(RdpProtocolError::protocol(format!("Progressive tile count mismatch: {} expected {}", tiles.len(), tile_count)));
    }

    for tile in &tiles {
        if !surface.tiles[tile.index].dirty {
            surface.updated.push(tile.index);
            surface.tiles[tile.index].dirty = true;
        }
    }

    Ok(Some(RegionBlock { rects, quants, prog_quants, flags, tiles }))
}

fn read_quant(cursor: &mut ReadCursor<'_>) -> RdpResult<Quant> {
    let mut bands = [0u32; BAND_COUNT];
    let value = cursor.read_u8()?;
    bands[BAND_LL3] = u32::from(value & 0x0F);
    bands[BAND_HL3] = u32::from(value >> 4);
    let value = cursor.read_u8()?;
    bands[BAND_LH3] = u32::from(value & 0x0F);
    bands[BAND_HH3] = u32::from(value >> 4);
    let value = cursor.read_u8()?;
    bands[BAND_HL2] = u32::from(value & 0x0F);
    bands[BAND_LH2] = u32::from(value >> 4);
    let value = cursor.read_u8()?;
    bands[BAND_HH2] = u32::from(value & 0x0F);
    bands[BAND_HL1] = u32::from(value >> 4);
    let value = cursor.read_u8()?;
    bands[BAND_LH1] = u32::from(value & 0x0F);
    bands[BAND_HH1] = u32::from(value >> 4);
    Ok(Quant { bands })
}

fn read_tile<'a>(cursor: &mut ReadCursor<'a>, simple: bool, surface: &Surface) -> RdpResult<TileBlock<'a>> {
    let quant_y = cursor.read_u8()? as usize;
    let quant_cb = cursor.read_u8()? as usize;
    let quant_cr = cursor.read_u8()? as usize;
    let column = cursor.read_u16_le()? as usize;
    let row = cursor.read_u16_le()? as usize;
    let flags = cursor.read_u8()?;
    let quality = match simple {
        true => FULL_QUALITY,
        false => cursor.read_u8()?,
    };
    let luma_length = cursor.read_u16_le()? as usize;
    let blue_length = cursor.read_u16_le()? as usize;
    let red_length = cursor.read_u16_le()? as usize;
    let tail_length = cursor.read_u16_le()? as usize;

    let components = [cursor.read_slice(luma_length)?, cursor.read_slice(blue_length)?, cursor.read_slice(red_length)?];
    cursor.skip(tail_length)?;

    Ok(TileBlock { index: tile_index(surface, column, row)?, quant_indexes: [quant_y, quant_cb, quant_cr], quality, flags, payload: TilePayload::First { components } })
}

fn read_tile_upgrade<'a>(cursor: &mut ReadCursor<'a>, surface: &Surface) -> RdpResult<TileBlock<'a>> {
    let quant_y = cursor.read_u8()? as usize;
    let quant_cb = cursor.read_u8()? as usize;
    let quant_cr = cursor.read_u8()? as usize;
    let column = cursor.read_u16_le()? as usize;
    let row = cursor.read_u16_le()? as usize;
    let quality = cursor.read_u8()?;
    let mut lengths = [(0usize, 0usize); COMPONENT_COUNT];
    for length in lengths.iter_mut() {
        length.0 = cursor.read_u16_le()? as usize;
        length.1 = cursor.read_u16_le()? as usize;
    }

    let mut srl = [&[] as &[u8]; COMPONENT_COUNT];
    let mut raw = [&[] as &[u8]; COMPONENT_COUNT];
    for (index, length) in lengths.iter().enumerate() {
        srl[index] = cursor.read_slice(length.0)?;
        raw[index] = cursor.read_slice(length.1)?;
    }

    Ok(TileBlock { index: tile_index(surface, column, row)?, quant_indexes: [quant_y, quant_cb, quant_cr], quality, flags: 0, payload: TilePayload::Upgrade { srl, raw } })
}

fn tile_index(surface: &Surface, column: usize, row: usize) -> RdpResult<usize> {
    let index = row * surface.grid_width + column;
    if column >= surface.grid_width || index >= surface.tiles.len() {
        return Err(RdpProtocolError::protocol(format!("Progressive tile index out of range: ({},{})", column, row)));
    }
    Ok(index)
}

thread_local! {
    static TILE_SCRATCH: RefCell<Scratch> = RefCell::new(Scratch::new());
}

fn tile_blocks_unique(blocks: &[TileBlock<'_>], tile_count: usize) -> bool {
    let mut bits = vec![0u64; tile_count.div_ceil(64)];
    for block in blocks {
        if block.index >= tile_count {
            return false;
        }
        let word = block.index / 64;
        let mask = 1u64 << (block.index % 64);
        if bits[word] & mask != 0 {
            return false;
        }
        bits[word] |= mask;
    }
    true
}

fn decode_region_tiles(tiles: &mut [Tile], region: &RegionBlock<'_>, scratch: &mut Scratch, bt709: bool) -> RdpResult<()> {
    if region.tiles.len() < PARALLEL_TILE_THRESHOLD || !tile_blocks_unique(&region.tiles, tiles.len()) {
        for block in &region.tiles {
            decode_tile(&mut tiles[block.index], block, region, scratch, bt709)?;
        }
        return Ok(());
    }
    let tile_ptr = tiles.as_mut_ptr() as usize;
    let tile_len = tiles.len();
    region.tiles.par_iter().try_for_each(|block| {
        if block.index >= tile_len {
            return Err(RdpProtocolError::protocol(format!("Progressive tile index out of range: {}", block.index)));
        }
        TILE_SCRATCH.with(|cell| {
            let tile = unsafe { &mut *(tile_ptr as *mut Tile).add(block.index) };
            decode_tile(tile, block, region, &mut cell.borrow_mut(), bt709)
        })
    })
}

fn decode_tile(tile: &mut Tile, block: &TileBlock<'_>, region: &RegionBlock<'_>, scratch: &mut Scratch, bt709: bool) -> RdpResult<()> {
    let mut quants = [Quant::default(); COMPONENT_COUNT];
    for (index, quant) in quants.iter_mut().enumerate() {
        let Some(value) = region.quants.get(block.quant_indexes[index]) else {
            return Err(RdpProtocolError::protocol(format!("Progressive quantization index out of range: {}", block.quant_indexes[index])));
        };
        *quant = *value;
    }

    let prog = match block.quality {
        FULL_QUALITY => ProgQuant::default(),
        quality => match region.prog_quants.get(quality as usize) {
            Some(value) => *value,
            None => return Err(RdpProtocolError::protocol(format!("Progressive progressive-quantization index out of range: {}", quality))),
        },
    };

    let coefficient_diff = block.flags & TILE_DIFFERENCE != 0;
    let extrapolate = region.flags & DWT_REDUCE_EXTRAPOLATE != 0;

    match &block.payload {
        TilePayload::First { components } => {
            for component in 0..COMPONENT_COUNT {
                let bit_pos = quants[component].add(&prog.components[component]);
                let shift = bit_pos.lsub(1)?;
                tile.bit_pos[component] = bit_pos;
                decode_component(
                    &shift,
                    components[component],
                    &mut scratch.buffer[component],
                    &mut tile.current[component],
                    &mut tile.sign[component],
                    &mut scratch.temp,
                    coefficient_diff,
                    extrapolate,
                )?;
            }
        }
        TilePayload::Upgrade { srl, raw } => {
            for component in 0..COMPONENT_COUNT {
                let bit_pos = quants[component].add(&prog.components[component]);
                let num_bits = tile.bit_pos[component].sub(&bit_pos)?;
                let shift = bit_pos.lsub(1)?;
                tile.bit_pos[component] = bit_pos;
                upgrade_component(
                    &shift,
                    &num_bits,
                    srl[component],
                    raw[component],
                    &mut scratch.buffer[component],
                    &mut tile.current[component],
                    &mut tile.sign[component],
                    &mut scratch.temp,
                    coefficient_diff,
                    extrapolate,
                )?;
            }
        }
    }

    for row in (0..TILE_SIZE).step_by(2) {
        for column in (0..TILE_SIZE).step_by(2) {
            let mut sum_u = 0u32;
            let mut sum_v = 0u32;
            for dy in 0..2 {
                for dx in 0..2 {
                    let index = (row + dy) * TILE_SIZE + (column + dx);
                    let rgba = color::ycbcr_to_rgba(scratch.buffer[0][index], scratch.buffer[1][index], scratch.buffer[2][index]);
                    tile.data[index * 4..index * 4 + 4].copy_from_slice(&rgba);
                    let (y, u, v) = color::rgb_to_yuv(rgba[0], rgba[1], rgba[2], bt709);
                    tile.luma[index] = y;
                    sum_u += u32::from(u);
                    sum_v += u32::from(v);
                }
            }
            let chroma_index = (row / 2) * TILE_SIZE + column;
            tile.chroma[chroma_index] = (sum_u / 4) as u8;
            tile.chroma[chroma_index + 1] = (sum_v / 4) as u8;
        }
    }
    Ok(())
}

fn decode_component(
    shift: &Quant,
    data: &[u8],
    buffer: &mut [i16; TILE_COEFFICIENTS],
    current: &mut [i16; TILE_COEFFICIENTS],
    sign: &mut [i16; TILE_COEFFICIENTS],
    temp: &mut [i16; TILE_COEFFICIENTS],
    coefficient_diff: bool,
    extrapolate: bool,
) -> RdpResult<()> {
    rfx_rlgr::decode(RlgrMode::Rlgr1, data, buffer)?;
    sign.copy_from_slice(&buffer[..]);

    let bands = match extrapolate {
        true => EXTRAPOLATE_BANDS,
        false => CLASSIC_BANDS,
    };
    let (offset, length) = bands[BAND_LL3];
    differential_decode(&mut buffer[offset..offset + length]);
    for (index, (offset, length)) in bands.iter().enumerate() {
        shift_band(&mut buffer[*offset..*offset + *length], shift.bands[index])?;
    }

    dwt_2d_decode(buffer, current, temp, coefficient_diff, extrapolate, false);
    Ok(())
}

fn upgrade_component(
    shift: &Quant,
    num_bits: &Quant,
    srl: &[u8],
    raw: &[u8],
    buffer: &mut [i16; TILE_COEFFICIENTS],
    current: &mut [i16; TILE_COEFFICIENTS],
    sign: &mut [i16; TILE_COEFFICIENTS],
    temp: &mut [i16; TILE_COEFFICIENTS],
    coefficient_diff: bool,
    extrapolate: bool,
) -> RdpResult<()> {
    let mut state = UpgradeState { srl: BitStream::new(srl), raw: BitStream::new(raw), kp: SRL_KP_INITIAL, mode: false, nz: 0, non_ll: true };

    for (index, (offset, length)) in EXTRAPOLATE_BANDS.iter().enumerate() {
        state.non_ll = index != BAND_LL3;
        upgrade_band(&mut state, &mut current[*offset..*offset + *length], &mut sign[*offset..*offset + *length], shift.bands[index], num_bits.bands[index]);
    }

    let pad = state.raw.remaining() % 8;
    state.raw.shift(pad);
    let pad = state.srl.remaining() % 8;
    state.srl.shift(pad);
    if state.srl.remaining() == 8 {
        state.srl.shift(8);
    }

    dwt_2d_decode(buffer, current, temp, coefficient_diff, extrapolate, true);
    Ok(())
}

fn upgrade_band(state: &mut UpgradeState<'_>, buffer: &mut [i16], sign: &mut [i16], shift: u32, num_bits: u32) {
    if num_bits < 1 {
        return;
    }

    for index in 0..buffer.len() {
        let input = match state.non_ll {
            false => i32::from(state.raw.read_bits(num_bits) as i16),
            true => match sign[index] {
                value if value > 0 => i32::from(state.raw.read_bits(num_bits) as i16),
                value if value < 0 => -i32::from(state.raw.read_bits(num_bits) as i16),
                _ => {
                    let value = srl_read(state, num_bits);
                    sign[index] = value;
                    i32::from(value)
                }
            },
        };
        buffer[index] = clamp_i16(i32::from(buffer[index]) + (input << shift));
    }
}

fn srl_read(state: &mut UpgradeState<'_>, num_bits: u32) -> i16 {
    if state.nz > 0 {
        state.nz -= 1;
        return 0;
    }

    let k = state.kp / 8;
    if !state.mode {
        if state.srl.read_bits(1) == 0 {
            state.nz = 1 << k;
            state.kp = (state.kp + SRL_KP_STEP_UP).min(SRL_KP_MAX);
            state.nz -= 1;
            return 0;
        }
        state.nz = 0;
        state.mode = true;
        if k > 0 {
            state.nz = state.srl.read_bits(k) as i32;
        }
        if state.nz > 0 {
            state.nz -= 1;
            return 0;
        }
    }

    state.mode = false;
    let negative = state.srl.read_bits(1) == 1;
    state.kp = state.kp.saturating_sub(SRL_KP_STEP_DOWN);
    if num_bits == 1 {
        return match negative {
            true => -1,
            false => 1,
        };
    }

    let mut magnitude = 1u32;
    let maximum = (1u32 << num_bits) - 1;
    while magnitude < maximum {
        if state.srl.read_bits(1) == 1 {
            break;
        }
        magnitude += 1;
    }

    let magnitude = magnitude.min(i16::MAX as u32) as i16;
    match negative {
        true => -magnitude,
        false => magnitude,
    }
}

fn dwt_2d_decode(buffer: &mut [i16; TILE_COEFFICIENTS], current: &mut [i16; TILE_COEFFICIENTS], temp: &mut [i16; TILE_COEFFICIENTS], coefficient_diff: bool, extrapolate: bool, reverse: bool) {
    if reverse {
        buffer.copy_from_slice(&current[..]);
    } else if !coefficient_diff {
        current.copy_from_slice(&buffer[..]);
    } else {
        for index in 0..TILE_COEFFICIENTS {
            let value = buffer[index].saturating_add(current[index]);
            buffer[index] = value;
            current[index] = value;
        }
    }

    match extrapolate {
        true => extrapolate_decode(buffer, temp),
        false => rfx_dwt::decode_2d(buffer, temp),
    }
}

fn extrapolate_decode(buffer: &mut [i16; TILE_COEFFICIENTS], temp: &mut [i16; TILE_COEFFICIENTS]) {
    decode_block(&mut buffer[EXTRAPOLATE_BANDS[BAND_HL3].0..], temp, 3);
    decode_block(&mut buffer[EXTRAPOLATE_BANDS[BAND_HL2].0..], temp, 2);
    decode_block(&mut buffer[EXTRAPOLATE_BANDS[BAND_HL1].0..], temp, 1);
}

fn decode_block(buffer: &mut [i16], temp: &mut [i16], level: usize) {
    let band_l = (TILE_SIZE >> level) + 1;
    let band_h = match level {
        1 => (TILE_SIZE >> 1) - 1,
        _ => (TILE_SIZE + (1 << (level - 1))) >> level,
    };
    let low_high = band_h * band_l;
    let high_high = low_high + band_l * band_h;
    let low_low = high_high + band_h * band_h;
    let step = band_l + band_h;
    let split = band_l * step;

    idwt_x(&buffer[low_low..], band_l, &buffer[..], band_h, &mut temp[..split], step, band_l, band_h, band_l);
    idwt_x(&buffer[low_high..], band_l, &buffer[high_high..], band_h, &mut temp[split..], step, band_l, band_h, band_h);

    let (low, high) = temp.split_at(split);
    idwt_y(low, step, high, step, buffer, step, band_l, band_h, step);
}

fn idwt_x(low: &[i16], low_step: usize, high: &[i16], high_step: usize, destination: &mut [i16], destination_step: usize, low_count: usize, high_count: usize, destination_count: usize) {
    for row in 0..destination_count {
        let low_row = row * low_step;
        let high_row = row * high_step;
        let target = row * destination_step;
        let mut low_index = 0usize;
        let mut high_index = 0usize;

        let mut h0 = i32::from(high[high_row]);
        high_index += 1;
        let mut l0 = i32::from(low[low_row]);
        low_index += 1;
        let mut x0 = clamp_i16(l0 - h0);
        let mut x2 = clamp_i16(l0 - h0);

        for column in 0..high_count - 1 {
            let h1 = i32::from(high[high_row + high_index]);
            high_index += 1;
            l0 = i32::from(low[low_row + low_index]);
            low_index += 1;
            x2 = clamp_i16(l0 - ((h0 + h1) / 2));
            let x1 = clamp_i16((i32::from(x0) + i32::from(x2)) / 2 + 2 * h0);
            destination[target + column * 2] = x0;
            destination[target + column * 2 + 1] = x1;
            x0 = x2;
            h0 = h1;
        }

        let tail = target + (high_count - 1) * 2;
        if low_count <= high_count + 1 {
            if low_count <= high_count {
                destination[tail] = x2;
                destination[tail + 1] = clamp_i16(i32::from(x2) + 2 * h0);
            } else {
                l0 = i32::from(low[low_row + low_index]);
                x0 = clamp_i16(l0 - h0);
                destination[tail] = x2;
                destination[tail + 1] = clamp_i16((i32::from(x0) + i32::from(x2)) / 2 + 2 * h0);
                destination[tail + 2] = x0;
            }
        } else {
            l0 = i32::from(low[low_row + low_index]);
            low_index += 1;
            x0 = clamp_i16(l0 - (h0 / 2));
            destination[tail] = x2;
            destination[tail + 1] = clamp_i16((i32::from(x0) + i32::from(x2)) / 2 + 2 * h0);
            destination[tail + 2] = x0;
            l0 = i32::from(low[low_row + low_index]);
            destination[tail + 3] = clamp_i16((i32::from(x0) + l0) / 2);
        }
    }
}

fn idwt_y(low: &[i16], low_step: usize, high: &[i16], high_step: usize, destination: &mut [i16], destination_step: usize, low_count: usize, high_count: usize, destination_count: usize) {
    for column in 0..destination_count {
        let mut low_index = column;
        let mut high_index = column;
        let mut target = column;

        let mut h0 = i32::from(high[high_index]);
        high_index += high_step;
        let mut l0 = i32::from(low[low_index]);
        low_index += low_step;
        let mut x0 = clamp_i16(l0 - h0);
        let mut x2 = clamp_i16(l0 - h0);

        for _ in 0..high_count - 1 {
            let h1 = i32::from(high[high_index]);
            high_index += high_step;
            l0 = i32::from(low[low_index]);
            low_index += low_step;
            x2 = clamp_i16(l0 - ((h0 + h1) / 2));
            let x1 = clamp_i16((i32::from(x0) + i32::from(x2)) / 2 + 2 * h0);
            destination[target] = x0;
            target += destination_step;
            destination[target] = x1;
            target += destination_step;
            x0 = x2;
            h0 = h1;
        }

        if low_count <= high_count + 1 {
            if low_count <= high_count {
                destination[target] = x2;
                target += destination_step;
                destination[target] = clamp_i16(i32::from(x2) + 2 * h0);
            } else {
                l0 = i32::from(low[low_index]);
                x0 = clamp_i16(l0 - h0);
                destination[target] = x2;
                target += destination_step;
                destination[target] = clamp_i16((i32::from(x0) + i32::from(x2)) / 2 + 2 * h0);
                target += destination_step;
                destination[target] = x0;
            }
        } else {
            l0 = i32::from(low[low_index]);
            low_index += low_step;
            x0 = clamp_i16(l0 - (h0 / 2));
            destination[target] = x2;
            target += destination_step;
            destination[target] = clamp_i16((i32::from(x0) + i32::from(x2)) / 2 + 2 * h0);
            target += destination_step;
            destination[target] = x0;
            target += destination_step;
            l0 = i32::from(low[low_index]);
            destination[target] = clamp_i16((i32::from(x0) + l0) / 2);
        }
    }
}

fn differential_decode(band: &mut [i16]) {
    for index in 1..band.len() {
        band[index] = (i32::from(band[index - 1]) + i32::from(band[index])) as i16;
    }
}

fn shift_band(band: &mut [i16], shift: u32) -> RdpResult<()> {
    if shift == 0 {
        return Ok(());
    }
    if shift >= 16 {
        return Err(RdpProtocolError::protocol(format!("Progressive shift out of range: {}", shift)));
    }
    for value in band.iter_mut() {
        *value = (i32::from(*value) << shift) as i16;
    }
    Ok(())
}

fn clamp_i16(value: i32) -> i16 {
    value.clamp(i32::from(i16::MIN), i32::from(i16::MAX)) as i16
}

fn update_tiles(surface: &mut Surface, region: &RegionBlock<'_>, target: &mut [u8], stride: usize, nv12: &mut Nv12Image, left: u16, top: u16, invalid: &mut Region) -> RdpResult<()> {
    let mut clipping = Region::default();
    for rect in &region.rects {
        clipping.union_rect(Rect::new(left.saturating_add(rect.left), top.saturating_add(rect.top), left.saturating_add(rect.right), top.saturating_add(rect.bottom))?);
    }

    for position in 0..surface.updated.len() {
        let index = surface.updated[position];
        let bounds = Rect::from_size(left.saturating_add(surface.tiles[index].left as u16), top.saturating_add(surface.tiles[index].top as u16), TILE_SIZE as u16, TILE_SIZE as u16)?;
        for rect in clipping.intersect_rect(&bounds).rects() {
            if rect.right as usize > surface.width || rect.bottom as usize > surface.height {
                return Err(RdpProtocolError::protocol(format!("Progressive update region out of bounds: {}x{}", rect.right, rect.bottom)));
            }
            let width = rect.width() as usize * 4;
            let source_left = (rect.left - bounds.left) as usize * 4;
            let source_top = (rect.top - bounds.top) as usize;
            for row in 0..rect.height() as usize {
                let origin = (source_top + row) * TILE_STRIDE + source_left;
                let destination = (rect.top as usize + row) * stride + rect.left as usize * 4;
                target[destination..destination + width].copy_from_slice(&surface.tiles[index].data[origin..origin + width]);
            }
            if bounds.left % 2 == 0 && bounds.top % 2 == 0 {
                nv12.blit_tile(bounds.left as usize, bounds.top as usize, *rect, surface.tiles[index].luma.as_ref(), surface.tiles[index].chroma.as_ref(), TILE_SIZE);
            } else {
                nv12.blit_rgba(*rect, &surface.tiles[index].data, TILE_STRIDE, rect.left - bounds.left, rect.top - bounds.top);
            }
            invalid.union_rect(*rect);
        }
        surface.tiles[index].dirty = false;
    }
    Ok(())
}
