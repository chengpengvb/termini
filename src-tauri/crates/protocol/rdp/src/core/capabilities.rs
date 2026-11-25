use crate::gdi::glyph_cache;
use crate::gdi::bitmap_cache;
use crate::core::stream::{ReadCursor, WriteCursor};
use crate::core::error::{RdpProtocolError, RdpResult};

pub const CAPSET_TYPE_GENERAL: u16 = 0x0001;
pub const CAPSET_TYPE_BITMAP: u16 = 0x0002;
pub const CAPSET_TYPE_ORDER: u16 = 0x0003;
pub const CAPSET_TYPE_CONTROL: u16 = 0x0005;
pub const CAPSET_TYPE_ACTIVATION: u16 = 0x0007;
pub const CAPSET_TYPE_POINTER: u16 = 0x0008;
pub const CAPSET_TYPE_SHARE: u16 = 0x0009;
pub const CAPSET_TYPE_COLOR_CACHE: u16 = 0x000A;
pub const CAPSET_TYPE_SOUND: u16 = 0x000C;
pub const CAPSET_TYPE_INPUT: u16 = 0x000D;
pub const CAPSET_TYPE_FONT: u16 = 0x000E;
pub const CAPSET_TYPE_BRUSH: u16 = 0x000F;
pub const CAPSET_TYPE_GLYPH_CACHE: u16 = 0x0010;
pub const CAPSET_TYPE_OFFSCREEN_CACHE: u16 = 0x0011;
pub const CAPSET_TYPE_BITMAP_CACHE_V2: u16 = 0x0013;
pub const CAPSET_TYPE_VIRTUAL_CHANNEL: u16 = 0x0014;
pub const CAPSET_TYPE_MULTI_FRAGMENT_UPDATE: u16 = 0x001A;
pub const CAPSET_TYPE_LARGE_POINTER: u16 = 0x001B;
pub const CAPSET_TYPE_SURFACE_COMMANDS: u16 = 0x001C;
pub const CAPSET_TYPE_BITMAP_CODECS: u16 = 0x001D;
pub const CAPSET_TYPE_FRAME_ACKNOWLEDGE: u16 = 0x001E;

const OS_MAJOR_TYPE_WINDOWS: u16 = 1;
const OS_MINOR_TYPE_WINDOWS_NT: u16 = 3;
const CAPS_PROTOCOL_VERSION: u16 = 0x0200;

const FASTPATH_OUTPUT_SUPPORTED: u16 = 0x0001;
const LONG_CREDENTIALS_SUPPORTED: u16 = 0x0004;
const AUTORECONNECT_SUPPORTED: u16 = 0x0008;
const ENC_SALTED_CHECKSUM: u16 = 0x0010;
const NO_BITMAP_COMPRESSION_HDR: u16 = 0x0400;

const DRAW_ALLOW_DYNAMIC_COLOR_FIDELITY: u8 = 0x02;
const DRAW_ALLOW_SKIP_ALPHA: u8 = 0x08;

const NEGOTIATE_ORDER_SUPPORT: u16 = 0x0002;
const ZERO_BOUNDS_DELTAS_SUPPORT: u16 = 0x0008;
const COLOR_INDEX_SUPPORT: u16 = 0x0020;
const ORDER_FLAGS_EXTRA_SUPPORT: u16 = 0x0080;
const ALTSEC_FRAME_MARKER_SUPPORT: u16 = 0x0004;
const CACHE_BITMAP_V3_SUPPORT: u16 = 0x0002;

