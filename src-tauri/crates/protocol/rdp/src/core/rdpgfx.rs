use crate::gdi::gfx;
use crate::gdi::framebuffer::Rect;
use crate::gdi::framebuffer::Framebuffer;
use crate::core::stream::{ReadCursor, WriteCursor};
use crate::core::error::{RdpProtocolError, RdpResult};
use crate::gdi::gfx::{GfxSurfaces, Point, SurfaceCommand};

pub const CHANNEL_NAME: &str = "Microsoft::Windows::RDS::Graphics";

const CMD_WIRE_TO_SURFACE_1: u16 = 0x0001;
const CMD_WIRE_TO_SURFACE_2: u16 = 0x0002;
const CMD_DELETE_ENCODING_CONTEXT: u16 = 0x0003;
const CMD_SOLID_FILL: u16 = 0x0004;
const CMD_SURFACE_TO_SURFACE: u16 = 0x0005;
const CMD_SURFACE_TO_CACHE: u16 = 0x0006;
const CMD_CACHE_TO_SURFACE: u16 = 0x0007;
const CMD_EVICT_CACHE_ENTRY: u16 = 0x0008;
const CMD_CREATE_SURFACE: u16 = 0x0009;
const CMD_DELETE_SURFACE: u16 = 0x000A;
const CMD_START_FRAME: u16 = 0x000B;
const CMD_END_FRAME: u16 = 0x000C;
const CMD_FRAME_ACKNOWLEDGE: u16 = 0x000D;
const CMD_RESET_GRAPHICS: u16 = 0x000E;
const CMD_MAP_SURFACE_TO_OUTPUT: u16 = 0x000F;
const CMD_CACHE_IMPORT_OFFER: u16 = 0x0010;
const CMD_CACHE_IMPORT_REPLY: u16 = 0x0011;
const CMD_CAPS_ADVERTISE: u16 = 0x0012;
const CMD_CAPS_CONFIRM: u16 = 0x0013;
const CMD_MAP_SURFACE_TO_WINDOW: u16 = 0x0015;
const CMD_QOE_FRAME_ACKNOWLEDGE: u16 = 0x0016;
const CMD_MAP_SURFACE_TO_SCALED_OUTPUT: u16 = 0x0017;
const CMD_MAP_SURFACE_TO_SCALED_WINDOW: u16 = 0x0018;
const CMD_PROTECT_SURFACE: u16 = 0x0019;
const CMD_WATERMARK: u16 = 0x001A;

const CAPVERSION_8: u32 = 0x0008_0004;
const CAPVERSION_81: u32 = 0x0008_0105;
const CAPVERSION_10: u32 = 0x000A_0002;
const CAPVERSION_101: u32 = 0x000A_0100;
const CAPVERSION_102: u32 = 0x000A_0200;
const CAPVERSION_103: u32 = 0x000A_0301;
const CAPVERSION_104: u32 = 0x000A_0400;
const CAPVERSION_105: u32 = 0x000A_0502;
const CAPVERSION_106: u32 = 0x000A_0600;
const CAPVERSION_107: u32 = 0x000A_0701;

const CAPS_FLAG_SMALL_CACHE: u32 = 0x0000_0002;
const CAPS_FLAG_AVC420_ENABLED: u32 = 0x0000_0010;
const CAPS_FLAG_SCALEDMAP_DISABLE: u32 = 0x0000_0080;
const CAPS10_FLAGS: u32 = CAPS_FLAG_SMALL_CACHE;
const CAPS81_FLAGS: u32 = CAPS_FLAG_SMALL_CACHE | CAPS_FLAG_AVC420_ENABLED;

const ADVERTISED_CAPS: [(u32, u32, u32); 10] = [
    (CAPVERSION_107, 4, CAPS10_FLAGS | CAPS_FLAG_SCALEDMAP_DISABLE),
    (CAPVERSION_106, 4, CAPS10_FLAGS),
    (CAPVERSION_105, 4, CAPS10_FLAGS),
    (CAPVERSION_104, 4, CAPS10_FLAGS),
    (CAPVERSION_103, 4, CAPS10_FLAGS),
    (CAPVERSION_102, 4, CAPS10_FLAGS),
    (CAPVERSION_101, 4, CAPS10_FLAGS),
    (CAPVERSION_10, 4, CAPS10_FLAGS),
    (CAPVERSION_81, 4, CAPS81_FLAGS),
    (CAPVERSION_8, 4, CAPS_FLAG_SMALL_CACHE),
];

const MOBILE_ADVERTISED_CAPS: [(u32, u32, u32); 2] = [(CAPVERSION_81, 4, CAPS81_FLAGS), (CAPVERSION_8, 4, CAPS_FLAG_SMALL_CACHE)];

