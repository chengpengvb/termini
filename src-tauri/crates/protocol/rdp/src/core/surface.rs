use crate::codec::nsc;
use crate::codec::rfx;
use crate::codec::jpeg;
use crate::codec::bitmap;
use crate::codec::color::Palette;
use crate::codec::rfx::RfxContext;
use crate::gdi::framebuffer::Rect;
use crate::core::stream::ReadCursor;
use crate::codec::bitmap::BitmapRect;
use crate::core::error::{RdpProtocolError, RdpResult};

pub const SURFACECMD_FRAMEACTION_END: u16 = 0x0001;

const CMDTYPE_SET_SURFACE_BITS: u16 = 0x0001;
const CMDTYPE_FRAME_MARKER: u16 = 0x0004;
const CMDTYPE_STREAM_SURFACE_BITS: u16 = 0x0006;

const EX_COMPRESSED_BITMAP_HEADER_PRESENT: u8 = 0x01;
const EX_COMPRESSED_BITMAP_HEADER_LENGTH: usize = 24;

const CODEC_ID_NONE: u8 = 0x00;
const CODEC_ID_NSCODEC: u8 = 0x01;
const CODEC_ID_JPEG: u8 = 0x02;
const CODEC_ID_REMOTEFX: u8 = 0x03;

pub enum SurfaceCommand {
    Bits(BitmapRect),
    FrameMarker { action: u16, frame_id: u32 },
}

pub fn parse_commands(cursor: &mut ReadCursor<'_>, palette: &Palette, remotefx: &mut RfxContext, surface: Rect) -> RdpResult<Vec<SurfaceCommand>> {
    let mut commands = Vec::new();
    while cursor.remaining() >= 2 {
        let command_type = cursor.read_u16_le()?;
        match command_type {
            CMDTYPE_SET_SURFACE_BITS | CMDTYPE_STREAM_SURFACE_BITS => parse_surface_bits(cursor, palette, remotefx, surface, &mut commands)?,
            CMDTYPE_FRAME_MARKER => commands.push(parse_frame_marker(cursor)?),
            other => return Err(RdpProtocolError::protocol(format!("unknown surface command type: 0x{:04x}", other))),
        }
    }
    Ok(commands)
}

fn parse_surface_bits(cursor: &mut ReadCursor<'_>, palette: &Palette, remotefx: &mut RfxContext, surface: Rect, commands: &mut Vec<SurfaceCommand>) -> RdpResult<()> {
    let left = cursor.read_u16_le()?;
    let top = cursor.read_u16_le()?;
    let right = cursor.read_u16_le()?;
    let bottom = cursor.read_u16_le()?;
    if right <= left || bottom <= top {
        return Err(RdpProtocolError::protocol(format!("surface bitmap destination rectangle is empty: ({},{})-({},{})", left, top, right, bottom)));
    }

    let bits_per_pixel = cursor.read_u8()? as u16;
    let flags = cursor.read_u8()?;
    cursor.skip(1)?;
    let codec_id = cursor.read_u8()?;
    let width = cursor.read_u16_le()?;
    let height = cursor.read_u16_le()?;
    let length = cursor.read_u32_le()? as usize;
    if width == 0 || height == 0 {
        return Err(RdpProtocolError::protocol(format!("surface bitmap size is zero: {}x{}", width, height)));
    }
    if bits_per_pixel < 1 || bits_per_pixel > 32 {
        return Err(RdpProtocolError::protocol(format!("invalid surface bitmap color depth: {}", bits_per_pixel)));
    }
    if flags & EX_COMPRESSED_BITMAP_HEADER_PRESENT != 0 {
        cursor.skip(EX_COMPRESSED_BITMAP_HEADER_LENGTH)?;
    }
    let data = cursor.read_slice(length)?;

    if codec_id == CODEC_ID_REMOTEFX {
        let target = Rect::new(left, top, right, bottom)?.intersect(&surface);
        commands.extend(rfx::decode(remotefx, data, target.left, target.top, surface)?.into_iter().map(SurfaceCommand::Bits));
        return Ok(());
    }

    if right - left != width || bottom - top != height {
        return Err(RdpProtocolError::protocol(format!("surface bitmap size does not match destination rectangle: {}x{} vs ({},{})-({},{})", width, height, left, top, right, bottom)));
    }
    let rgba = match codec_id {
        CODEC_ID_NONE => bitmap::decode(data, width as usize, height as usize, bits_per_pixel, false, palette)?,
        CODEC_ID_NSCODEC => nsc::decode(data, width as usize, height as usize)?,
        CODEC_ID_JPEG => jpeg::decode(data, width as usize, height as usize)?,
        other => return Err(RdpProtocolError::unsupported(format!("server sent an unnegotiated surface bitmap codec: {}", other))),
    };
    commands.push(SurfaceCommand::Bits(BitmapRect { left, top, dest_right: right.saturating_sub(1), dest_bottom: bottom.saturating_sub(1), width, height, rgba }));
    Ok(())
}

fn parse_frame_marker(cursor: &mut ReadCursor<'_>) -> RdpResult<SurfaceCommand> {
    let action = cursor.read_u16_le()?;
    let frame_id = cursor.read_u32_le()?;
    Ok(SurfaceCommand::FrameMarker { action, frame_id })
}