const ORDER_SUPPORT_LENGTH: usize = 32;
const NEG_DSTBLT_INDEX: usize = 0x00;
const NEG_PATBLT_INDEX: usize = 0x01;
const NEG_SCRBLT_INDEX: usize = 0x02;
const NEG_MEMBLT_INDEX: usize = 0x03;
const NEG_MEM3BLT_INDEX: usize = 0x04;
const NEG_LINETO_INDEX: usize = 0x08;
const NEG_OPAQUE_RECT_INDEX: usize = 0x0A;
const NEG_MEMBLT_V2_INDEX: usize = 0x0D;
const NEG_MEM3BLT_V2_INDEX: usize = 0x0E;
const NEG_MULTIOPAQUERECT_INDEX: usize = 0x12;
const NEG_FAST_INDEX_INDEX: usize = 0x13;
const NEG_POLYLINE_INDEX: usize = 0x16;
const NEG_FAST_GLYPH_INDEX: usize = 0x18;
const NEG_GLYPH_INDEX_INDEX: usize = 0x1B;
const LOCAL_ORDER_INDEXES: [usize; 14] = [
    NEG_DSTBLT_INDEX,
    NEG_PATBLT_INDEX,
    NEG_SCRBLT_INDEX,
    NEG_MEMBLT_INDEX,
    NEG_MEM3BLT_INDEX,
    NEG_LINETO_INDEX,
    NEG_OPAQUE_RECT_INDEX,
    NEG_MEMBLT_V2_INDEX,
    NEG_MEM3BLT_V2_INDEX,
    NEG_MULTIOPAQUERECT_INDEX,
    NEG_FAST_INDEX_INDEX,
    NEG_POLYLINE_INDEX,
    NEG_FAST_GLYPH_INDEX,
    NEG_GLYPH_INDEX_INDEX,
];

const INPUT_FLAG_SCANCODES: u16 = 0x0001;
pub const INPUT_FLAG_FASTPATH_INPUT: u16 = 0x0008;
pub const INPUT_FLAG_FASTPATH_INPUT2: u16 = 0x0020;
pub const INPUT_FLAG_MOUSEX: u16 = 0x0004;
pub const INPUT_FLAG_UNICODE: u16 = 0x0010;
pub const INPUT_FLAG_MOUSE_RELATIVE: u16 = 0x0080;
pub const INPUT_FLAG_MOUSE_HWHEEL: u16 = 0x0100;

const ALLOW_CACHE_WAITING_LIST_FLAG: u16 = 0x0002;
const PERSISTENT_KEYS_EXPECTED_FLAG: u16 = 0x0001;
const BITMAP_CACHE_PERSISTENT: u32 = 0x8000_0000;
const SOUND_BEEPS_FLAG: u16 = 0x0001;
const POINTER_CACHE_SIZE: u16 = 128;

const CODEC_GUID_JPEG: [u8; 16] = [0xED, 0x9E, 0x0C, 0x43, 0xAF, 0x1B, 0xE6, 0x4C, 0x86, 0x9A, 0xCB, 0x8B, 0x37, 0xB6, 0x62, 0x37];
const CODEC_ID_JPEG: u8 = 0x02;
const JPEG_PROPERTIES_LENGTH: u16 = 1;
const JPEG_QUALITY: u8 = 75;

const CODEC_GUID_NSCODEC: [u8; 16] = [0xB9, 0x1B, 0x8D, 0xCA, 0x0F, 0x00, 0x4F, 0x15, 0x58, 0x9F, 0xAE, 0x2D, 0x1A, 0x87, 0xE2, 0xD6];
const CODEC_ID_NSCODEC: u8 = 0x01;
const NSCODEC_PROPERTIES_LENGTH: u16 = 3;
const NSCODEC_COLOR_LOSS_LEVEL: u8 = 3;

const CODEC_GUID_REMOTEFX: [u8; 16] = [0x12, 0x2F, 0x77, 0x76, 0x72, 0xBD, 0x63, 0x44, 0xAF, 0xB3, 0xB7, 0x3C, 0x9C, 0x6F, 0x78, 0x86];
const CODEC_ID_REMOTEFX: u8 = 0x03;
const REMOTEFX_PROPERTIES_LENGTH: u16 = 49;
const CARDP_CAPS_CAPTURE_NON_CAC: u32 = 0x0000_0001;
const CBY_CAPS: u16 = 0xCBC0;
const CBY_CAPSET: u16 = 0xCBC1;
const CLY_CAPSET: u16 = 0xCFC0;
const CLW_VERSION_1_0: u16 = 0x0100;
const CT_TILE_64X64: u16 = 0x0040;
const CLW_COL_CONV_ICT: u8 = 0x01;
const CLW_XFORM_DWT_53_A: u8 = 0x01;
const CLW_ENTROPY_RLGR1: u8 = 0x01;
const CLW_ENTROPY_RLGR3: u8 = 0x04;
const REMOTEFX_CODEC_MODE: u8 = 0x00;