const HEADER_LENGTH: u32 = 8;
const CAPSET_BASE_LENGTH: u32 = 8;
const MONITOR_LENGTH: usize = 20;
const MAX_CACHE_SLOTS: u16 = 4096;
const MAX_CACHE_ENTRIES: u16 = 5462;
const CACHE_IMPORT_ENTRY_LENGTH: usize = 12;
const QUEUE_DEPTH_UNAVAILABLE: u32 = 0;

pub struct GfxDesktop {
    pub width: u16,
    pub height: u16,
}

#[derive(Default)]
pub struct GfxOutcome {
    pub responses: Vec<Vec<u8>>,
    pub desktop: Option<GfxDesktop>,
}

pub fn build_caps_advertise() -> Vec<u8> {
    build_caps_advertise_for(&ADVERTISED_CAPS)
}

pub fn build_mobile_caps_advertise() -> Vec<u8> {
    build_caps_advertise_for(&MOBILE_ADVERTISED_CAPS)
}

fn build_caps_advertise_for(capabilities: &[(u32, u32, u32)]) -> Vec<u8> {
    let length = HEADER_LENGTH + 2 + capabilities.iter().map(|(_, caps_length, _)| CAPSET_BASE_LENGTH + caps_length).sum::<u32>();
    let mut cursor = WriteCursor::with_capacity(length as usize);
    write_header(&mut cursor, CMD_CAPS_ADVERTISE, length);
    cursor.write_u16_le(capabilities.len() as u16);
    for &(version, caps_length, flags) in capabilities {
        cursor.write_u32_le(version);
        cursor.write_u32_le(caps_length);
        cursor.write_u32_le(flags);
        cursor.write_zeros(caps_length as usize - 4);
    }
    cursor.into_vec()
}

pub fn process(surfaces: &mut GfxSurfaces, framebuffer: &mut Framebuffer, message: &[u8]) -> RdpResult<GfxOutcome> {
    let mut outcome = GfxOutcome::default();
    let mut cursor = ReadCursor::new(message);
    while cursor.remaining() >= HEADER_LENGTH as usize {
        let command = cursor.read_u16_le()?;
        cursor.skip(2)?;
        let length = cursor.read_u32_le()?;
        if length < HEADER_LENGTH {
            return Err(RdpProtocolError::protocol(format!("rdpgfx header too short: {}", length)));
        }
        let mut body = cursor.sub_cursor(length as usize - HEADER_LENGTH as usize)?;
        dispatch(surfaces, framebuffer, command, &mut body, &mut outcome)?;
    }
    Ok(outcome)
}

fn dispatch(surfaces: &mut GfxSurfaces, framebuffer: &mut Framebuffer, command: u16, body: &mut ReadCursor<'_>, outcome: &mut GfxOutcome) -> RdpResult<()> {
    match command {
        CMD_WIRE_TO_SURFACE_1 => {
            read_wire_to_surface_1(surfaces, body)?;
            inter_frame_update(surfaces, framebuffer)
        }
        CMD_WIRE_TO_SURFACE_2 => {
            read_wire_to_surface_2(surfaces, body)?;
            inter_frame_update(surfaces, framebuffer)
        }
        CMD_DELETE_ENCODING_CONTEXT => {
            body.skip(6)?;
            Ok(())
        }
        CMD_SOLID_FILL => {
            read_solid_fill(surfaces, body)?;
            inter_frame_update(surfaces, framebuffer)
        }
        CMD_SURFACE_TO_SURFACE => {
            read_surface_to_surface(surfaces, body)?;
            inter_frame_update(surfaces, framebuffer)
        }
        CMD_SURFACE_TO_CACHE => read_surface_to_cache(surfaces, body),
        CMD_CACHE_TO_SURFACE => {
            read_cache_to_surface(surfaces, body)?;
            inter_frame_update(surfaces, framebuffer)
        }
        CMD_EVICT_CACHE_ENTRY => {
            surfaces.evict(read_cache_slot(body)?);
            Ok(())
        }
        CMD_CREATE_SURFACE => read_create_surface(surfaces, body),
        CMD_DELETE_SURFACE => {
            surfaces.delete(body.read_u16_le()?);
            Ok(())
        }
        CMD_START_FRAME => {
            body.skip(4)?;
            surfaces.start_frame(body.read_u32_le()?);
            Ok(())
        }
        CMD_END_FRAME => {
            let frame_id = body.read_u32_le()?;
            surfaces.update(framebuffer)?;
            let decoded = surfaces.end_frame();
            outcome.responses.push(build_frame_acknowledge(frame_id, decoded));
            Ok(())
        }
        CMD_RESET_GRAPHICS => {
            outcome.desktop = Some(read_reset_graphics(surfaces, body)?);
            Ok(())
        }
        CMD_MAP_SURFACE_TO_OUTPUT => {
            let surface_id = body.read_u16_le()?;
            body.skip(2)?;
            let origin_x = body.read_u32_le()?;
            let origin_y = body.read_u32_le()?;
            surfaces.map_to_output(surface_id, origin_x, origin_y)
        }
        CMD_CACHE_IMPORT_OFFER => read_cache_import_offer(body),
        CMD_CACHE_IMPORT_REPLY => read_cache_import_reply(body),
        CMD_CAPS_CONFIRM => {
            body.skip(body.remaining())?;
            Ok(())
        }
        CMD_MAP_SURFACE_TO_WINDOW | CMD_MAP_SURFACE_TO_SCALED_WINDOW => {
            body.skip(10)?;
            Ok(())
        }
        CMD_MAP_SURFACE_TO_SCALED_OUTPUT => {
            let surface_id = body.read_u16_le()?;
            body.skip(2)?;
            let origin_x = body.read_u32_le()?;
            let origin_y = body.read_u32_le()?;
            body.skip(8)?;
            surfaces.map_to_output(surface_id, origin_x, origin_y)
        }
        CMD_QOE_FRAME_ACKNOWLEDGE | CMD_PROTECT_SURFACE | CMD_WATERMARK => Ok(()),
        other => Err(RdpProtocolError::unsupported(format!("rdpgfx command 0x{:04x} not implemented", other))),
    }
}