const BRUSH_COLOR_FULL: u32 = 0x0000_0002;
pub const GLYPH_SUPPORT_ENCODE: u16 = 0x0003;
const FONT_SUPPORT_FONT_LIST: u16 = 0x0001;
const LARGE_POINTER_FLAG_96X96: u16 = 0x0001;

const KEYBOARD_TYPE_IBM_ENHANCED: u32 = 4;
const KEYBOARD_FUNCTION_KEYS: u32 = 12;

const CAPSET_TYPE_BITMAP_CACHE_V3_CODEC_ID: u16 = 0x0006;
const BASE_CAPABILITY_SET_COUNT: u16 = 17;

pub struct ClientCapabilities {
    pub node_id: u16,
    pub color_depth: u16,
    pub desktop_width: u16,
    pub bitmap_codecs: bool,
    pub desktop_height: u16,
    pub keyboard_layout: u32,
    pub input_flags: u16,
    pub order_support: [u8; ORDER_SUPPORT_LENGTH],
    pub large_pointer: Option<u16>,
    pub surface_commands: Option<u32>,
    pub frame_acknowledge: Option<u32>,
    pub virtual_channel_chunk_size: u32,
    pub multifragment_update: Option<u32>,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct ServerCapabilities {
    pub desktop_width: u16,
    pub desktop_height: u16,
    pub color_depth: u16,
    pub desktop_resize: bool,
    pub refresh_rect: bool,
    pub suppress_output: bool,
    pub input_flags: u16,
    pub order_support: [u8; ORDER_SUPPORT_LENGTH],
    pub virtual_channel_chunk_size: Option<u32>,
    pub large_pointer: Option<u16>,
    pub multifragment_update: Option<u32>,
    pub surface_commands: Option<u32>,
    pub bitmap_codecs: bool,
    pub frame_acknowledge: Option<u32>,
    pub salted_checksum: bool,
}

pub fn negotiate_input_flags(server_flags: u16) -> u16 {
    INPUT_FLAG_SCANCODES | (server_flags & (INPUT_FLAG_FASTPATH_INPUT | INPUT_FLAG_FASTPATH_INPUT2 | INPUT_FLAG_MOUSEX | INPUT_FLAG_UNICODE | INPUT_FLAG_MOUSE_RELATIVE | INPUT_FLAG_MOUSE_HWHEEL))
}

pub fn fastpath_input(flags: u16) -> bool {
    flags & (INPUT_FLAG_FASTPATH_INPUT | INPUT_FLAG_FASTPATH_INPUT2) != 0
}

pub fn negotiate_order_support(server_support: &[u8; ORDER_SUPPORT_LENGTH]) -> [u8; ORDER_SUPPORT_LENGTH] {
    let mut order_support = [0u8; ORDER_SUPPORT_LENGTH];
    for index in LOCAL_ORDER_INDEXES {
        order_support[index] = u8::from(server_support[index] != 0);
    }
    order_support
}

pub fn write_client_capability_sets(cursor: &mut WriteCursor, settings: &ClientCapabilities) -> RdpResult<u16> {
    write_general(cursor)?;
    write_bitmap(cursor, settings)?;
    write_order(cursor, settings)?;
    write_bitmap_cache_v2(cursor)?;
    write_pointer(cursor)?;
    write_input(cursor, settings)?;
    write_brush(cursor)?;
    write_glyph_cache(cursor)?;
    write_offscreen_cache(cursor)?;
    write_virtual_channel(cursor, settings.virtual_channel_chunk_size)?;
    write_sound(cursor)?;
    write_share(cursor, settings.node_id)?;
    write_color_cache(cursor)?;
    write_font(cursor)?;
    write_control(cursor)?;
    write_window_activation(cursor)?;
    write_bitmap_cache_v3_codec_id(cursor)?;

    let mut count = BASE_CAPABILITY_SET_COUNT;
    if let Some(max_request_size) = settings.multifragment_update {
        write_multifragment_update(cursor, max_request_size)?;
        count += 1;
    }
    if let Some(flags) = settings.large_pointer.filter(|flags| *flags != 0) {
        write_large_pointer(cursor, flags)?;
        count += 1;
    }
    if let Some(command_flags) = settings.surface_commands {
        write_surface_commands(cursor, command_flags)?;
        count += 1;
    }
    if settings.bitmap_codecs {
        write_bitmap_codecs(cursor)?;
        count += 1;
    }
    if let Some(frame_acknowledge) = settings.frame_acknowledge.filter(|value| *value != 0) {
        write_frame_acknowledge(cursor, frame_acknowledge)?;
        count += 1;
    }
    Ok(count)
}

pub fn parse_server_capability_sets(cursor: &mut ReadCursor<'_>, count: u16) -> RdpResult<ServerCapabilities> {
    let mut capabilities = ServerCapabilities { salted_checksum: true, ..ServerCapabilities::default() };
    for _ in 0..count {
        let kind = cursor.read_u16_le()?;
        let length = cursor.read_u16_le()? as usize;
        if length < 4 {
            return Err(RdpProtocolError::protocol(format!("unexpected capability set length: type={} length={}", kind, length)));
        }
        let mut body = cursor.sub_cursor(length - 4)?;
        match kind {
            CAPSET_TYPE_GENERAL => {
                body.skip(10)?;
                let extra_flags = body.read_u16_le()?;
                body.skip(6)?;
                capabilities.refresh_rect = body.read_u8()? != 0;
                capabilities.suppress_output = body.read_u8()? != 0;
                capabilities.salted_checksum = extra_flags & ENC_SALTED_CHECKSUM != 0;
            }
            CAPSET_TYPE_ORDER => {
                body.skip(32)?;
                capabilities.order_support.copy_from_slice(body.read_slice(ORDER_SUPPORT_LENGTH)?);
            }
            CAPSET_TYPE_INPUT => capabilities.input_flags = body.read_u16_le()?,
            CAPSET_TYPE_BITMAP => {
                capabilities.color_depth = body.read_u16_le()?;
                body.skip(6)?;
                capabilities.desktop_width = body.read_u16_le()?;
                capabilities.desktop_height = body.read_u16_le()?;
                body.skip(2)?;
                capabilities.desktop_resize = body.read_u16_le()? != 0;
            }
            CAPSET_TYPE_VIRTUAL_CHANNEL => {
                body.read_u32_le()?;
                if body.remaining() >= 4 {
                    capabilities.virtual_channel_chunk_size = Some(body.read_u32_le()?);
                }
            }
            CAPSET_TYPE_MULTI_FRAGMENT_UPDATE => capabilities.multifragment_update = Some(body.read_u32_le()?),
            CAPSET_TYPE_LARGE_POINTER => capabilities.large_pointer = Some(body.read_u16_le()? & LARGE_POINTER_FLAG_96X96),
            CAPSET_TYPE_SURFACE_COMMANDS => capabilities.surface_commands = Some(body.read_u32_le()?),
            CAPSET_TYPE_BITMAP_CODECS => capabilities.bitmap_codecs = true,
            CAPSET_TYPE_FRAME_ACKNOWLEDGE => capabilities.frame_acknowledge = Some(body.read_u32_le()?),
            _ => {}
        }
    }
    if capabilities.desktop_width == 0 || capabilities.desktop_height == 0 {
        return Err(RdpProtocolError::protocol("server capability sets lack desktop size".to_string()));
    }
    Ok(capabilities)
}

fn open_capability_set(cursor: &mut WriteCursor, kind: u16) -> usize {
    let offset = cursor.len();
    cursor.write_u16_le(kind);
    cursor.write_u16_le(0);
    offset
}

fn close_capability_set(cursor: &mut WriteCursor, offset: usize) -> RdpResult<()> {
    let length = cursor.len() - offset;
    cursor.patch_u16_le(offset + 2, u16::try_from(length).map_err(|_| RdpProtocolError::encoding("capability set too long".to_string()))?)
}

fn write_general(cursor: &mut WriteCursor) -> RdpResult<()> {
    let offset = open_capability_set(cursor, CAPSET_TYPE_GENERAL);
    cursor.write_u16_le(OS_MAJOR_TYPE_WINDOWS);
    cursor.write_u16_le(OS_MINOR_TYPE_WINDOWS_NT);
    cursor.write_u16_le(CAPS_PROTOCOL_VERSION);
    cursor.write_u16_le(0);
    cursor.write_u16_le(0);
    cursor.write_u16_le(FASTPATH_OUTPUT_SUPPORTED | NO_BITMAP_COMPRESSION_HDR | LONG_CREDENTIALS_SUPPORTED | AUTORECONNECT_SUPPORTED | ENC_SALTED_CHECKSUM);
    cursor.write_u16_le(0);
    cursor.write_u16_le(0);
    cursor.write_u16_le(0);
    cursor.write_u8(1);
    cursor.write_u8(1);
    close_capability_set(cursor, offset)
}

fn write_bitmap(cursor: &mut WriteCursor, settings: &ClientCapabilities) -> RdpResult<()> {
    let offset = open_capability_set(cursor, CAPSET_TYPE_BITMAP);
    cursor.write_u16_le(settings.color_depth);
    cursor.write_u16_le(1);
    cursor.write_u16_le(1);
    cursor.write_u16_le(1);
    cursor.write_u16_le(settings.desktop_width);
    cursor.write_u16_le(settings.desktop_height);
    cursor.write_u16_le(0);
    cursor.write_u16_le(1);
    cursor.write_u16_le(1);
    cursor.write_u8(0);
    cursor.write_u8(DRAW_ALLOW_DYNAMIC_COLOR_FIDELITY | DRAW_ALLOW_SKIP_ALPHA);
    cursor.write_u16_le(1);
    cursor.write_u16_le(0);
    close_capability_set(cursor, offset)
}

fn write_order(cursor: &mut WriteCursor, settings: &ClientCapabilities) -> RdpResult<()> {
    let offset = open_capability_set(cursor, CAPSET_TYPE_ORDER);
    cursor.write_zeros(16);
    cursor.write_u32_le(0);
    cursor.write_u16_le(1);
    cursor.write_u16_le(20);
    cursor.write_u16_le(0);
    cursor.write_u16_le(1);
    cursor.write_u16_le(0);
    cursor.write_u16_le(NEGOTIATE_ORDER_SUPPORT | ZERO_BOUNDS_DELTAS_SUPPORT | COLOR_INDEX_SUPPORT | ORDER_FLAGS_EXTRA_SUPPORT);
    cursor.write_slice(&settings.order_support);
    cursor.write_u16_le(0);
    cursor.write_u16_le(CACHE_BITMAP_V3_SUPPORT | ALTSEC_FRAME_MARKER_SUPPORT);
    cursor.write_u32_le(0);
    cursor.write_u32_le(230_400);
    cursor.write_u16_le(0);
    cursor.write_u16_le(0);
    cursor.write_u16_le(0);
    cursor.write_u16_le(0);
    close_capability_set(cursor, offset)
}

fn write_bitmap_cache_v2(cursor: &mut WriteCursor) -> RdpResult<()> {
    let offset = open_capability_set(cursor, CAPSET_TYPE_BITMAP_CACHE_V2);
    cursor.write_u16_le(ALLOW_CACHE_WAITING_LIST_FLAG | PERSISTENT_KEYS_EXPECTED_FLAG);
    cursor.write_u8(0);
    cursor.write_u8(bitmap_cache::CELL_ENTRIES.len() as u8);
    for entries in bitmap_cache::CELL_ENTRIES {
        cursor.write_u32_le(entries as u32 | BITMAP_CACHE_PERSISTENT);
    }
    cursor.write_zeros(12);
    close_capability_set(cursor, offset)
}

fn write_pointer(cursor: &mut WriteCursor) -> RdpResult<()> {
    let offset = open_capability_set(cursor, CAPSET_TYPE_POINTER);
    cursor.write_u16_le(1);
    cursor.write_u16_le(POINTER_CACHE_SIZE);
    cursor.write_u16_le(POINTER_CACHE_SIZE);
    close_capability_set(cursor, offset)
}

fn write_input(cursor: &mut WriteCursor, settings: &ClientCapabilities) -> RdpResult<()> {
    let offset = open_capability_set(cursor, CAPSET_TYPE_INPUT);
    cursor.write_u16_le(settings.input_flags);
    cursor.write_u16_le(0);
    cursor.write_u32_le(settings.keyboard_layout);
    cursor.write_u32_le(KEYBOARD_TYPE_IBM_ENHANCED);
    cursor.write_u32_le(0);
    cursor.write_u32_le(KEYBOARD_FUNCTION_KEYS);
    cursor.write_zeros(64);
    close_capability_set(cursor, offset)
}

fn write_brush(cursor: &mut WriteCursor) -> RdpResult<()> {
    let offset = open_capability_set(cursor, CAPSET_TYPE_BRUSH);
    cursor.write_u32_le(BRUSH_COLOR_FULL);
    close_capability_set(cursor, offset)
}

fn write_glyph_cache(cursor: &mut WriteCursor) -> RdpResult<()> {
    let offset = open_capability_set(cursor, CAPSET_TYPE_GLYPH_CACHE);
    for (entries, cell_size) in glyph_cache::GLYPH_CACHE_CELLS {
        cursor.write_u16_le(entries);
        cursor.write_u16_le(cell_size);
    }
    cursor.write_u16_le(glyph_cache::FRAG_CACHE_ENTRIES);
    cursor.write_u16_le(glyph_cache::FRAG_CACHE_CELL_SIZE);
    cursor.write_u16_le(GLYPH_SUPPORT_ENCODE);
    cursor.write_u16_le(0);
    close_capability_set(cursor, offset)
}

fn write_offscreen_cache(cursor: &mut WriteCursor) -> RdpResult<()> {
    let offset = open_capability_set(cursor, CAPSET_TYPE_OFFSCREEN_CACHE);
    cursor.write_u32_le(0);
    cursor.write_u16_le(0);
    cursor.write_u16_le(0);
    close_capability_set(cursor, offset)
}

fn write_virtual_channel(cursor: &mut WriteCursor, chunk_size: u32) -> RdpResult<()> {
    let offset = open_capability_set(cursor, CAPSET_TYPE_VIRTUAL_CHANNEL);
    cursor.write_u32_le(0);
    cursor.write_u32_le(chunk_size);
    close_capability_set(cursor, offset)
}

fn write_sound(cursor: &mut WriteCursor) -> RdpResult<()> {
    let offset = open_capability_set(cursor, CAPSET_TYPE_SOUND);
    cursor.write_u16_le(SOUND_BEEPS_FLAG);
    cursor.write_u16_le(0);
    close_capability_set(cursor, offset)
}

fn write_share(cursor: &mut WriteCursor, node_id: u16) -> RdpResult<()> {
    let offset = open_capability_set(cursor, CAPSET_TYPE_SHARE);
    cursor.write_u16_le(node_id);
    cursor.write_u16_le(0);
    close_capability_set(cursor, offset)
}

fn write_color_cache(cursor: &mut WriteCursor) -> RdpResult<()> {
    let offset = open_capability_set(cursor, CAPSET_TYPE_COLOR_CACHE);
    cursor.write_u16_le(6);
    cursor.write_u16_le(0);
    close_capability_set(cursor, offset)
}

fn write_font(cursor: &mut WriteCursor) -> RdpResult<()> {
    let offset = open_capability_set(cursor, CAPSET_TYPE_FONT);
    cursor.write_u16_le(FONT_SUPPORT_FONT_LIST);
    cursor.write_u16_le(0);
    close_capability_set(cursor, offset)
}

fn write_control(cursor: &mut WriteCursor) -> RdpResult<()> {
    let offset = open_capability_set(cursor, CAPSET_TYPE_CONTROL);
    cursor.write_u16_le(0);
    cursor.write_u16_le(0);
    cursor.write_u16_le(2);
    cursor.write_u16_le(2);
    close_capability_set(cursor, offset)
}

fn write_window_activation(cursor: &mut WriteCursor) -> RdpResult<()> {
    let offset = open_capability_set(cursor, CAPSET_TYPE_ACTIVATION);
    cursor.write_u16_le(0);
    cursor.write_u16_le(0);
    cursor.write_u16_le(0);
    cursor.write_u16_le(0);
    close_capability_set(cursor, offset)
}

fn write_multifragment_update(cursor: &mut WriteCursor, max_request_size: u32) -> RdpResult<()> {
    let offset = open_capability_set(cursor, CAPSET_TYPE_MULTI_FRAGMENT_UPDATE);
    cursor.write_u32_le(max_request_size);
    close_capability_set(cursor, offset)
}

fn write_large_pointer(cursor: &mut WriteCursor, flags: u16) -> RdpResult<()> {
    let offset = open_capability_set(cursor, CAPSET_TYPE_LARGE_POINTER);
    cursor.write_u16_le(flags);
    close_capability_set(cursor, offset)
}

fn write_surface_commands(cursor: &mut WriteCursor, command_flags: u32) -> RdpResult<()> {
    let offset = open_capability_set(cursor, CAPSET_TYPE_SURFACE_COMMANDS);
    cursor.write_u32_le(command_flags);
    cursor.write_u32_le(0);
    close_capability_set(cursor, offset)
}

fn write_bitmap_codecs(cursor: &mut WriteCursor) -> RdpResult<()> {
    let offset = open_capability_set(cursor, CAPSET_TYPE_BITMAP_CODECS);
    cursor.write_u8(3);

    cursor.write_slice(&CODEC_GUID_REMOTEFX);
    cursor.write_u8(CODEC_ID_REMOTEFX);
    cursor.write_u16_le(REMOTEFX_PROPERTIES_LENGTH);
    cursor.write_u32_le(u32::from(REMOTEFX_PROPERTIES_LENGTH));
    cursor.write_u32_le(CARDP_CAPS_CAPTURE_NON_CAC);
    cursor.write_u32_le(37);
    cursor.write_u16_le(CBY_CAPS);
    cursor.write_u32_le(8);
    cursor.write_u16_le(1);
    cursor.write_u16_le(CBY_CAPSET);
    cursor.write_u32_le(29);
    cursor.write_u8(0x01);
    cursor.write_u16_le(CLY_CAPSET);
    cursor.write_u16_le(2);
    cursor.write_u16_le(8);
    write_remotefx_entropy(cursor, CLW_ENTROPY_RLGR1);
    write_remotefx_entropy(cursor, CLW_ENTROPY_RLGR3);

    cursor.write_slice(&CODEC_GUID_NSCODEC);
    cursor.write_u8(CODEC_ID_NSCODEC);
    cursor.write_u16_le(NSCODEC_PROPERTIES_LENGTH);
    cursor.write_u8(1);
    cursor.write_u8(1);
    cursor.write_u8(NSCODEC_COLOR_LOSS_LEVEL);

    cursor.write_slice(&CODEC_GUID_JPEG);
    cursor.write_u8(CODEC_ID_JPEG);
    cursor.write_u16_le(JPEG_PROPERTIES_LENGTH);
    cursor.write_u8(JPEG_QUALITY);
    close_capability_set(cursor, offset)
}

fn write_remotefx_entropy(cursor: &mut WriteCursor, entropy: u8) {
    cursor.write_u16_le(CLW_VERSION_1_0);
    cursor.write_u16_le(CT_TILE_64X64);
    cursor.write_u8(REMOTEFX_CODEC_MODE);
    cursor.write_u8(CLW_COL_CONV_ICT);
    cursor.write_u8(CLW_XFORM_DWT_53_A);
    cursor.write_u8(entropy);
}

fn write_frame_acknowledge(cursor: &mut WriteCursor, frame_acknowledge: u32) -> RdpResult<()> {
    let offset = open_capability_set(cursor, CAPSET_TYPE_FRAME_ACKNOWLEDGE);
    cursor.write_u32_le(frame_acknowledge);
    close_capability_set(cursor, offset)
}

fn write_bitmap_cache_v3_codec_id(cursor: &mut WriteCursor) -> RdpResult<()> {
    let offset = open_capability_set(cursor, CAPSET_TYPE_BITMAP_CACHE_V3_CODEC_ID);
    cursor.write_u8(CODEC_ID_JPEG);
    close_capability_set(cursor, offset)
}