fn inter_frame_update(surfaces: &mut GfxSurfaces, framebuffer: &mut Framebuffer) -> RdpResult<()> {
    if surfaces.in_frame() {
        return Ok(());
    }
    surfaces.update(framebuffer)
}

fn read_wire_to_surface_1(surfaces: &mut GfxSurfaces, body: &mut ReadCursor<'_>) -> RdpResult<()> {
    let surface_id = body.read_u16_le()?;
    let codec_id = body.read_u16_le()?;
    let alpha = read_pixel_alpha(body.read_u8()?)?;
    let rect = read_rect(body)?;
    let length = body.read_u32_le()? as usize;
    let data = body.read_slice(length)?;
    surfaces.surface_command(&SurfaceCommand { surface_id, codec_id, alpha, rect, data })
}

fn read_wire_to_surface_2(surfaces: &mut GfxSurfaces, body: &mut ReadCursor<'_>) -> RdpResult<()> {
    let surface_id = body.read_u16_le()?;
    let codec_id = body.read_u16_le()?;
    body.skip(4)?;
    let alpha = read_pixel_alpha(body.read_u8()?)?;
    let length = body.read_u32_le()? as usize;
    let data = body.read_slice(length)?;
    surfaces.surface_command(&SurfaceCommand { surface_id, codec_id, alpha, rect: Rect::default(), data })
}

fn read_solid_fill(surfaces: &mut GfxSurfaces, body: &mut ReadCursor<'_>) -> RdpResult<()> {
    let surface_id = body.read_u16_le()?;
    let blue = body.read_u8()?;
    let green = body.read_u8()?;
    let red = body.read_u8()?;
    body.skip(1)?;
    let count = body.read_u16_le()?;
    let mut rects = Vec::with_capacity(count as usize);
    for _ in 0..count {
        rects.push(read_rect(body)?);
    }
    surfaces.solid_fill(surface_id, [red, green, blue, 0xFF], &rects)
}

fn read_surface_to_surface(surfaces: &mut GfxSurfaces, body: &mut ReadCursor<'_>) -> RdpResult<()> {
    let source_id = body.read_u16_le()?;
    let target_id = body.read_u16_le()?;
    let rect = read_rect(body)?;
    let points = read_points(body)?;
    surfaces.surface_to_surface(source_id, target_id, rect, &points)
}

fn read_surface_to_cache(surfaces: &mut GfxSurfaces, body: &mut ReadCursor<'_>) -> RdpResult<()> {
    let surface_id = body.read_u16_le()?;
    body.skip(8)?;
    let slot = read_cache_slot(body)?;
    let rect = read_rect(body)?;
    surfaces.surface_to_cache(surface_id, slot, rect)
}

fn read_cache_to_surface(surfaces: &mut GfxSurfaces, body: &mut ReadCursor<'_>) -> RdpResult<()> {
    let slot = read_cache_slot(body)?;
    let surface_id = body.read_u16_le()?;
    let points = read_points(body)?;
    surfaces.cache_to_surface(slot, surface_id, &points)
}

fn read_create_surface(surfaces: &mut GfxSurfaces, body: &mut ReadCursor<'_>) -> RdpResult<()> {
    let surface_id = body.read_u16_le()?;
    let width = body.read_u16_le()?;
    let height = body.read_u16_le()?;
    let pixel_format = body.read_u8()?;
    if u32::from(width) > gfx::MAX_SURFACE_SIZE || u32::from(height) > gfx::MAX_SURFACE_SIZE {
        return Err(RdpProtocolError::protocol(format!("rdpgfx surface size out of range: {}x{}", width, height)));
    }
    surfaces.create(surface_id, width, height, pixel_format)
}

fn read_reset_graphics(surfaces: &mut GfxSurfaces, body: &mut ReadCursor<'_>) -> RdpResult<GfxDesktop> {
    let width = body.read_u32_le()?;
    let height = body.read_u32_le()?;
    let monitor_count = body.read_u32_le()?;
    if width > gfx::MAX_SURFACE_SIZE || height > gfx::MAX_SURFACE_SIZE || monitor_count > gfx::MAX_MONITOR_COUNT {
        return Err(RdpProtocolError::protocol(format!("rdpgfx reset graphics size out of range: {}x{} monitors {}", width, height, monitor_count)));
    }
    body.skip(monitor_count as usize * MONITOR_LENGTH)?;
    let width = u16::try_from(width).map_err(|_| RdpProtocolError::protocol(format!("rdpgfx desktop width out of range: {}", width)))?;
    let height = u16::try_from(height).map_err(|_| RdpProtocolError::protocol(format!("rdpgfx desktop height out of range: {}", height)))?;
    surfaces.reset();
    Ok(GfxDesktop { width, height })
}

fn read_cache_import_offer(body: &mut ReadCursor<'_>) -> RdpResult<()> {
    let count = body.read_u16_le()?;
    if count > MAX_CACHE_ENTRIES {
        return Err(RdpProtocolError::protocol(format!("rdpgfx cache import count out of range: {}", count)));
    }
    body.skip(count as usize * CACHE_IMPORT_ENTRY_LENGTH)
}

fn read_cache_import_reply(body: &mut ReadCursor<'_>) -> RdpResult<()> {
    let count = body.read_u16_le()?;
    if count > MAX_CACHE_ENTRIES {
        return Err(RdpProtocolError::protocol(format!("rdpgfx cache import count out of range: {}", count)));
    }
    body.skip(count as usize * 2)
}

fn read_points(body: &mut ReadCursor<'_>) -> RdpResult<Vec<Point>> {
    let count = body.read_u16_le()?;
    let mut points = Vec::with_capacity(count as usize);
    for _ in 0..count {
        points.push(Point { x: body.read_u16_le()?, y: body.read_u16_le()? });
    }
    Ok(points)
}

fn read_rect(body: &mut ReadCursor<'_>) -> RdpResult<Rect> {
    let left = body.read_u16_le()?;
    let top = body.read_u16_le()?;
    let right = body.read_u16_le()?;
    let bottom = body.read_u16_le()?;
    if left >= right || top >= bottom {
        return Err(RdpProtocolError::protocol(format!("rdpgfx invalid rectangle: ({},{})-({},{})", left, top, right, bottom)));
    }
    Rect::new(left, top, right, bottom)
}

fn read_cache_slot(body: &mut ReadCursor<'_>) -> RdpResult<u16> {
    let slot = body.read_u16_le()?;
    if slot == 0 || slot > MAX_CACHE_SLOTS {
        return Err(RdpProtocolError::protocol(format!("rdpgfx invalid cache slot: {}", slot)));
    }
    Ok(slot)
}

fn read_pixel_alpha(pixel_format: u8) -> RdpResult<bool> {
    match pixel_format {
        gfx::PIXEL_FORMAT_XRGB => Ok(false),
        gfx::PIXEL_FORMAT_ARGB => Ok(true),
        other => Err(RdpProtocolError::protocol(format!("rdpgfx invalid pixel format: 0x{:02x}", other))),
    }
}

fn build_frame_acknowledge(frame_id: u32, decoded_frames: u32) -> Vec<u8> {
    let length = HEADER_LENGTH + 12;
    let mut cursor = WriteCursor::with_capacity(length as usize);
    write_header(&mut cursor, CMD_FRAME_ACKNOWLEDGE, length);
    cursor.write_u32_le(QUEUE_DEPTH_UNAVAILABLE);
    cursor.write_u32_le(frame_id);
    cursor.write_u32_le(decoded_frames);
    cursor.into_vec()
}

fn write_header(cursor: &mut WriteCursor, command: u16, length: u32) {
    cursor.write_u16_le(command);
    cursor.write_u16_le(0);
    cursor.write_u32_le(length);
}
